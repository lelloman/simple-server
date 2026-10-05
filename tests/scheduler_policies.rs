#![cfg(all(feature = "task-scheduling", feature = "task-policies"))]
use simple_server::task_scheduling as scheduling;
use simple_server::{lifecycle::Shutdown, tasks};
use tokio::{test as runtime_test, time as clock};
#[path = "support/scheduler_policies_contracts.rs"]
mod contracts;
use contracts::{immediate, limits};
use simple_server::{task_policies::*, task_scheduling::*};
use std::time::Duration;
#[test]
fn blocking_executor_queue_delay_is_not_execution_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (release, held) = std::sync::mpsc::channel();
        let (send, ready) = tokio::sync::oneshot::channel();
        let busy = tokio::task::spawn_blocking(move || {
            send.send(()).unwrap();
            held.recv().unwrap();
        });
        ready.await.unwrap();
        let mut s = Scheduler::<(), &str>::new(limits()).unwrap();
        let stop = Shutdown::new();
        let mut policy = ExecutionPolicy::default();
        policy.budget.max_runtime = Some(Duration::from_millis(5));
        s.register(Job::blocking("queued-in-tokio", immediate(), |_| Ok(())).with_policy(policy))
            .unwrap();
        let (send, mut admitted) = tokio::sync::mpsc::unbounded_channel();
        let (result, _) = tokio::join!(
            s.run(stop.clone(), |event| match event {
                Event::Admitted { .. } => {
                    send.send(()).unwrap();
                }
                Event::RuntimeExceeded { .. } => panic!("executor queue time is not runtime"),
                Event::Completed {
                    runtime_exceeded, ..
                } => {
                    assert!(!runtime_exceeded);
                    stop.request();
                }
                _ => {}
            }),
            async {
                admitted.recv().await.unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
                release.send(()).unwrap();
            }
        );
        result.unwrap();
        busy.await.unwrap();
    });
}
