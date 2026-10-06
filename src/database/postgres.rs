//! Typed application queries backed by the separately compiled PostgreSQL engine.
use serde_json::Value;
use serde_json::json;
use simple_server_sys::Resource;
use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub enum Error {
    RowNotFound,
    Configuration(Box<dyn std::error::Error + Send + Sync>),
    Database(DatabaseError),
    Decode(String),
}
#[derive(Debug)]
pub struct DatabaseError {
    message: String,
    code: Option<String>,
}
impl DatabaseError {
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }
}
impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for DatabaseError {}
impl Error {
    pub fn as_database_error(&self) -> Option<&DatabaseError> {
        if let Self::Database(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RowNotFound => f.write_str("no rows returned by query"),
            Self::Configuration(e) => e.fmt(f),
            Self::Database(e) => e.fmt(f),
            Self::Decode(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Configuration(e) => Some(e.as_ref()),
            Self::Database(e) => Some(e),
            _ => None,
        }
    }
}
fn decode(error: impl fmt::Display) -> Error {
    Error::Decode(error.to_string())
}
async fn command(
    header: serde_json::Value,
    body: &[u8],
) -> Result<(serde_json::Value, Vec<u8>), Error> {
    let (header, body) = crate::engine_wire::command(header, body)
        .await
        .map_err(|e| Error::Configuration(Box::new(e)))?;
    if header["ok"] == true {
        Ok((header, body))
    } else if header["row_not_found"] == true {
        Err(Error::RowNotFound)
    } else {
        Err(Error::Database(DatabaseError {
            message: header["message"]
                .as_str()
                .unwrap_or("engine PostgreSQL error")
                .to_owned(),
            code: header["code"].as_str().map(str::to_owned),
        }))
    }
}

#[derive(Debug)]
pub struct Postgres;
#[derive(Clone)]
pub struct PgPool {
    resource: Resource,
}
impl fmt::Debug for PgPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgPool").finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub struct PgPoolOptions {
    maximum: u32,
    timeout: std::time::Duration,
}
impl Default for PgPoolOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl PgPoolOptions {
    pub fn new() -> Self {
        Self {
            maximum: 5,
            timeout: std::time::Duration::from_secs(3),
        }
    }
    pub fn max_connections(mut self, n: u32) -> Self {
        self.maximum = n;
        self
    }
    pub fn acquire_timeout(mut self, d: std::time::Duration) -> Self {
        self.timeout = d;
        self
    }
    pub async fn connect(self, url: &str) -> Result<PgPool, Error> {
        let pool = self.connect_lazy(url)?;
        query("SELECT 1").execute(&pool).await?;
        Ok(pool)
    }
    pub fn connect_lazy(self, url: &str) -> Result<PgPool, Error> {
        if self.maximum == 0 {
            return Err(decode("pool size must be positive"));
        }
        let command = json!({"op":"postgres_pool","url":url,"maximum":self.maximum,"timeout_ms":self.timeout.as_millis()});
        let bytes = simple_server_sys::resource_new(&serde_json::to_vec(&command).map_err(decode)?)
            .map_err(|e| Error::Configuration(Box::new(e)))?;
        let (header, _) = crate::engine_wire::decode(&bytes).map_err(decode)?;
        Ok(PgPool {
            resource: Resource::new(
                18,
                header["id"]
                    .as_u64()
                    .ok_or_else(|| decode("missing pool ID"))?,
            ),
        })
    }
}
impl PgPool {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        PgPoolOptions::new().connect(url).await
    }

    pub async fn begin(&self) -> Result<Transaction<'static, Postgres>, Error> {
        let (header, _) = command(
            json!({"op":"postgres","action":"begin","pool":self.resource.id()}),
            &[],
        )
        .await?;
        Ok(Transaction {
            connection: Connection {
                resource: Resource::new(
                    19,
                    header["id"]
                        .as_u64()
                        .ok_or_else(|| decode("missing transaction ID"))?,
                ),
            },
            marker: PhantomData,
        })
    }
    pub async fn close(&self) {
        command(
            json!({"op":"postgres","action":"close","pool":self.resource.id()}),
            &[],
        )
        .await
        .expect("engine pool close failed");
    }
}
pub struct Connection {
    resource: Resource,
}
pub struct Transaction<'a, DB = Postgres> {
    connection: Connection,
    marker: PhantomData<&'a mut DB>,
}
impl<DB> Deref for Transaction<'_, DB> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.connection
    }
}
impl<DB> DerefMut for Transaction<'_, DB> {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}
impl<DB> Transaction<'_, DB> {
    pub async fn commit(self) -> Result<(), Error> {
        command(
            json!({"op":"postgres","action":"commit","transaction":self.connection.resource.id()}),
            &[],
        )
        .await?;
        Ok(())
    }
    pub async fn rollback(self) -> Result<(), Error> {
        command(
            json!({"op":"postgres","action":"rollback","transaction":self.connection.resource.id()}),
            &[],
        )
        .await?;
        Ok(())
    }
}

#[doc(hidden)]
pub struct Target {
    resource: Resource,
    transaction: bool,
}
pub trait Executor<'e> {
    type Database;
    #[doc(hidden)]
    fn target(self) -> Target;
}
impl<'e> Executor<'e> for &'e PgPool {
    type Database = Postgres;
    fn target(self) -> Target {
        Target {
            resource: self.resource.clone(),
            transaction: false,
        }
    }
}
impl<'e> Executor<'e> for &'e mut Connection {
    type Database = Postgres;
    fn target(self) -> Target {
        Target {
            resource: self.resource.clone(),
            transaction: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    cells: Vec<(String, Value, bool)>,
}
pub type PgRow = Row;
pub trait ColumnIndex {
    fn index(&self, row: &Row) -> Option<usize>;
}
impl ColumnIndex for usize {
    fn index(&self, row: &Row) -> Option<usize> {
        (*self < row.cells.len()).then_some(*self)
    }
}
impl ColumnIndex for &str {
    fn index(&self, row: &Row) -> Option<usize> {
        row.cells.iter().position(|(name, _, _)| name == self)
    }
}
impl Row {
    pub fn try_get<T: FromValue, I: ColumnIndex>(&self, index: I) -> Result<T, Error> {
        let index = index
            .index(self)
            .ok_or_else(|| decode("column not found"))?;
        if !self.cells[index].2 {
            return Err(decode("unsupported PostgreSQL column type"));
        }
        T::from_value(&self.cells[index].1)
    }
    pub fn get<T: FromValue, I: ColumnIndex>(&self, index: I) -> T {
        self.try_get(index)
            .expect("PostgreSQL column decoding failed")
    }
}
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, Error>;
}
impl<T: serde::de::DeserializeOwned> FromValue for T {
    fn from_value(value: &Value) -> Result<Self, Error> {
        serde_json::from_value(value.clone()).map_err(decode)
    }
}
/// Parameters carry an explicit PostgreSQL type, including typed NULLs.
pub trait IntoParameter {
    fn parameter(self) -> Value;
}
macro_rules! params {($($ty:ty=>$name:literal),*)=>{$(impl IntoParameter for $ty {fn parameter(self)->Value{json!({"type":$name,"value":self})}} impl IntoParameter for Option<$ty>{fn parameter(self)->Value{json!({"type":$name,"value":self,"null":self.is_none()})}})*};}
params!(String=>"text", &str=>"text", i16=>"i16", i32=>"i32", i64=>"i64", bool=>"bool", Vec<u8> =>"bytes", Vec<String> =>"texts", Value=>"json", &[u8] => "bytes", Vec<&str> => "texts", Vec<&String> => "texts");
impl<T: IntoParameter + Clone> IntoParameter for &T {
    fn parameter(self) -> Value {
        self.clone().parameter()
    }
}
impl IntoParameter for &[String] {
    fn parameter(self) -> Value {
        json!({"type":"texts","value":self})
    }
}
pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self, Error>;
}
impl FromRow for Row {
    fn from_row(row: &Row) -> Result<Self, Error> {
        Ok(row.clone())
    }
}
macro_rules! tuple {($($ty:ident:$index:tt),*)=>{impl<$($ty:FromValue),*> FromRow for ($($ty,)*) {fn from_row(row:&Row)->Result<Self,Error> {Ok(($(row.try_get::<$ty,_>($index)?,)*))}}};}
tuple!(A:0);
tuple!(A:0,B:1);
tuple!(A:0,B:1,C:2);
tuple!(A:0,B:1,C:2,D:3);
tuple!(A:0,B:1,C:2,D:3,E:4);
tuple!(A:0,B:1,C:2,D:3,E:4,F:5);

pub struct Query<DB = Postgres> {
    sql: String,
    bindings: Vec<Value>,
    raw: bool,
    marker: PhantomData<DB>,
}
pub fn query(sql: &str) -> Query {
    Query {
        sql: sql.into(),
        bindings: Vec::new(),
        raw: false,
        marker: PhantomData,
    }
}
pub fn raw_sql(sql: &str) -> Query {
    Query {
        raw: true,
        ..query(sql)
    }
}
#[derive(Debug)]
pub struct QueryResult {
    affected: u64,
}
impl QueryResult {
    pub fn rows_affected(&self) -> u64 {
        self.affected
    }
}
impl<DB> Query<DB> {
    pub fn bind(mut self, value: impl IntoParameter) -> Self {
        self.bindings.push(value.parameter());
        self
    }
    async fn run<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
        action: &str,
    ) -> Result<(serde_json::Value, Vec<u8>), Error> {
        let target = executor.target();
        let id = target.resource.id();
        let payload = serde_json::to_vec(&self.bindings).map_err(decode)?;
        command(json!({"op":"postgres","action":if self.raw {"raw"}else {action},"sql":self.sql,"pool":if target.transaction {None}else {Some(id)},"transaction":if target.transaction {Some(id)}else {None}}),&payload).await
    }
    pub async fn execute<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<QueryResult, Error> {
        let (header, _) = self.run(executor, "execute").await?;
        Ok(QueryResult {
            affected: header["rows_affected"]
                .as_u64()
                .ok_or_else(|| decode("missing rows_affected"))?,
        })
    }
    pub async fn fetch_all<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Vec<Row>, Error> {
        self.fetch_rows(executor, "fetch").await
    }
    async fn fetch_rows<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
        action: &str,
    ) -> Result<Vec<Row>, Error> {
        let (header, _) = self.run(executor, action).await?;
        let rows: Vec<Vec<(String, Value, bool)>> =
            serde_json::from_value(header["rows"].clone()).map_err(decode)?;
        Ok(rows.into_iter().map(|cells| Row { cells }).collect())
    }

    pub async fn fetch_optional<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Option<Row>, Error> {
        Ok(self
            .fetch_rows(executor, "fetch_optional")
            .await?
            .into_iter()
            .next())
    }
    pub async fn fetch_one<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Row, Error> {
        self.fetch_optional(executor)
            .await?
            .ok_or(Error::RowNotFound)
    }
}
pub struct QueryAs<DB, O> {
    query: Query<DB>,
    output: PhantomData<O>,
}
pub fn query_as<DB, O: FromRow>(sql: &str) -> QueryAs<DB, O> {
    QueryAs {
        query: Query {
            sql: sql.into(),
            bindings: Vec::new(),
            raw: false,
            marker: PhantomData,
        },
        output: PhantomData,
    }
}
impl<DB, O: FromRow> QueryAs<DB, O> {
    pub fn bind(mut self, value: impl IntoParameter) -> Self {
        self.query = self.query.bind(value);
        self
    }
    pub async fn fetch_all<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Vec<O>, Error> {
        self.query
            .fetch_all(executor)
            .await?
            .iter()
            .map(O::from_row)
            .collect()
    }
    pub async fn fetch_optional<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Option<O>, Error> {
        self.query
            .fetch_optional(executor)
            .await?
            .as_ref()
            .map(O::from_row)
            .transpose()
    }
    pub async fn fetch_one<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<O, Error> {
        O::from_row(&self.query.fetch_one(executor).await?)
    }
}
pub struct Scalar<T>(T);
impl<T: FromValue> FromRow for Scalar<T> {
    fn from_row(row: &Row) -> Result<Self, Error> {
        row.try_get(0_usize).map(Self)
    }
}
pub struct QueryScalar<DB, O>(QueryAs<DB, Scalar<O>>);
pub fn query_scalar<DB, O: FromValue>(sql: &str) -> QueryScalar<DB, O> {
    QueryScalar(query_as(sql))
}
impl<DB, O: FromValue> QueryScalar<DB, O> {
    pub fn bind(mut self, value: impl IntoParameter) -> Self {
        self.0 = self.0.bind(value);
        self
    }
    pub async fn fetch_all<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Vec<O>, Error> {
        Ok(self
            .0
            .fetch_all(executor)
            .await?
            .into_iter()
            .map(|v| v.0)
            .collect())
    }
    pub async fn fetch_optional<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<Option<O>, Error> {
        Ok(self.0.fetch_optional(executor).await?.map(|v| v.0))
    }
    pub async fn fetch_one<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
    ) -> Result<O, Error> {
        Ok(self.0.fetch_one(executor).await?.0)
    }
}

pub type PgConnection = Connection;
pub mod migrate;

impl IntoParameter for Option<&String> {
    fn parameter(self) -> Value {
        self.map(String::as_str).parameter()
    }
}

impl IntoParameter for f64 {
    fn parameter(self) -> Value {
        json!({"type":if self.is_finite(){"f64"}else{"nonfinite_float"},"value":self})
    }
}
impl IntoParameter for Option<f64> {
    fn parameter(self) -> Value {
        self.map(IntoParameter::parameter)
            .unwrap_or_else(|| json!({"type":"f64","value":null,"null":true}))
    }
}
