#![cfg(feature = "runtime")]
use simple_server::time::{self, MissedTickBehavior};
use std::time::Duration;

#[simple_server::test(start_paused = true)]
async fn missed_ticks_preserve_burst_delay_and_skip_contracts() {
    for (behavior, expected) in [
        (MissedTickBehavior::Burst, 20),
        (MissedTickBehavior::Delay, 45),
        (MissedTickBehavior::Skip, 40),
    ] {
        let start = time::Instant::now();
        let mut interval = time::interval(Duration::from_millis(10));
        interval.set_missed_tick_behavior(behavior);
        assert_eq!(interval.tick().await, start);
        time::advance(Duration::from_millis(35)).await;
        assert_eq!(interval.tick().await - start, Duration::from_millis(10));
        assert_eq!(
            interval.tick().await - start,
            Duration::from_millis(expected)
        );
    }
}
#[simple_server::test(start_paused = true)]
async fn cancelling_a_pending_tick_keeps_its_deadline() {
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Waker},
    };
    let start = time::Instant::now();
    let mut interval = time::interval(Duration::from_secs(1));
    interval.tick().await;
    {
        let mut pending = pin!(interval.tick());
        assert!(
            pending
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
    }
    time::advance(Duration::from_secs(1)).await;
    assert_eq!(interval.tick().await, start + Duration::from_secs(1));
}
