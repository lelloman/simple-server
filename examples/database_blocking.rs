//! Application-defined priorities and lanes, with explicit real-work drain.
use simple_server::database::blocking::{CloseMode, Config, Executor, Options, Priority};
use std::{convert::Infallible, num::NonZeroUsize, time::Duration};
fn main() {
    let one = NonZeroUsize::new(1).unwrap();
    let executor = Executor::new(Config {
        workers: one,
        priorities: vec![Priority {
            weight: one,
            queue_capacity: NonZeroUsize::new(8).unwrap(),
        }],
        lane_limits: vec![one],
    })
    .unwrap();
    let ticket = executor
        .submit(
            Options {
                priority: 0,
                lane: 0,
                queue_timeout: Some(Duration::from_secs(1)),
                execution_timeout: Some(Duration::from_secs(5)),
            },
            || Ok::<_, Infallible>("typed database result"),
        )
        .unwrap();
    println!("{}", ticket.wait_blocking().unwrap());
    executor.close(CloseMode::Drain);
    assert!(executor.drain_blocking(Duration::from_secs(5)));
}
