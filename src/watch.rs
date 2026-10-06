//! Runtime-independent latest-value notifications.
//!
//! Receivers observe the newest value, coalescing intermediate sends. Cancelling
//! `changed` neither consumes an update nor retains a waker. This deliberately
//! exposes only the channel operations needed by shared application workers.
use std::{
    collections::BTreeMap,
    fmt,
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};
struct State<T> {
    value: T,
    version: u64,
    senders: usize,
    receivers: usize,
    next_waiter: u64,
    waiters: BTreeMap<u64, Waker>,
}
pub struct Sender<T>(Arc<Mutex<State<T>>>);
pub struct Receiver<T> {
    state: Arc<Mutex<State<T>>>,
    seen: u64,
}
/// The initial value is considered seen by the initial receiver.
pub fn channel<T>(value: T) -> (Sender<T>, Receiver<T>) {
    let state = Arc::new(Mutex::new(State {
        value,
        version: 0,
        senders: 1,
        receivers: 1,
        next_waiter: 0,
        waiters: BTreeMap::new(),
    }));
    (Sender(state.clone()), Receiver { state, seen: 0 })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecvError;
impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("watch channel closed")
    }
}
impl std::error::Error for RecvError {}
pub struct SendError<T>(pub T);
impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SendError(..)")
    }
}
impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("watch channel has no receivers")
    }
}
impl<T> std::error::Error for SendError<T> {}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.0.lock().unwrap().senders += 1;
        Self(self.0.clone())
    }
}
impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let (previous, waiters) = {
            let mut state = self.0.lock().unwrap();
            if state.receivers == 0 {
                return Err(SendError(value));
            }
            let previous = std::mem::replace(&mut state.value, value);
            state.version = state
                .version
                .checked_add(1)
                .expect("watch version exhausted");
            (previous, std::mem::take(&mut state.waiters))
        };
        // Values and wakers may reenter application code: drop/wake outside locks.
        drop(previous);
        for waker in waiters.into_values() {
            waker.wake();
        }
        Ok(())
    }
}
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let waiters = {
            let mut state = self.0.lock().unwrap();
            state.senders -= 1;
            if state.senders != 0 {
                return;
            }
            std::mem::take(&mut state.waiters)
        };
        for waker in waiters.into_values() {
            waker.wake();
        }
    }
}
impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        self.state.lock().unwrap().receivers += 1;
        Self {
            state: self.state.clone(),
            seen: self.seen,
        }
    }
}
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.state.lock().unwrap().receivers -= 1;
    }
}
pub struct Ref<'a, T>(MutexGuard<'a, State<T>>);
impl<T> Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.value
    }
}
impl<T> Receiver<T> {
    /// Synchronous observation reports closure even if an unseen value remains.
    pub fn has_changed(&self) -> Result<bool, RecvError> {
        let state = self.state.lock().unwrap();
        if state.senders == 0 {
            Err(RecvError)
        } else {
            Ok(state.version != self.seen)
        }
    }
    /// Hold only briefly; the borrow locks out sends. Never hold across an await.
    pub fn borrow_and_update(&mut self) -> Ref<'_, T> {
        let state = self.state.lock().unwrap();
        self.seen = state.version;
        Ref(state)
    }
    /// Mark the newest version seen when ready; deliver an unseen final value
    /// before reporting closure. Cancellation leaves the seen version unchanged.
    pub async fn changed(&mut self) -> Result<(), RecvError> {
        Changed {
            receiver: self,
            waiter: None,
        }
        .await
    }
}
struct Changed<'a, T> {
    receiver: &'a mut Receiver<T>,
    waiter: Option<u64>,
}
impl<T> Future for Changed<'_, T> {
    type Output = Result<(), RecvError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut removed = None;
        let waker = cx.waker().clone();
        let mut state = this.receiver.state.lock().unwrap();
        if state.version != this.receiver.seen || state.senders == 0 {
            if let Some(id) = this.waiter.take() {
                removed = state.waiters.remove(&id);
            }
            if state.version != this.receiver.seen {
                this.receiver.seen = state.version;
                drop(state);
                drop(removed);
                return Poll::Ready(Ok(()));
            }
            drop(state);
            drop(removed);
            return Poll::Ready(Err(RecvError));
        }
        let id = *this.waiter.get_or_insert_with(|| {
            let id = state.next_waiter;
            state.next_waiter = id.checked_add(1).expect("watch waiter IDs exhausted");
            id
        });
        let previous = state.waiters.insert(id, waker);
        drop(state);
        drop(previous);
        Poll::Pending
    }
}
impl<T> Drop for Changed<'_, T> {
    fn drop(&mut self) {
        if let Some(id) = self.waiter {
            let previous = self.receiver.state.lock().unwrap().waiters.remove(&id);
            drop(previous);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn poll<T>(future: Pin<&mut Changed<'_, T>>) -> Poll<Result<(), RecvError>> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }
    #[test]
    fn pending_receivers_are_woken_on_send_and_close() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Count(AtomicUsize);
        impl std::task::Wake for Count {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let count = Arc::new(Count(AtomicUsize::new(0)));
        let waker = Waker::from(count.clone());
        let mut cx = Context::from_waker(&waker);
        let (tx, mut rx) = channel(0);
        let mut other = rx.clone();
        let mut first = Box::pin(rx.changed());
        let mut second = Box::pin(other.changed());
        assert!(first.as_mut().poll(&mut cx).is_pending());
        assert!(second.as_mut().poll(&mut cx).is_pending());
        tx.send(1).unwrap();
        assert_eq!(count.0.load(Ordering::SeqCst), 2);
        assert_eq!(first.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
        assert_eq!(second.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
        drop(first);
        drop(second);
        let mut wait = Box::pin(rx.changed());
        assert!(wait.as_mut().poll(&mut cx).is_pending());
        drop(tx);
        assert_eq!(count.0.load(Ordering::SeqCst), 3);
        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Ready(Err(RecvError)));
    }
    #[test]
    fn cancellation_coalescing_and_independent_receivers() {
        let (tx, mut rx) = channel(0);
        let mut other = rx.clone();
        let mut wait = Box::pin(Changed {
            receiver: &mut rx,
            waiter: None,
        });
        assert!(poll(wait.as_mut()).is_pending());
        assert_eq!(tx.0.lock().unwrap().waiters.len(), 1);
        drop(wait);
        assert!(tx.0.lock().unwrap().waiters.is_empty());
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        assert_eq!(rx.has_changed(), Ok(true));
        assert_eq!(*rx.borrow_and_update(), 2);
        assert_eq!(rx.has_changed(), Ok(false));
        assert_eq!(other.has_changed(), Ok(true));
        let mut wait = Box::pin(Changed {
            receiver: &mut other,
            waiter: None,
        });
        assert_eq!(poll(wait.as_mut()), Poll::Ready(Ok(())));
        drop(wait);
        assert_eq!(other.has_changed(), Ok(false));
    }
    #[test]
    fn final_unseen_value_is_delivered_before_close_and_send_rejects_no_receivers() {
        let (tx, mut rx) = channel(0);
        let clone = tx.clone();
        drop(tx);
        assert_eq!(rx.has_changed(), Ok(false));
        clone.send(1).unwrap();
        drop(clone);
        assert_eq!(rx.has_changed(), Err(RecvError));
        let mut wait = Box::pin(Changed {
            receiver: &mut rx,
            waiter: None,
        });
        assert_eq!(poll(wait.as_mut()), Poll::Ready(Ok(())));
        drop(wait);
        let mut wait = Box::pin(Changed {
            receiver: &mut rx,
            waiter: None,
        });
        assert_eq!(poll(wait.as_mut()), Poll::Ready(Err(RecvError)));
        let (tx, rx) = channel(0);
        drop(rx);
        assert_eq!(tx.send(2).unwrap_err().0, 2);
        assert_eq!(tx.0.lock().unwrap().value, 0);
    }
}
