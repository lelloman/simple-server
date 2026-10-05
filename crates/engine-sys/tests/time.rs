use simple_server_sys::{
    Builder, Elapsed, Instant, advance, sleep, sleep_until, timeout, timeout_at, yield_now,
};
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
    time::Duration,
};

#[test]
fn paused_clock_arithmetic_and_absolute_timers_use_one_origin() {
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let start = Instant::now();
            assert_eq!(start, Instant::now());
            let prior = start - Duration::from_nanos(1);
            assert_eq!(start - prior, Duration::from_nanos(1));
            assert_eq!(prior.duration_since(start), Duration::ZERO);
            assert!(prior.checked_duration_since(start).is_none());
            assert!(start.checked_add(Duration::MAX).is_none());
            assert!(start.checked_sub(Duration::MAX).is_none());
            let deadline = start + Duration::from_secs(3);
            sleep_until(deadline).await;
            assert_eq!(Instant::now(), deadline);
            assert_eq!(start.elapsed(), Duration::from_secs(3));
            sleep_until(prior).await;
            assert_eq!(Instant::now(), deadline);
        });
}

#[test]
fn relative_sleep_and_timeout_fix_deadline_at_construction() {
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let start = Instant::now();
            let timer = sleep(Duration::from_secs(5));
            assert_eq!(timer.deadline(), start + Duration::from_secs(5));
            let timeout = timeout(Duration::from_secs(5), std::future::pending::<()>());
            advance(Duration::from_secs(6)).await;
            timer.await;
            assert_eq!(timeout.await, Err(Elapsed));
            assert_eq!(Instant::now() - start, Duration::from_secs(6));
        });
}

#[test]
fn elapsed_timer_can_be_repolled_and_ready_value_beats_expired_timeout() {
    Builder::new_current_thread()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let now = Instant::now();
            assert_eq!(timeout_at(now, async { 17 }).await, Ok(17));
            let mut timer = pin!(sleep_until(now));
            timer.as_mut().await;
            assert_eq!(
                timer.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(())
            );
            yield_now().await;
        });
}

#[test]
fn real_clock_is_available_without_an_active_runtime() {
    let before = Instant::now();
    std::thread::sleep(Duration::from_millis(2));
    assert!(before.elapsed() >= Duration::from_millis(1));
}
