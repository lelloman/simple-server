//! Application-owned serialization. A real service saves this representation in
//! its own database; neither the policy nor this example implements a job queue.
use simple_server::task_policies::{
    CircuitBreaker, CircuitOutcome, CircuitPolicy, CircuitSnapshot,
};
use std::{
    num::NonZeroU32,
    time::{Duration, SystemTime},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = CircuitPolicy {
        failure_threshold: NonZeroU32::new(1).unwrap(),
        cooldown: Duration::from_secs(30),
    };
    let mut breaker = CircuitBreaker::new(policy.clone())?;
    let now = SystemTime::now();
    let permit = breaker.admit(now).unwrap();
    breaker.record(permit, CircuitOutcome::Failure, now);
    let state = breaker.snapshot();
    let deadline = state
        .open_until
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)?;
    // The application chooses the schema and storage transaction boundary.
    let stored = serde_json::to_string(&serde_json::json!({
        "version": 1, "failures": state.consecutive_failures,
        "open_until_secs": deadline.as_secs(), "open_until_nanos": deadline.subsec_nanos()
    }))?;
    let row: serde_json::Value = serde_json::from_str(&stored)?;
    let restored = CircuitSnapshot {
        consecutive_failures: row["failures"].as_u64().unwrap().try_into()?,
        open_until: Some(
            SystemTime::UNIX_EPOCH
                + Duration::new(
                    row["open_until_secs"].as_u64().unwrap(),
                    row["open_until_nanos"].as_u64().unwrap().try_into()?,
                ),
        ),
    };
    let restored = CircuitBreaker::restore(policy, restored)?;
    assert!(!restored.can_admit(now));
    assert_eq!(restored.snapshot(), state);
    Ok(())
}
