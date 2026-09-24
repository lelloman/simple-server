#![cfg(feature = "rate-limit-async")]
use simple_server::rate_limit::DelayedReleaseLimiter;
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn burst_then_delayed_return_is_shared_and_independent_of_request_lifetime() {
    let limiter = DelayedReleaseLimiter::new(2, Duration::from_millis(22));
    let started = tokio::time::Instant::now();
    limiter.acquire().await.unwrap();
    limiter.clone().acquire().await.unwrap();
    assert_eq!(tokio::time::Instant::now(), started);
    assert_eq!(limiter.available_permits(), 0);
    tokio::task::yield_now().await; // release timers begin on their first poll
    tokio::time::advance(Duration::from_millis(21)).await;
    assert_eq!(limiter.available_permits(), 0);
    tokio::time::advance(Duration::from_millis(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(limiter.available_permits(), 2);
    limiter.acquire().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn cancelled_waiter_never_consumes_a_future_slot() {
    let limiter = DelayedReleaseLimiter::new(1, Duration::from_secs(1));
    limiter.acquire().await.unwrap();
    let clone = limiter.clone();
    let waiting = tokio::spawn(async move { clone.acquire().await.unwrap() });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    waiting.abort();
    assert!(waiting.await.unwrap_err().is_cancelled());
    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(limiter.available_permits(), 1);
    limiter.acquire().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn zero_capacity_waits_and_close_wakes_waiters_without_charging() {
    let limiter = DelayedReleaseLimiter::new(0, Duration::ZERO);
    let clone = limiter.clone();
    let waiter = tokio::spawn(async move { clone.acquire().await });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());
    limiter.close();
    assert!(waiter.await.unwrap().is_err());
    assert!(limiter.acquire().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn queued_waiters_keep_fifo_order_across_releases() {
    let limiter = DelayedReleaseLimiter::new(1, Duration::from_millis(22));
    limiter.acquire().await.unwrap();
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    for i in 0..3 {
        let (limit, send) = (limiter.clone(), send.clone());
        tokio::spawn(async move {
            limit.acquire().await.unwrap();
            send.send(i).unwrap();
        });
        tokio::task::yield_now().await;
    }
    for i in 0..3 {
        assert_eq!(recv.recv().await, Some(i));
    }
}
