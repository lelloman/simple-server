//! Drivers for application-owned durable work. No storage or leases are inferred.
use std::{future::Future, num::NonZeroUsize, time::Duration};
use tokio::task::{JoinError, JoinSet};

/// Result of a completed claim-and-execute cycle. The caller commits durable
/// outcomes before returning. Errors can be logged/classified by the callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollOutcome {
    Worked,
    Idle,
    Failed,
}

/// Delay after completion (not a fixed-rate timer). Each state is independent:
/// busy queues may run immediately, while idle/error paths back off. A zero delay
/// yields once so an immediately-ready callback cannot starve shutdown/tasks.
#[derive(Clone, Copy, Debug)]
pub struct PollCadence {
    pub worked: Duration,
    pub idle: Duration,
    pub failed: Duration,
}
impl PollCadence {
    pub fn delay(self, outcome: PollOutcome) -> Duration {
        match outcome {
            PollOutcome::Worked => self.worked,
            PollOutcome::Idle => self.idle,
            PollOutcome::Failed => self.failed,
        }
    }
}

/// Poll while admission is open, completing an accepted cycle before checking
/// again. This driver never races shutdown against a claim or execution future.
/// The caller controls waking/timer/stop behavior in `wait`, including whether
/// errors are wakeable. External cancellation drops the current cycle; durable
/// recovery/fencing is the backend's responsibility. Own this future in the
/// application's task supervisor; this function starts no hidden task.
pub async fn run_poll_worker<S, C, CF, W, WF>(
    mut admitted: S,
    cadence: PollCadence,
    mut cycle: C,
    mut wait: W,
) where
    S: FnMut() -> bool,
    C: FnMut() -> CF,
    CF: Future<Output = PollOutcome>,
    W: FnMut(PollOutcome, Duration) -> WF,
    WF: Future<Output = ()>,
{
    while admitted() {
        let outcome = cycle().await;
        let delay = cadence.delay(outcome);
        if delay.is_zero() {
            tokio::task::yield_now().await;
        } else {
            wait(outcome, delay).await;
        }
    }
}

/// Execute a finite caller-selected batch with a strict bound on spawned work.
/// Input order governs admission; completion order governs observation. Each
/// callback owns its atomic claim/eligibility checks. A panic is observed like
/// any completion and does not strand later candidates. Dropping this future
/// aborts its JoinSet; normal completion drains every accepted item.
pub async fn run_bounded_batch<I, T, F, Fut, R, O>(
    items: I,
    concurrency: NonZeroUsize,
    mut execute: F,
    mut observe: O,
) where
    I: IntoIterator<Item = T>,
    F: FnMut(T) -> Fut,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
    O: FnMut(Result<R, JoinError>),
{
    let mut running = JoinSet::new();
    for item in items {
        if running.len() == concurrency.get()
            && let Some(result) = running.join_next().await
        {
            observe(result);
        }
        running.spawn(execute(item));
    }
    while let Some(result) = running.join_next().await {
        observe(result);
    }
}
