//! Opt-in Tokio waiting adapter; the synchronous rate-limit feature stays runtime-free.
use std::{sync::Arc, time::Duration};
use tokio::sync::{AcquireError, Semaphore};

/// FIFO semaphore admission whose permits return after a configured hold time.
///
/// This preserves burst-and-delayed-release pacing: a burst of N is immediate,
/// then each admitted slot is replenished independently after the delay. It is
/// not a sustained N-per-second budget or an HTTP-operation concurrency cap.
///
/// Each successful acquire explicitly spawns one Tokio release timer. The timer
/// owns its permit independently of the request lifetime; cancellation while
/// waiting takes no slot. Hold time begins when that timer is polled, matching
/// caller-owned spawn/sleep/drop implementations. Requires an active Tokio
/// runtime with time enabled. Runtime shutdown drops outstanding timer permits.
#[derive(Clone, Debug)]
pub struct DelayedReleaseLimiter {
    semaphore: Arc<Semaphore>,
    hold: Duration,
}
impl DelayedReleaseLimiter {
    /// Zero permits deliberately waits indefinitely unless the limiter closes.
    /// Panics for capacities above Tokio's Semaphore::MAX_PERMITS.
    pub fn new(permits: usize, hold: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            hold,
        }
    }
    pub async fn acquire(&self) -> Result<(), AcquireError> {
        let permit = self.semaphore.clone().acquire_owned().await?;
        let hold = self.hold;
        tokio::spawn(async move {
            tokio::time::sleep(hold).await;
            drop(permit);
        });
        Ok(())
    }
    /// Wake pending waiters with an error and reject later admission.
    pub fn close(&self) {
        self.semaphore.close();
    }
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}
