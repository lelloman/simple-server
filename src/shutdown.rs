use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

#[derive(Debug, Default)]
struct State {
    requested: bool,
    next: u64,
    waiters: BTreeMap<u64, Waker>,
}

/// Sticky, idempotent shutdown notification, independent of the runtime.
/// Dropping a handle does not request shutdown. Cancelling a wait unregisters it.
#[derive(Clone, Debug, Default)]
pub struct Shutdown(Arc<Mutex<State>>);
impl Shutdown {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_requested(&self) -> bool {
        self.0.lock().unwrap().requested
    }
    pub fn request(&self) {
        let waiters = {
            let mut state = self.0.lock().unwrap();
            state.requested = true;
            std::mem::take(&mut state.waiters)
        };
        // Wakers may reenter application code, so never wake under the lock.
        for waker in waiters.into_values() {
            waker.wake();
        }
    }
    pub async fn requested(&self) {
        Requested {
            shutdown: self,
            id: None,
        }
        .await
    }
}
struct Requested<'a> {
    shutdown: &'a Shutdown,
    id: Option<u64>,
}
impl Future for Requested<'_> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.shutdown.0.lock().unwrap();
        if state.requested {
            return Poll::Ready(());
        }
        let id = *self.id.get_or_insert_with(|| {
            let id = state.next;
            state.next = state
                .next
                .checked_add(1)
                .expect("shutdown waiter IDs exhausted");
            id
        });
        state.waiters.insert(id, cx.waker().clone());
        Poll::Pending
    }
}
impl Drop for Requested<'_> {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            self.shutdown.0.lock().unwrap().waiters.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_waits_unregister_and_requests_remain_sticky() {
        let shutdown = Shutdown::new();
        let mut waiting = Box::pin(shutdown.requested());
        assert!(
            waiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        assert_eq!(shutdown.0.lock().unwrap().waiters.len(), 1);
        drop(waiting);
        assert!(shutdown.0.lock().unwrap().waiters.is_empty());
        shutdown.request();
        let mut waiting = Box::pin(shutdown.requested());
        assert!(
            waiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_ready()
        );
        assert!(shutdown.0.lock().unwrap().waiters.is_empty());
    }
}
