#![cfg(feature = "runtime")]

#[simple_server::test(start_paused = true)]
async fn owned_entry_point_runs_with_engine_clock() {
    let task = simple_server::runtime::spawn(async {
        simple_server::time::sleep(std::time::Duration::from_secs(10)).await;
        42
    });
    simple_server::runtime::yield_now().await;
    simple_server::time::advance(std::time::Duration::from_secs(10)).await;
    assert_eq!(task.await.unwrap(), 42);
}

#[simple_server::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_entry_point_propagates_results() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(simple_server::runtime::spawn(async { 3 }).await?, 3);
    Ok(())
}
