//! Typed application queries backed by the separately compiled SQLite engine.
use serde_json::json;
pub use simple_server_sys::values::Value;
use simple_server_sys::{Resource, values};
use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
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
                .unwrap_or("engine SQLite error")
                .to_owned(),
            code: header["code"].as_str().map(str::to_owned),
        }))
    }
}

#[derive(Debug)]
pub struct Sqlite;
#[derive(Clone)]
pub struct SqlitePool {
    resource: Resource,
}
impl fmt::Debug for SqlitePool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqlitePool").finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub struct SqliteConnectOptions {
    filename: PathBuf,
    create: bool,
    init_sql: Vec<String>,
}
impl Default for SqliteConnectOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl SqliteConnectOptions {
    pub fn new() -> Self {
        Self {
            filename: PathBuf::from(":memory:"),
            create: false,
            init_sql: Vec::new(),
        }
    }
    pub fn filename(mut self, path: impl AsRef<Path>) -> Self {
        self.filename = path.as_ref().to_owned();
        self
    }
    pub fn create_if_missing(mut self, create: bool) -> Self {
        self.create = create;
        self
    }
    pub fn connection_policy(
        mut self,
        policy: &super::sqlite::ConnectionPolicy,
    ) -> Result<Self, Error> {
        self.init_sql = policy
            .commands()
            .map_err(|e| Error::Configuration(Box::new(e)))?;
        Ok(self)
    }
}
#[derive(Clone, Debug)]
pub struct SqlitePoolOptions {
    maximum: u32,
}
impl Default for SqlitePoolOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl SqlitePoolOptions {
    pub fn new() -> Self {
        Self { maximum: 1 }
    }
    pub fn max_connections(mut self, maximum: u32) -> Self {
        self.maximum = maximum;
        self
    }
    pub async fn connect_with(self, options: SqliteConnectOptions) -> Result<SqlitePool, Error> {
        if self.maximum == 0 {
            return Err(decode("pool size must be positive"));
        }
        let (header,_)=command(json!({"op":"sqlite","action":"connect","filename":options.filename.as_os_str().as_bytes(),"create":options.create,"init_sql":options.init_sql,"max_connections":self.maximum}),&[]).await?;
        Ok(SqlitePool {
            resource: Resource::new(
                3,
                header["id"]
                    .as_u64()
                    .ok_or_else(|| decode("missing pool ID"))?,
            ),
        })
    }
}
impl SqlitePool {
    pub async fn begin(&self) -> Result<Transaction<'static, Sqlite>, Error> {
        let (header, _) = command(
            json!({"op":"sqlite","action":"begin","pool":self.resource.id()}),
            &[],
        )
        .await?;
        Ok(Transaction {
            connection: Connection {
                resource: Resource::new(
                    4,
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
            json!({"op":"sqlite","action":"close","pool":self.resource.id()}),
            &[],
        )
        .await
        .expect("engine pool close failed");
    }
}
pub struct Connection {
    resource: Resource,
}
pub struct Transaction<'a, DB = Sqlite> {
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
            json!({"op":"sqlite","action":"commit","transaction":self.connection.resource.id()}),
            &[],
        )
        .await?;
        Ok(())
    }
    pub async fn rollback(self) -> Result<(), Error> {
        command(
            json!({"op":"sqlite","action":"rollback","transaction":self.connection.resource.id()}),
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
impl<'e> Executor<'e> for &'e SqlitePool {
    type Database = Sqlite;
    fn target(self) -> Target {
        Target {
            resource: self.resource.clone(),
            transaction: false,
        }
    }
}
impl<'e> Executor<'e> for &'e mut Connection {
    type Database = Sqlite;
    fn target(self) -> Target {
        Target {
            resource: self.resource.clone(),
            transaction: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    cells: Vec<(String, Value)>,
}
pub type SqliteRow = Row;
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
        row.cells.iter().position(|(name, _)| name == self)
    }
}
impl Row {
    pub fn try_get<T: FromValue, I: ColumnIndex>(&self, index: I) -> Result<T, Error> {
        let index = index
            .index(self)
            .ok_or_else(|| decode("column not found"))?;
        T::from_value(&self.cells[index].1)
    }
    pub fn get<T: FromValue, I: ColumnIndex>(&self, index: I) -> T {
        self.try_get(index).expect("SQLite column decoding failed")
    }
}
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, Error>;
}
impl FromValue for String {
    fn from_value(v: &Value) -> Result<Self, Error> {
        if let Value::Text(s) = v {
            Ok(s.clone())
        } else {
            Err(decode("expected SQLite text"))
        }
    }
}
impl FromValue for Vec<u8> {
    fn from_value(v: &Value) -> Result<Self, Error> {
        if let Value::Blob(b) = v {
            Ok(b.clone())
        } else {
            Err(decode("expected SQLite blob"))
        }
    }
}
impl FromValue for bool {
    fn from_value(v: &Value) -> Result<Self, Error> {
        Ok(i64::from_value(v)? != 0)
    }
}
impl FromValue for f64 {
    fn from_value(v: &Value) -> Result<Self, Error> {
        match v {
            Value::Real(n) => Ok(*n),
            Value::Integer(n) => Ok(*n as f64),
            _ => Err(decode("expected SQLite real")),
        }
    }
}
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(v: &Value) -> Result<Self, Error> {
        if matches!(v, Value::Null) {
            Ok(None)
        } else {
            T::from_value(v).map(Some)
        }
    }
}
macro_rules! integers {($($ty:ty),*)=>{$(impl FromValue for $ty {fn from_value(v:&Value)->Result<Self,Error> {if let Value::Integer(n)=v {(*n).try_into().map_err(decode)}else {Err(decode("expected SQLite integer"))}}})*};}
integers!(i8, i16, i32, i64, u8, u16, u32, u64);
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

pub struct Query<DB = Sqlite> {
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
pub struct QueryResult {
    affected: u64,
    rowid: i64,
}
impl QueryResult {
    pub fn rows_affected(&self) -> u64 {
        self.affected
    }
    pub fn last_insert_rowid(&self) -> i64 {
        self.rowid
    }
}
impl<DB> Query<DB> {
    pub fn bind(mut self, value: impl Into<Value>) -> Self {
        self.bindings.push(value.into());
        self
    }
    async fn run<'e, E: Executor<'e, Database = DB>>(
        self,
        executor: E,
        action: &str,
    ) -> Result<(serde_json::Value, Vec<u8>), Error> {
        let target = executor.target();
        let id = target.resource.id();
        let payload = values::encode(&self.bindings).map_err(decode)?;
        command(json!({"op":"sqlite","action":if self.raw {"raw"}else {action},"sql":self.sql,"pool":if target.transaction {None}else {Some(id)},"transaction":if target.transaction {Some(id)}else {None}}),&payload).await
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
            rowid: header["last_insert_rowid"]
                .as_i64()
                .ok_or_else(|| decode("missing rowid"))?,
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
        let (header, body) = self.run(executor, action).await?;
        let names: Vec<Vec<String>> =
            serde_json::from_value(header["columns"].clone()).map_err(decode)?;
        let mut cells = values::decode(&body).map_err(decode)?.into_iter();
        let rows = names
            .into_iter()
            .map(|names| {
                names
                    .into_iter()
                    .map(|name| {
                        cells
                            .next()
                            .map(|cell| (name, cell))
                            .ok_or_else(|| decode("missing row cell"))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|cells| Row { cells })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if cells.next().is_some() {
            return Err(decode("trailing row cells"));
        }
        Ok(rows)
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
    pub fn bind(mut self, value: impl Into<Value>) -> Self {
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
    pub fn bind(mut self, value: impl Into<Value>) -> Self {
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

pub use simple_server_macros::FromRow;
