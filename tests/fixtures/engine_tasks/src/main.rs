use simple_server::{
    engine_tasks::{TaskExit, TaskSet, WorkTracker},
    time::{Instant, sleep},
};
use std::{num::NonZeroUsize, time::Duration};

#[simple_server::main(flavor = "current_thread", start_paused = true)]
async fn main() {
    let tracker = WorkTracker::new();
    let guard = tracker.try_acquire("external").unwrap();
    tracker.close();
    std::thread::spawn(move || drop(guard)).join().unwrap();
    tracker.wait().await;
    let mut tasks = TaskSet::<()>::new(NonZeroUsize::new(2).unwrap());
    let start = Instant::now();
    tasks
        .spawn("worker", Default::default(), |context| async move {
            context.cancelled().await;
            sleep(Duration::from_secs(2)).await;
            Ok(())
        })
        .unwrap();
    tasks.request_shutdown();
    let report = tasks.drain_until(start + Duration::from_secs(5)).await;
    assert!(report.is_drained());
    assert_eq!(report.completed.len(), 1);
    assert!(matches!(
        report.completed[0].exit,
        TaskExit::Finished(Ok(()))
    ));
    assert_eq!(
        report.completed[0].timing.finished,
        Some(start + Duration::from_secs(2))
    );
    println!("engine task supervisor consumer passed");
}
