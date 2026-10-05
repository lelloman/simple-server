use simple_server::{database::client as db, process::Command, runtime, time};
use std::time::Duration;

#[simple_server::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = db::SqlitePoolOptions::new()
        .connect_with(db::SqliteConnectOptions::new())
        .await?;
    let value: i64 = db::query_scalar("SELECT ?")
        .bind(i64::MAX)
        .fetch_one(&pool)
        .await?;
    assert_eq!(value, i64::MAX);
    pool.close().await;
    assert_eq!(
        Command::new("/bin/echo")
            .arg("engine")
            .output()
            .await?
            .stdout,
        b"engine\n"
    );
    let value = runtime::spawn(async {
        time::sleep(Duration::from_millis(1)).await;
        42
    })
    .await?;
    assert_eq!(value, 42);
    let _client = simple_server::client::Client::builder()
        .no_proxy()
        .build()?;
    println!("engine consumer passed");
    Ok(())
}
