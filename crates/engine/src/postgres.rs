//! PostgreSQL pools and transactions remain entirely inside the engine.
use crate::operations::{encode, id};
use serde_json::{Value, json};
use sqlx::{
    Arguments, Column, Executor, PgPool, Postgres, Row, TypeInfo, ValueRef,
    postgres::{PgArguments, PgConnectOptions, PgPoolOptions},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
struct Pool {
    options: PgPoolOptions,
    connect: PgConnectOptions,
    inner: tokio::sync::OnceCell<PgPool>,
    runtime: Mutex<Option<tokio::runtime::Handle>>,
}
static POOLS: OnceLock<Mutex<HashMap<u64, Arc<Pool>>>> = OnceLock::new();
struct Transaction {
    runtime: tokio::runtime::Handle,
    inner: tokio::sync::Mutex<Option<sqlx::Transaction<'static, Postgres>>>,
}
impl Drop for Transaction {
    fn drop(&mut self) {
        let _enter = self.runtime.enter();
        drop(self.inner.get_mut().take());
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.get_mut().unwrap().take() {
            let _enter = runtime.enter();
            drop(self.inner.take());
        }
    }
}
type Tx = Arc<Transaction>;
static TRANSACTIONS: OnceLock<Mutex<HashMap<u64, Tx>>> = OnceLock::new();
fn pools() -> &'static Mutex<HashMap<u64, Arc<Pool>>> {
    POOLS.get_or_init(Default::default)
}
fn transactions() -> &'static Mutex<HashMap<u64, Tx>> {
    TRANSACTIONS.get_or_init(Default::default)
}
pub fn release(kind: u32, id: u64) {
    match kind {
        18 => {
            let pool = pools().lock().unwrap().remove(&id);
            drop(pool);
        }
        19 => {
            let tx = transactions().lock().unwrap().remove(&id);
            drop(tx);
        }
        _ => {}
    }
}
fn invalid(s: impl Into<String>) -> sqlx::Error {
    sqlx::Error::Protocol(s.into())
}
pub fn connect_lazy(c: Value) -> Result<Vec<u8>, String> {
    let options = PgPoolOptions::new()
        .max_connections(
            c["maximum"]
                .as_u64()
                .unwrap_or(5)
                .try_into()
                .map_err(|_| "invalid maximum")?,
        )
        .acquire_timeout(Duration::from_millis(
            c["timeout_ms"].as_u64().unwrap_or(3000),
        ));
    let connect = c["url"]
        .as_str()
        .ok_or("missing database URL")?
        .parse::<PgConnectOptions>()
        .map_err(|e| e.to_string())?;
    let pool = Arc::new(Pool {
        options,
        connect,
        inner: tokio::sync::OnceCell::new(),
        runtime: Mutex::new(None),
    });
    let id = id();
    pools().lock().unwrap().insert(id, pool);
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
pub async fn run(c: Value, payload: Vec<u8>) -> Vec<u8> {
    match execute(c, payload).await {
        Ok(v) => v,
        Err(e) => encode(
            json!({"ok":false,"message":e.to_string(),"code":e.as_database_error().and_then(|e|e.code()).as_deref(),"row_not_found":matches!(e,sqlx::Error::RowNotFound)}),
            &[],
        ),
    }
}
async fn execute(c: Value, payload: Vec<u8>) -> Result<Vec<u8>, sqlx::Error> {
    let pool = c["pool"]
        .as_u64()
        .and_then(|id| pools().lock().unwrap().get(&id).cloned());
    let pool = match pool {
        Some(pool) => Some(
            pool.inner
                .get_or_init(|| async {
                    *pool.runtime.lock().unwrap() = Some(tokio::runtime::Handle::current());
                    pool.options.clone().connect_lazy_with(pool.connect.clone())
                })
                .await
                .clone(),
        ),
        None => None,
    };
    let tx = c["transaction"]
        .as_u64()
        .and_then(|id| transactions().lock().unwrap().get(&id).cloned());
    match c["action"].as_str() {
        Some("begin") => {
            let tx = pool
                .ok_or_else(|| invalid("pool released"))?
                .begin()
                .await?;
            let id = id();
            transactions().lock().unwrap().insert(
                id,
                Arc::new(Transaction {
                    runtime: tokio::runtime::Handle::current(),
                    inner: tokio::sync::Mutex::new(Some(tx)),
                }),
            );
            Ok(encode(json!({"ok":true,"id":id}), &[]))
        }
        Some("commit" | "rollback") => {
            let tx = tx.ok_or_else(|| invalid("transaction released"))?;
            let tx = tx
                .inner
                .lock()
                .await
                .take()
                .ok_or_else(|| invalid("transaction finished"))?;
            if c["action"] == "commit" {
                tx.commit().await?;
            } else {
                tx.rollback().await?;
            }
            Ok(encode(json!({"ok":true}), &[]))
        }
        Some("close") => {
            pool.ok_or_else(|| invalid("pool released"))?.close().await;
            Ok(encode(json!({"ok":true}), &[]))
        }
        Some("migrate") => {
            let mut migrations = Vec::new();
            for m in c["migrations"]
                .as_array()
                .ok_or_else(|| invalid("missing migrations"))?
            {
                migrations.push(sqlx::migrate::Migration::new(
                    m["version"].as_i64().ok_or_else(|| invalid("version"))?,
                    m["description"]
                        .as_str()
                        .ok_or_else(|| invalid("description"))?
                        .to_owned()
                        .into(),
                    sqlx::migrate::MigrationType::Simple,
                    m["sql"]
                        .as_str()
                        .ok_or_else(|| invalid("sql"))?
                        .to_owned()
                        .into(),
                    m["no_tx"].as_bool().unwrap_or(false),
                ));
            }
            let migrator = sqlx::migrate::Migrator {
                migrations: migrations.into(),
                ignore_missing: c["ignore_missing"].as_bool().unwrap_or(false),
                locking: c["locking"].as_bool().unwrap_or(true),
                no_tx: c["no_tx"].as_bool().unwrap_or(false),
            };
            migrator
                .run(&pool.ok_or_else(|| invalid("pool released"))?)
                .await
                .map_err(|e| invalid(e.to_string()))?;
            Ok(encode(json!({"ok":true}), &[]))
        }
        Some("execute" | "fetch" | "fetch_optional" | "raw") => {
            let sql = c["sql"].as_str().ok_or_else(|| invalid("missing SQL"))?;
            let bindings: Vec<Value> =
                serde_json::from_slice(&payload).map_err(|e| invalid(e.to_string()))?;
            let mut args = PgArguments::default();
            for b in bindings {
                let v = &b["value"];
                macro_rules! add {
                    ($ty:ty) => {{
                        let v: Option<$ty> = serde_json::from_value(v.clone())
                            .map_err(|e| invalid(e.to_string()))?;
                        args.add(v)
                    }};
                }
                let r = match b["type"].as_str() {
                    Some("text") => add!(String),
                    Some("i16") => add!(i16),
                    Some("i32") => add!(i32),
                    Some("i64") => add!(i64),
                    Some("bool") => add!(bool),
                    Some("f64") => add!(f64),
                    Some("bytes") => add!(Vec<u8>),
                    Some("texts") => add!(Vec<String>),
                    Some("json") => {
                        if b["null"] == true {
                            args.add(Option::<sqlx::types::Json<Value>>::None)
                        } else {
                            args.add(sqlx::types::Json(v.clone()))
                        }
                    }
                    _ => return Err(invalid("unsupported parameter type")),
                };
                r.map_err(|e| invalid(e.to_string()))?;
            }
            let mut guard = if let Some(tx) = &tx {
                Some(tx.inner.lock().await)
            } else {
                None
            };
            let mut pooled = if guard.is_none() {
                Some(
                    pool.ok_or_else(|| invalid("pool released"))?
                        .acquire()
                        .await?,
                )
            } else {
                None
            };
            let conn = if let Some(g) = &mut guard {
                &mut **g.as_mut().ok_or_else(|| invalid("transaction finished"))?
            } else {
                &mut **pooled.as_mut().unwrap()
            };
            if c["action"] == "raw" {
                let r = conn.execute(sql).await?;
                return Ok(encode(
                    json!({"ok":true,"rows_affected":r.rows_affected()}),
                    &[],
                ));
            }
            let query = sqlx::query_with(sql, args);
            if c["action"] == "execute" {
                let r = conn.execute(query).await?;
                return Ok(encode(
                    json!({"ok":true,"rows_affected":r.rows_affected()}),
                    &[],
                ));
            }
            let rows = if c["action"] == "fetch_optional" {
                conn.fetch_optional(query).await?.into_iter().collect()
            } else {
                conn.fetch_all(query).await?
            };
            let mut output = Vec::new();
            for row in rows {
                let mut cells = Vec::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let raw = row.try_get_raw(i)?;
                    let ty = raw.type_info();
                    let value = if raw.is_null() {
                        Value::Null
                    } else {
                        match ty.name() {
                            "BOOL" => json!(row.try_get::<bool, _>(i)?),
                            "INT2" => json!(row.try_get::<i16, _>(i)?),
                            "INT4" => json!(row.try_get::<i32, _>(i)?),
                            "INT8" => json!(row.try_get::<i64, _>(i)?),
                            "FLOAT4" => json!(row.try_get::<f32, _>(i)?),
                            "FLOAT8" => json!(row.try_get::<f64, _>(i)?),
                            "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" => {
                                json!(row.try_get::<String, _>(i)?)
                            }
                            "JSON" | "JSONB" => row.try_get::<Value, _>(i)?,
                            "BYTEA" => json!(row.try_get::<Vec<u8>, _>(i)?),
                            "TEXT[]" | "VARCHAR[]" => json!(row.try_get::<Vec<String>, _>(i)?),
                            _ => json!({"unsupported_postgres_type":ty.name()}),
                        }
                    };
                    let supported = raw.is_null()
                        || matches!(
                            ty.name(),
                            "BOOL"
                                | "INT2"
                                | "INT4"
                                | "INT8"
                                | "FLOAT4"
                                | "FLOAT8"
                                | "TEXT"
                                | "VARCHAR"
                                | "BPCHAR"
                                | "NAME"
                                | "JSON"
                                | "JSONB"
                                | "BYTEA"
                                | "TEXT[]"
                                | "VARCHAR[]"
                        );
                    let supported = supported
                        && !(matches!(ty.name(), "FLOAT4" | "FLOAT8")
                            && !raw.is_null()
                            && value.is_null());
                    cells.push(json!([col.name(), value, supported]));
                }
                output.push(cells);
            }
            Ok(encode(json!({"ok":true,"rows":output}), &[]))
        }
        _ => Err(invalid("unknown PostgreSQL action")),
    }
}
