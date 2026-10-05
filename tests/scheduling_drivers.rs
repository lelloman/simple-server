#![cfg(feature = "task-scheduling")]
use simple_server::task_scheduling as drivers;
use simple_server::task_scheduling::{PreferenceOrder, missing_last};
use tokio::{task as execution, test as runtime_test, time};

#[path = "support/driver_contracts.rs"]
mod contracts;

#[test]
fn preference_and_missing_ranks_have_stable_extreme_boundaries() {
    let order = PreferenceOrder::new(["high".to_string(), "low".into(), "high".into()]);
    assert_eq!(order.position("high"), 2);
    assert_eq!(order.position("low"), 1);
    assert_eq!(order.position("unknown"), usize::MAX);
    let mut values = vec![None, Some(i64::MAX), Some(i64::MIN), Some(0)];
    values.sort_by_key(|v| missing_last(*v));
    assert_eq!(values, vec![Some(i64::MIN), Some(0), Some(i64::MAX), None]);
}
