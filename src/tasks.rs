//! Explicit ownership of external work and spawned jobs.
//!
//! Cancellation is cooperative. A deadline bounds waiting, not execution. Dropping
//! a task set requests shutdown and detaches unfinished work; only draining proves
//! completion. Neither task handles nor HTTP framework types escape this module.

use crate::lifecycle::Shutdown;
use std::{
    collections::BTreeMap,
    fmt,
    future::Future,
    num::NonZeroUsize,
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};
use tokio::{
    sync::watch,
    task::{AbortHandle, Id, JoinSet},
};

/// Identity within one tracker or task set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkInfo {
    pub id: TaskId,
    pub name: String,
}

#[derive(Debug, Default)]
struct Tracking {
    closed: bool,
    next: u64,
    active: BTreeMap<TaskId, String>,
}
#[derive(Debug)]
struct TrackerInner {
    state: Mutex<Tracking>,
    changed: watch::Sender<()>,
}

/// Tracks scopes owned by other systems, including callbacks not yet started.
/// Guards report completion of a scope, not its success.
#[derive(Clone, Debug)]
pub struct WorkTracker(Arc<TrackerInner>);
impl Default for WorkTracker {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkTracker {
    pub fn new() -> Self {
        Self(Arc::new(TrackerInner {
            state: Mutex::new(Tracking::default()),
            changed: watch::channel(()).0,
        }))
    }
    pub fn try_acquire(&self, name: impl Into<String>) -> Result<WorkGuard, AdmissionError> {
        let name = name.into();
        let mut state = self.0.state.lock().unwrap();
        if state.closed {
            return Err(AdmissionError::Closed);
        }
        let id = TaskId(state.next);
        state.next = state
            .next
            .checked_add(1)
            .ok_or(AdmissionError::IdsExhausted)?;
        state.active.insert(id, name);
        Ok(WorkGuard {
            tracker: self.clone(),
            id,
        })
    }
    /// Irreversible; serialized with acquisition.
    pub fn close(&self) {
        self.0.state.lock().unwrap().closed = true;
        self.0.changed.send_replace(());
    }
    pub fn is_closed(&self) -> bool {
        self.0.state.lock().unwrap().closed
    }
    pub fn unfinished(&self) -> Vec<WorkInfo> {
        self.0
            .state
            .lock()
            .unwrap()
            .active
            .iter()
            .map(|(&id, name)| WorkInfo {
                id,
                name: name.clone(),
            })
            .collect()
    }
    /// Does not finish merely because an open tracker is temporarily empty.
    pub async fn wait(&self) {
        let mut changed = self.0.changed.subscribe();
        loop {
            {
                let state = self.0.state.lock().unwrap();
                if state.closed && state.active.is_empty() {
                    return;
                }
            }
            let _ = changed.changed().await;
        }
    }
}

/// Non-cloneable reservation. Acquire before handing work to an external executor.
#[must_use]
#[derive(Debug)]
pub struct WorkGuard {
    tracker: WorkTracker,
    id: TaskId,
}
impl Drop for WorkGuard {
    fn drop(&mut self) {
        self.tracker.0.state.lock().unwrap().active.remove(&self.id);
        self.tracker.0.changed.send_replace(());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    Closed,
    Full,
    NoRuntime,
    IdsExhausted,
}
impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task admission rejected: {self:?}")
    }
}
impl std::error::Error for AdmissionError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShutdownBehavior {
    #[default]
    CancelOnShutdown,
    FinishOnShutdown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancellationReason {
    Explicit,
    Shutdown,
    RuntimeBudget,
}

#[derive(Debug, Default)]
struct Timing {
    started: OnceLock<Instant>,
    finished: OnceLock<Instant>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct TaskTiming {
    pub started: Option<Instant>,
    pub finished: Option<Instant>,
}
struct FinishTiming(Arc<Timing>);
impl Drop for FinishTiming {
    fn drop(&mut self) {
        let _ = self.0.finished.set(tokio::time::Instant::now().into_std());
    }
}

#[derive(Clone, Debug)]
pub struct TaskContext {
    pub id: TaskId,
    pub name: String,
    cancellation: Shutdown,
    timing: Arc<Timing>,
}
impl TaskContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_requested()
    }
    pub async fn cancelled(&self) {
        self.cancellation.requested().await;
    }
}

/// The actual execution result, independent of whether cancellation was requested.
#[derive(Debug)]
pub enum TaskExit<E> {
    Finished(Result<(), E>),
    Panicked(String),
    Aborted,
}
#[derive(Debug)]
pub struct TaskCompletion<E> {
    pub timing: TaskTiming,
    pub task: WorkInfo,
    pub exit: TaskExit<E>,
    pub cancellation: Option<CancellationReason>,
}
#[derive(Debug)]
pub struct DrainReport<E> {
    pub completed: Vec<TaskCompletion<E>>,
    pub unfinished: Vec<WorkInfo>,
}
impl<E> DrainReport<E> {
    pub fn is_drained(&self) -> bool {
        self.unfinished.is_empty()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortError {
    UnknownTask,
    BlockingTask,
}

impl fmt::Display for AbortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot abort task: {self:?}")
    }
}
impl std::error::Error for AbortError {}

struct Entry {
    timing: Arc<Timing>,
    task: WorkInfo,
    cancellation: Shutdown,
    reason: Option<CancellationReason>,
    behavior: ShutdownBehavior,
    blocking: bool,
    abort: AbortHandle,
}

/// Bounded task ownership. Finished outcomes occupy slots until consumed.
///
/// Jobs execute on the caller's Tokio runtime. No background supervisor is created.
#[must_use = "drain the set explicitly; dropping it detaches unfinished execution"]
pub struct TaskSet<E: Send + 'static> {
    tasks: JoinSet<Result<(), E>>,
    entries: BTreeMap<Id, Entry>,
    completed: Vec<TaskCompletion<E>>,
    next: u64,
    limit: NonZeroUsize,
    closed: bool,
}
impl<E: Send + 'static> TaskSet<E> {
    pub fn new(max_tracked: NonZeroUsize) -> Self {
        Self {
            tasks: JoinSet::new(),
            entries: BTreeMap::new(),
            completed: Vec::new(),
            next: 0,
            limit: max_tracked,
            closed: false,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len() + self.completed.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn close(&mut self) {
        self.closed = true;
    }
    fn prepare(&mut self, name: String) -> Result<TaskContext, AdmissionError> {
        if self.closed {
            return Err(AdmissionError::Closed);
        }
        if self.len() >= self.limit.get() {
            return Err(AdmissionError::Full);
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(AdmissionError::NoRuntime);
        }
        let id = TaskId(self.next);
        self.next = self
            .next
            .checked_add(1)
            .ok_or(AdmissionError::IdsExhausted)?;
        Ok(TaskContext {
            id,
            name,
            cancellation: Shutdown::new(),
            timing: Arc::new(Timing::default()),
        })
    }
    fn register(
        &mut self,
        context: TaskContext,
        behavior: ShutdownBehavior,
        blocking: bool,
        abort: AbortHandle,
    ) -> TaskId {
        let id = context.id;
        self.entries.insert(
            abort.id(),
            Entry {
                timing: context.timing,
                task: WorkInfo {
                    id,
                    name: context.name,
                },
                cancellation: context.cancellation,
                reason: None,
                behavior,
                blocking,
                abort,
            },
        );
        id
    }
    pub fn spawn<F, Fut>(
        &mut self,
        name: impl Into<String>,
        behavior: ShutdownBehavior,
        job: F,
    ) -> Result<TaskId, AdmissionError>
    where
        F: FnOnce(TaskContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
    {
        let context = self.prepare(name.into())?;
        let supplied = context.clone();
        let abort = self.tasks.spawn(async move {
            let _finish = FinishTiming(supplied.timing.clone());
            let _ = supplied
                .timing
                .started
                .set(tokio::time::Instant::now().into_std());
            job(supplied).await
        });
        Ok(self.register(context, behavior, false, abort))
    }
    pub fn spawn_blocking<F>(
        &mut self,
        name: impl Into<String>,
        behavior: ShutdownBehavior,
        job: F,
    ) -> Result<TaskId, AdmissionError>
    where
        F: FnOnce(TaskContext) -> Result<(), E> + Send + 'static,
    {
        let context = self.prepare(name.into())?;
        let supplied = context.clone();
        let abort = self.tasks.spawn_blocking(move || {
            let _finish = FinishTiming(supplied.timing.clone());
            let _ = supplied
                .timing
                .started
                .set(tokio::time::Instant::now().into_std());
            job(supplied)
        });
        Ok(self.register(context, behavior, true, abort))
    }
    pub fn request_cancel(&mut self, id: TaskId, reason: CancellationReason) -> bool {
        if let Some(entry) = self.entries.values_mut().find(|entry| entry.task.id == id) {
            entry.reason.get_or_insert(reason);
            entry.cancellation.request();
            true
        } else {
            false
        }
    }
    pub fn request_shutdown(&mut self) {
        self.close();
        for entry in self.entries.values_mut() {
            if entry.behavior == ShutdownBehavior::CancelOnShutdown {
                entry.reason.get_or_insert(CancellationReason::Shutdown);
                entry.cancellation.request();
            }
        }
    }
    pub fn abort_async(&mut self, id: TaskId) -> Result<(), AbortError> {
        let entry = self
            .entries
            .values()
            .find(|entry| entry.task.id == id)
            .ok_or(AbortError::UnknownTask)?;
        if entry.blocking {
            return Err(AbortError::BlockingTask);
        }
        entry.abort.abort();
        Ok(())
    }
    /// Actual execution boundaries, independent of when the owner observes completion.
    pub fn timing(&self, id: TaskId) -> Option<TaskTiming> {
        self.entries
            .values()
            .find(|entry| entry.task.id == id)
            .map(|entry| TaskTiming {
                started: entry.timing.started.get().copied(),
                finished: entry.timing.finished.get().copied(),
            })
    }
    pub fn unfinished(&self) -> Vec<WorkInfo> {
        self.entries
            .values()
            .map(|entry| entry.task.clone())
            .collect()
    }
    async fn join_raw(&mut self) -> Option<TaskCompletion<E>> {
        let joined = self.tasks.join_next_with_id().await?;
        let (id, exit) = match joined {
            Ok((id, result)) => (id, TaskExit::Finished(result)),
            Err(error) => {
                let id = error.id();
                let exit = if error.is_cancelled() {
                    TaskExit::Aborted
                } else {
                    let panic = error.into_panic();
                    let message = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "non-string panic payload".into());
                    TaskExit::Panicked(message)
                };
                (id, exit)
            }
        };
        let entry = self
            .entries
            .remove(&id)
            .expect("joined task has ownership metadata");
        Some(TaskCompletion {
            timing: TaskTiming {
                started: entry.timing.started.get().copied(),
                finished: entry.timing.finished.get().copied(),
            },
            task: entry.task,
            exit,
            cancellation: entry.reason,
        })
    }
    /// Cancellation safe: an outcome is removed only when this future returns it.
    pub async fn join_next(&mut self) -> Option<TaskCompletion<E>> {
        if !self.completed.is_empty() {
            return Some(self.completed.remove(0));
        }
        self.join_raw().await
    }
    /// Close admission and wait without implicitly requesting cancellation.
    /// A timed-out set retains ownership; call again to continue draining.
    pub async fn drain_until(&mut self, deadline: Instant) -> DrainReport<E> {
        self.close();
        while !self.entries.is_empty() {
            tokio::select! { biased;
                completion = self.join_raw() => { if let Some(completion) = completion { self.completed.push(completion); } }
                _ = tokio::time::sleep_until(deadline.into()) => break,
            }
        }
        DrainReport {
            completed: std::mem::take(&mut self.completed),
            unfinished: self.unfinished(),
        }
    }
}
impl<E: Send + 'static> Drop for TaskSet<E> {
    fn drop(&mut self) {
        self.request_shutdown();
        self.tasks.detach_all();
    }
}
