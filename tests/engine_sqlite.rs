#![cfg(feature = "sqlite-client")]
use simple_server::database::client::{
    self as db, FromRow, SqliteConnectOptions, SqlitePoolOptions,
};

#[derive(Debug, FromRow, PartialEq)]
struct Example {
    value: i64,
    bytes: Vec<u8>,
    optional: Option<String>,
    #[simple_server(rename = "type")]
    kind: String,
}

#[simple_server::test]
async fn query_values_transactions_and_dropped_transaction_rollback() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new())
        .await
        .unwrap();
    db::raw_sql("CREATE TABLE sample(value INTEGER, bytes BLOB, optional TEXT, type TEXT); CREATE TABLE other(id INTEGER);").execute(&pool).await.unwrap();
    let mut transaction = pool.begin().await.unwrap();
    db::query("INSERT INTO sample VALUES(?,?,?,?)")
        .bind(i64::MAX)
        .bind(vec![0_u8, 255])
        .bind(None::<String>)
        .bind("first")
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    let value = db::query_as::<_, Example>("SELECT * FROM sample")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        value,
        Example {
            value: i64::MAX,
            bytes: vec![0, 255],
            optional: None,
            kind: "first".into()
        }
    );
    let mut transaction = pool.begin().await.unwrap();
    db::query("INSERT INTO sample VALUES(0,x'',NULL,'rollback')")
        .execute(&mut *transaction)
        .await
        .unwrap();
    drop(transaction);
    assert_eq!(
        db::query_scalar::<_, i64>("SELECT count(*) FROM sample")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        db::query("SELECT * FROM sample WHERE 0")
            .fetch_one(&pool)
            .await,
        Err(db::Error::RowNotFound)
    ));
    pool.close().await;
}
