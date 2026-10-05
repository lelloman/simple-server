use crate::{AbortHandle, Handle, Id, JoinError, JoinHandle};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Wake, Waker},
};

#[derive(Default)]
struct Ready {
    ids: VecDeque<Id>,
    queued: BTreeSet<Id>,
    waiter: Option<Waker>,
}
struct TaskWake {
    id: Id,
    ready: Weak<Mutex<Ready>>,
}
impl Wake for TaskWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let Some(ready) = self.ready.upgrade() else {
            return;
        };
        let waiter = {
            let mut ready = ready.lock().unwrap();
            if !ready.queued.insert(self.id) {
                return;
            }
            ready.ids.push_back(self.id);
            ready.waiter.clone()
        };
        // Application wakers can reenter or panic; never call one under the lock.
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}
struct Entry<T> {
    handle: JoinHandle<T>,
    waker: Waker,
}

/// A collection of engine tasks, yielding results as tasks become ready.
///
/// Results (including panic payloads) remain in the host. Finished tasks count
/// toward `len` until joined. Dropping the set aborts its tasks; it does not wait
/// for cancellation, and already-running blocking functions cannot be stopped.
/// Use [`Self::shutdown`] to abort and wait, or [`Self::detach_all`] to relinquish
/// ownership without aborting. No Tokio dependency is required by this type.
pub struct JoinSet<T> {
    entries: BTreeMap<Id, Entry<T>>,
    ready: Arc<Mutex<Ready>>,
}
impl<T> Default for JoinSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> JoinSet<T> {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            ready: Arc::default(),
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn insert(&mut self, handle: JoinHandle<T>) -> AbortHandle {
        let abort = handle.abort_handle();
        let id = handle.id();
        let waker = Waker::from(Arc::new(TaskWake {
            id,
            ready: Arc::downgrade(&self.ready),
        }));
        self.entries.insert(
            id,
            Entry {
                handle,
                waker: waker.clone(),
            },
        );
        // Initial poll registers the per-task completion waker. If the task
        // already finished, that poll consumes the stored result instead.
        waker.wake();
        abort
    }
    pub fn spawn<F>(&mut self, future: F) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.insert(crate::spawn(future))
    }
    pub fn spawn_blocking<F>(&mut self, function: F) -> AbortHandle
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.insert(crate::spawn_blocking(function))
    }
    /// Spawn on an explicitly selected runtime, including from a foreign thread.
    pub fn spawn_on<F>(&mut self, future: F, runtime: &Handle) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.insert(runtime.spawn(future))
    }
    pub fn spawn_blocking_on<F>(&mut self, function: F, runtime: &Handle) -> AbortHandle
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.insert(runtime.spawn_blocking(function))
    }
    /// Cancellation safe: cancelling this wait never removes a task's result.
    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
        poll_fn(|cx| self.poll_join_next(cx)).await
    }
    pub async fn join_next_with_id(&mut self) -> Option<Result<(Id, T), JoinError>> {
        poll_fn(|cx| self.poll_join_next_with_id(cx)).await
    }
    pub fn poll_join_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<T, JoinError>>> {
        self.poll_join_next_with_id(cx)
            .map(|value| value.map(|result| result.map(|(_, value)| value)))
    }
    pub fn poll_join_next_with_id(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<(Id, T), JoinError>>> {
        if self.is_empty() {
            self.ready.lock().unwrap().waiter = None;
            return Poll::Ready(None);
        }
        self.ready.lock().unwrap().waiter = Some(cx.waker().clone());
        // Bound initial registration and stale-wake processing per poll so a
        // large set does not monopolize the current-thread executor.
        for _ in 0..64 {
            let Some(id) = self.pop_ready() else {
                return Poll::Pending;
            };
            if let Some(result) = self.poll_task(id) {
                return Poll::Ready(Some(result));
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
    pub fn try_join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.try_join_next_with_id()
            .map(|result| result.map(|(_, value)| value))
    }
    pub fn try_join_next_with_id(&mut self) -> Option<Result<(Id, T), JoinError>> {
        while let Some(id) = self.pop_ready() {
            if let Some(result) = self.poll_task(id) {
                return Some(result);
            }
        }
        None
    }
    fn pop_ready(&self) -> Option<Id> {
        let mut ready = self.ready.lock().unwrap();
        let id = ready.ids.pop_front()?;
        ready.queued.remove(&id);
        Some(id)
    }
    fn poll_task(&mut self, id: Id) -> Option<Result<(Id, T), JoinError>> {
        let entry = self.entries.get_mut(&id)?;
        let result = Pin::new(&mut entry.handle).poll(&mut Context::from_waker(&entry.waker));
        match result {
            Poll::Pending => None,
            Poll::Ready(result) => {
                self.entries.remove(&id);
                Some(result.map(|value| (id, value)))
            }
        }
    }
    /// Request cancellation without removing any task or completed result.
    pub fn abort_all(&self) {
        for entry in self.entries.values() {
            entry.handle.abort();
        }
    }
    /// Keep tasks running independently. Their outputs are discarded.
    pub fn detach_all(&mut self) {
        // Disconnect old task wakers before dropping handles. Detached tasks
        // cannot retain or wake the collection's next waiter.
        self.ready = Arc::default();
        self.entries.clear();
    }
    /// Abort every task and drain all outcomes, ignoring panics and cancellation.
    pub async fn shutdown(&mut self) {
        self.abort_all();
        while self.join_next().await.is_some() {}
    }
}
impl<T> Drop for JoinSet<T> {
    fn drop(&mut self) {
        self.abort_all();
    }
}
