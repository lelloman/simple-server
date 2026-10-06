#![cfg(feature = "postgres-client")]
use simple_server::database::postgres::{self as db, PgPoolOptions};

#[simple_server::test]
#[ignore = "requires isolated PostgreSQL in SIMPLE_SERVER_TEST_POSTGRES"]
async fn postgres_types_transactions_and_cancellation() {
    exercise().await;
}

#[simple_server::test(host_runtime = true)]
#[ignore = "requires isolated PostgreSQL in SIMPLE_SERVER_TEST_POSTGRES"]
async fn postgres_drop_on_external_executor_keeps_native_context() {
    exercise().await;
}

async fn exercise() {
    let url = std::env::var("SIMPLE_SERVER_TEST_POSTGRES").unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    db::query("CREATE TEMP TABLE runtime_contract (id bigint PRIMARY KEY, value jsonb)")
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    db::query("INSERT INTO runtime_contract VALUES ($1,$2)")
        .bind(1_i64)
        .bind(serde_json::json!({"s":"a\u{0}b","n":7}))
        .execute(&mut *tx)
        .await
        .expect_err("PostgreSQL rejects NUL in JSON strings");
    tx.rollback().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    db::query("INSERT INTO runtime_contract VALUES ($1,$2)")
        .bind(1_i64)
        .bind(serde_json::json!({"n":7}))
        .execute(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    let count: i64 = db::query_scalar("SELECT count(*) FROM runtime_contract")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "dropping a transaction must roll back before pool reuse"
    );
    let mut tx = pool.begin().await.unwrap();
    assert!(
        simple_server::time::timeout(
            std::time::Duration::from_millis(20),
            db::query("SELECT pg_sleep(0.1)").execute(&mut *tx)
        )
        .await
        .is_err()
    );
    drop(tx);
    let row=db::query("SELECT $1::bytea AS bytes,$2::text[] AS texts,$3::bigint AS absent,$4::jsonb AS document,$5::boolean AS flag,$6::double precision AS number")
        .bind(vec![0_u8,255,42]).bind(vec!["a", "b"]).bind(None::<i64>).bind(serde_json::json!({"n":7})).bind(true).bind(1.25_f64).fetch_one(&pool).await.unwrap();
    assert_eq!(row.get::<Vec<u8>, _>("bytes"), [0, 255, 42]);
    assert_eq!(row.get::<Vec<String>, _>("texts"), ["a", "b"]);
    assert_eq!(row.get::<Option<i64>, _>("absent"), None);
    assert_eq!(
        row.get::<serde_json::Value, _>("document"),
        serde_json::json!({"n":7})
    );
    assert!(row.get::<bool, _>("flag"));
    assert_eq!(row.get::<f64, _>("number"), 1.25);
    db::query("INSERT INTO runtime_contract VALUES(1,'null')")
        .execute(&pool)
        .await
        .unwrap();
    let error = db::query("INSERT INTO runtime_contract VALUES(1,'null')")
        .execute(&pool)
        .await
        .err()
        .unwrap();
    assert_eq!(error.as_database_error().unwrap().code(), Some("23505"));
    let row = db::query("SELECT $1::jsonb IS NULL AS absent, $2::jsonb = 'null'::jsonb AS json_null, clock_timestamp() AS unsupported")
        .bind(None::<serde_json::Value>).bind(serde_json::Value::Null).fetch_one(&pool).await.unwrap();
    assert!(row.get::<bool, _>("absent"));
    assert!(row.get::<bool, _>("json_null"));
    assert!(row.try_get::<serde_json::Value, _>("unsupported").is_err());
    assert!(
        db::query("SELECT $1::float8")
            .bind(f64::NAN)
            .fetch_one(&pool)
            .await
            .is_err()
    );
    let row = db::query("SELECT 'NaN'::float8 AS nonfinite")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(row.try_get::<Option<f64>, _>("nonfinite").is_err());
    pool.close().await;
}

#[simple_server::test]
#[ignore = "requires isolated PostgreSQL in SIMPLE_SERVER_TEST_POSTGRES"]
async fn native_migrations_preserve_checksums_and_reject_modified_sql() {
    use db::migrate::{Migration, MigrationType, Migrator};
    let url = std::env::var("SIMPLE_SERVER_TEST_POSTGRES").unwrap();
    let admin = db::PgPool::connect(&url).await.unwrap();
    let schema = format!(
        "engine_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    db::raw_sql(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    let separator = if url.contains('?') { '&' } else { '?' };
    let pool = db::PgPool::connect(&format!("{url}{separator}options=-csearch_path%3D{schema}"))
        .await
        .unwrap();
    let mut migrator = Migrator {
        migrations: vec![Migration::new(
            1,
            "first".into(),
            MigrationType::Simple,
            "CREATE TABLE data(id bigint);".into(),
            false,
        )]
        .into(),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    };
    migrator.run(&pool).await.unwrap();
    migrator.run(&pool).await.unwrap();
    migrator.migrations = vec![Migration::new(
        1,
        "first".into(),
        MigrationType::Simple,
        "CREATE TABLE data(id text);".into(),
        false,
    )]
    .into();
    assert!(migrator.run(&pool).await.is_err());
    pool.close().await;
    db::raw_sql(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
