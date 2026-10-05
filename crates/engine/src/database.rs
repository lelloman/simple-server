use crate::operations::encode;
use serde_json::{Value, json};
use simple_server_abi::values::{self, Value as Cell};
use sqlx::{
    Arguments, Column, Executor, Row, Sqlite, SqlitePool, TypeInfo, ValueRef,
    sqlite::{SqliteArguments, SqliteConnectOptions, SqlitePoolOptions},
};
use std::{
    collections::HashMap,
    os::unix::ffi::OsStringExt,
    sync::{Arc, Mutex, OnceLock},
};

static POOLS: OnceLock<Mutex<HashMap<u64, SqlitePool>>> = OnceLock::new();
type Tx = Arc<tokio::sync::Mutex<Option<sqlx::Transaction<'static, Sqlite>>>>;
static TRANSACTIONS: OnceLock<Mutex<HashMap<u64, Tx>>> = OnceLock::new();
fn pools() -> &'static Mutex<HashMap<u64, SqlitePool>> {
    POOLS.get_or_init(Default::default)
}
fn transactions() -> &'static Mutex<HashMap<u64, Tx>> {
    TRANSACTIONS.get_or_init(Default::default)
}

pub fn release(kind: u32, id: u64) {
    match kind {
        3 => {
            pools().lock().unwrap().remove(&id);
        }
        4 => {
            transactions().lock().unwrap().remove(&id);
        }
        _ => {}
    }
}
fn error(error: sqlx::Error) -> Vec<u8> {
    encode(
        json!({"ok":false,"message":error.to_string(),"code":error.as_database_error().and_then(|e| e.code()).as_deref(),"row_not_found":matches!(error,sqlx::Error::RowNotFound)}),
        &[],
    )
}
pub async fn run(command: Value, payload: Vec<u8>) -> Vec<u8> {
    match execute(command, payload).await {
        Ok(bytes) => bytes,
        Err(e) => error(e),
    }
}
fn invalid(message: impl Into<String>) -> sqlx::Error {
    sqlx::Error::Protocol(message.into())
}

async fn execute(command: Value, payload: Vec<u8>) -> Result<Vec<u8>, sqlx::Error> {
    if command["action"] == "connect" {
        let filename: Vec<u8> = serde_json::from_value(command["filename"].clone())
            .map_err(|e| invalid(e.to_string()))?;
        let commands: Vec<String> = serde_json::from_value(command["init_sql"].clone())
            .map_err(|e| invalid(e.to_string()))?;
        let options = SqliteConnectOptions::new()
            .filename(std::ffi::OsString::from_vec(filename))
            .create_if_missing(command["create"].as_bool().unwrap_or(false));
        let pool = SqlitePoolOptions::new()
            .max_connections(
                command["max_connections"]
                    .as_u64()
                    .unwrap_or(1)
                    .try_into()
                    .map_err(|_| invalid("invalid pool size"))?,
            )
            .after_connect(move |connection, _| {
                let commands = commands.clone();
                Box::pin(async move {
                    for command in commands {
                        connection.execute(command.as_str()).await?;
                    }
                    Ok(())
                })
            })
            .connect_with(options)
            .await?;
        let id = crate::operations::id();
        pools().lock().unwrap().insert(id, pool);
        return Ok(encode(json!({"ok":true,"id":id}), &[]));
    }
    let pool = command["pool"]
        .as_u64()
        .and_then(|id| pools().lock().unwrap().get(&id).cloned());
    let tx = command["transaction"]
        .as_u64()
        .and_then(|id| transactions().lock().unwrap().get(&id).cloned());
    match command["action"].as_str() {
        Some("begin") => {
            let tx = pool
                .ok_or_else(|| invalid("pool released"))?
                .begin()
                .await?;
            let id = crate::operations::id();
            transactions()
                .lock()
                .unwrap()
                .insert(id, Arc::new(tokio::sync::Mutex::new(Some(tx))));
            Ok(encode(json!({"ok":true,"id":id}), &[]))
        }
        Some("commit" | "rollback") => {
            let tx = tx.ok_or_else(|| invalid("transaction released"))?;
            let transaction = tx
                .lock()
                .await
                .take()
                .ok_or_else(|| invalid("transaction finished"))?;
            if command["action"] == "commit" {
                transaction.commit().await?;
            } else {
                transaction.rollback().await?;
            }
            Ok(encode(json!({"ok":true}), &[]))
        }
        Some("close") => {
            pool.ok_or_else(|| invalid("pool released"))?.close().await;
            Ok(encode(json!({"ok":true}), &[]))
        }
        Some("execute" | "fetch" | "fetch_optional" | "raw") => {
            let sql = command["sql"]
                .as_str()
                .ok_or_else(|| invalid("missing SQL"))?;
            let mut args = SqliteArguments::default();
            for cell in values::decode(&payload).map_err(invalid)? {
                let result = match cell {
                    Cell::Null => args.add(Option::<i64>::None),
                    Cell::Integer(n) => args.add(n),
                    Cell::Real(n) => args.add(n),
                    Cell::Text(s) => args.add(s),
                    Cell::Blob(b) => args.add(b),
                };
                result.map_err(|e| invalid(e.to_string()))?;
            }
            let mut tx_guard = if let Some(tx) = &tx {
                Some(tx.lock().await)
            } else {
                None
            };
            let mut pooled = if tx_guard.is_none() {
                Some(
                    pool.ok_or_else(|| invalid("pool released"))?
                        .acquire()
                        .await?,
                )
            } else {
                None
            };
            let connection = if let Some(guard) = &mut tx_guard {
                &mut **guard
                    .as_mut()
                    .ok_or_else(|| invalid("transaction finished"))?
            } else {
                &mut **pooled.as_mut().unwrap()
            };
            if command["action"] == "raw" {
                let result = connection.execute(sql).await?;
                return Ok(encode(
                    json!({"ok":true,"rows_affected":result.rows_affected(),"last_insert_rowid":result.last_insert_rowid()}),
                    &[],
                ));
            }
            let query = sqlx::query_with(sql, args);
            if command["action"] == "execute" {
                let result = connection.execute(query).await?;
                return Ok(encode(
                    json!({"ok":true,"rows_affected":result.rows_affected(),"last_insert_rowid":result.last_insert_rowid()}),
                    &[],
                ));
            }
            let rows = if command["action"] == "fetch_optional" {
                connection
                    .fetch_optional(query)
                    .await?
                    .into_iter()
                    .collect()
            } else {
                connection.fetch_all(query).await?
            };
            let mut cells = Vec::new();
            let mut names = Vec::new();
            for row in &rows {
                let mut columns = Vec::new();
                for (index, column) in row.columns().iter().enumerate() {
                    columns.push(column.name().to_owned());
                    let raw = row.try_get_raw(index)?;
                    cells.push(if raw.is_null() {
                        Cell::Null
                    } else {
                        match raw.type_info().name() {
                            "INTEGER" | "BOOLEAN" => Cell::Integer(row.try_get_unchecked(index)?),
                            "REAL" => Cell::Real(row.try_get_unchecked(index)?),
                            "TEXT" => Cell::Text(row.try_get_unchecked(index)?),
                            "BLOB" => Cell::Blob(row.try_get_unchecked(index)?),
                            ty => {
                                return Err(invalid(format!(
                                    "unsupported SQLite storage type {ty}"
                                )));
                            }
                        }
                    });
                }
                names.push(columns);
            }
            Ok(encode(
                json!({"ok":true,"columns":names}),
                &values::encode(&cells).map_err(invalid)?,
            ))
        }
        _ => Err(invalid("unknown SQLite action")),
    }
}
