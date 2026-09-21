use simple_server::tasks::{TaskSet, WorkTracker};
use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let callbacks = WorkTracker::new();
    let reservation = callbacks.try_acquire("upgrade").unwrap();
    callbacks.close();
    drop(reservation); // External callback or failed handshake has finished.
    callbacks.wait().await;
    let mut jobs = TaskSet::<std::io::Error>::new(NonZeroUsize::new(4).unwrap());
    jobs.spawn("worker", Default::default(), |ctx| async move {
        ctx.cancelled().await;
        Ok(())
    })
    .unwrap();
    jobs.request_shutdown();
    let report = jobs
        .drain_until(Instant::now() + Duration::from_secs(1))
        .await;
    assert!(report.is_drained());
}
