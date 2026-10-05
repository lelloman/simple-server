use simple_server::{
    runtime,
    task_drivers::{PollCadence, PollOutcome, run_bounded_batch, run_poll_worker},
    time,
};
use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    time::Duration,
};

#[simple_server::main(flavor = "current_thread", start_paused = true)]
async fn main() {
    let mut completed = Vec::new();
    run_bounded_batch(
        0..8,
        NonZeroUsize::new(2).unwrap(),
        |value| async move {
            assert!(runtime::Handle::try_current().is_ok());
            time::sleep(Duration::from_secs(1)).await;
            value * 2
        },
        |result| completed.push(result.unwrap()),
    )
    .await;
    completed.sort();
    assert_eq!(completed, (0..8).map(|value| value * 2).collect::<Vec<_>>());

    let attempts = Cell::new(0);
    let waited = RefCell::new(Vec::new());
    run_poll_worker(
        || attempts.get() < 3,
        PollCadence {
            worked: Duration::ZERO,
            idle: Duration::from_secs(1),
            failed: Duration::from_secs(2),
        },
        || {
            let attempt = attempts.get() + 1;
            attempts.set(attempt);
            async move {
                assert!(runtime::Handle::try_current().is_ok());
                match attempt {
                    1 => PollOutcome::Worked,
                    2 => PollOutcome::Idle,
                    _ => PollOutcome::Failed,
                }
            }
        },
        |outcome, delay| {
            waited.borrow_mut().push((outcome, delay));
            time::sleep(delay)
        },
    )
    .await;
    assert_eq!(attempts.get(), 3);
    assert_eq!(
        waited.into_inner(),
        vec![
            (PollOutcome::Idle, Duration::from_secs(1)),
            (PollOutcome::Failed, Duration::from_secs(2))
        ]
    );
    println!("engine task drivers consumer passed");
}
