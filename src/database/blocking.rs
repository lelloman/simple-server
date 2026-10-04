//! Dedicated bounded worker threads for synchronous operations. Queue capacity,
//! weighted priorities and lane limits are explicit, with no database defaults.
//! Caller timeouts never release capacity held by an operation still executing.
use std::{
    collections::VecDeque,
    fmt,
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::sync::Notify;

#[derive(Clone, Debug)]
pub struct Priority {
    pub weight: NonZeroUsize,
    pub queue_capacity: NonZeroUsize,
}
#[derive(Clone, Debug)]
pub struct Config {
    pub workers: NonZeroUsize,
    pub priorities: Vec<Priority>,
    /// Application maps its lane identities to these indices.
    pub lane_limits: Vec<NonZeroUsize>,
}
#[derive(Debug)]
pub enum BuildError {
    Invalid(&'static str),
    Spawn(std::io::Error),
}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blocking executor configuration: {self:?}")
    }
}
impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(e) => Some(e),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub priority: usize,
    pub lane: usize,
    /// Includes time waiting behind a busy lane or worker. None is unlimited.
    pub queue_timeout: Option<Duration>,
    /// Starts at dispatch, not admission. Does not interrupt native work.
    pub execution_timeout: Option<Duration>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    Closed,
    QueueFull,
    UnknownPriority,
    UnknownLane,
    InvalidDeadline,
}
impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blocking submission: {self:?}")
    }
}
impl std::error::Error for SubmitError {}
#[derive(Debug, PartialEq, Eq)]
pub enum RunError<E> {
    QueueTimeout,
    /// Operation may still be running; no rollback or retry is implied.
    ExecutionTimeout,
    Cancelled,
    Panicked(String),
    Operation(E),
}
impl<E: fmt::Debug> fmt::Display for RunError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blocking operation: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for RunError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(e) => Some(e),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseMode {
    Drain,
    CancelQueued,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub closed: bool,
    pub active: usize,
    pub queued: Vec<usize>,
    pub active_by_lane: Vec<usize>,
}

#[derive(Clone, Copy)]
enum Phase {
    Queued,
    Running(Option<Instant>),
    Complete,
    Cancelled,
    Expired,
}
struct Control {
    phase: Mutex<Phase>,
    changed: Condvar,
    notify: Notify,
}
impl Control {
    fn new() -> Self {
        Self {
            phase: Mutex::new(Phase::Queued),
            changed: Condvar::new(),
            notify: Notify::new(),
        }
    }
    fn wake(&self) {
        self.changed.notify_all();
        self.notify.notify_waiters();
    }
}
struct Cell<T, E> {
    control: Arc<Control>,
    result: Mutex<Option<Result<T, RunError<E>>>>,
}
struct Job {
    lane: usize,
    deadline: Option<Instant>,
    execution_timeout: Option<Duration>,
    control: Arc<Control>,
    run: Box<dyn FnOnce() + Send>,
}
// Application-owned closure captures can reenter the scheduler or panic in
// Drop. Retire outside scheduler locks and isolate each destructor separately.
struct Retired(Vec<Job>);
impl Retired {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl Drop for Retired {
    fn drop(&mut self) {
        for job in self.0.drain(..) {
            let _ = catch_unwind(AssertUnwindSafe(|| drop(job)));
        }
    }
}
struct State {
    queues: Vec<VecDeque<Job>>,
    lanes: Vec<usize>,
    active: usize,
    cursor: usize,
    closed: bool,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    notify: Notify,
    config: Config,
    schedule: Vec<usize>,
}
impl Shared {
    fn wake(&self) {
        self.changed.notify_all();
        self.notify.notify_waiters();
    }
    fn prune(&self, state: &mut State) -> Retired {
        let now = Instant::now();
        let mut retired = Vec::new();
        for queue in &mut state.queues {
            let mut kept = VecDeque::new();
            while let Some(job) = queue.pop_front() {
                let mut phase = job.control.phase.lock().expect("job state poisoned");
                if matches!(*phase, Phase::Queued) && job.deadline.is_some_and(|d| d <= now) {
                    *phase = Phase::Expired;
                    job.control.wake();
                }
                let keep = matches!(*phase, Phase::Queued);
                drop(phase);
                if keep {
                    kept.push_back(job);
                } else {
                    retired.push(job);
                }
            }
            *queue = kept;
        }
        if !retired.is_empty() {
            self.wake();
        }
        Retired(retired)
    }
    fn close(&self, mode: CloseMode) {
        let mut state = self.state.lock().expect("executor state poisoned");
        state.closed = true;
        let mut retired = Vec::new();
        if mode == CloseMode::CancelQueued {
            for queue in &mut state.queues {
                for job in queue.drain(..) {
                    *job.control.phase.lock().expect("job state poisoned") = Phase::Cancelled;
                    job.control.wake();
                    retired.push(job);
                }
            }
        }
        self.wake();
        drop(state);
        drop(Retired(retired));
    }
}
struct Owner {
    shared: Arc<Shared>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.shared.close(CloseMode::CancelQueued);
    }
}

/// Clones share queues and workers. Dropping the last clone closes admission and
/// cancels queued work; running threads finish their current operations. No join
/// is performed in Drop. Explicit close + drain reports actual completion.
#[derive(Clone)]
pub struct Executor {
    owner: Arc<Owner>,
}
impl Executor {
    pub fn new(config: Config) -> Result<Self, BuildError> {
        if config.priorities.is_empty() || config.lane_limits.is_empty() {
            return Err(BuildError::Invalid(
                "at least one priority and lane required",
            ));
        }
        let weight = config
            .priorities
            .iter()
            .try_fold(0usize, |sum, p| sum.checked_add(p.weight.get()));
        if weight.is_none_or(|sum| sum > 65_536) {
            return Err(BuildError::Invalid("total priority weight exceeds 65536"));
        }
        let schedule = config
            .priorities
            .iter()
            .enumerate()
            .flat_map(|(i, p)| std::iter::repeat_n(i, p.weight.get()))
            .collect();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                queues: (0..config.priorities.len())
                    .map(|_| VecDeque::new())
                    .collect(),
                lanes: vec![0; config.lane_limits.len()],
                active: 0,
                cursor: 0,
                closed: false,
            }),
            changed: Condvar::new(),
            notify: Notify::new(),
            config,
            schedule,
        });
        for index in 0..shared.config.workers.get() {
            let worker = shared.clone();
            if let Err(error) = thread::Builder::new()
                .name(format!("simple-server-db-{index}"))
                .spawn(move || work(worker))
            {
                shared.close(CloseMode::CancelQueued);
                return Err(BuildError::Spawn(error));
            }
        }
        Ok(Self {
            owner: Arc::new(Owner { shared }),
        })
    }
    /// Immediate, bounded admission. A full queue is rejected without retaining
    /// the closure. Queue deadlines begin here; there is no automatic retry.
    pub fn submit<T: Send + 'static, E: Send + 'static>(
        &self,
        options: Options,
        operation: impl FnOnce() -> Result<T, E> + Send + 'static,
    ) -> Result<Ticket<T, E>, SubmitError> {
        let shared = &self.owner.shared;
        if options.priority >= shared.config.priorities.len() {
            return Err(SubmitError::UnknownPriority);
        }
        if options.lane >= shared.config.lane_limits.len() {
            return Err(SubmitError::UnknownLane);
        }
        let now = Instant::now();
        let deadline = match options.queue_timeout {
            Some(duration) => Some(
                now.checked_add(duration)
                    .ok_or(SubmitError::InvalidDeadline)?,
            ),
            None => None,
        };
        if options
            .execution_timeout
            .is_some_and(|d| now.checked_add(d).is_none())
        {
            return Err(SubmitError::InvalidDeadline);
        }
        let cell = Arc::new(Cell {
            control: Arc::new(Control::new()),
            result: Mutex::new(None),
        });
        let result = cell.clone();
        let run = Box::new(move || {
            let outcome = match catch_unwind(AssertUnwindSafe(operation)) {
                Ok(value) => value.map_err(RunError::Operation),
                Err(payload) => {
                    let message = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "non-string panic".to_owned());
                    Err(RunError::Panicked(message))
                }
            };
            let expired = matches!(*result.control.phase.lock().expect("job state poisoned"), Phase::Running(Some(deadline)) if deadline <= Instant::now());
            let outcome = if expired {
                Err(RunError::ExecutionTimeout)
            } else {
                outcome
            };
            *result.result.lock().expect("job result poisoned") = Some(outcome);
            *result.control.phase.lock().expect("job state poisoned") = Phase::Complete;
            result.control.wake();
        });
        let retired;
        let mut state = shared.state.lock().expect("executor state poisoned");
        retired = shared.prune(&mut state);
        let _ = &retired;
        if state.closed {
            return Err(SubmitError::Closed);
        }
        if state.queues[options.priority].len()
            >= shared.config.priorities[options.priority]
                .queue_capacity
                .get()
        {
            return Err(SubmitError::QueueFull);
        }
        state.queues[options.priority].push_back(Job {
            lane: options.lane,
            deadline,
            execution_timeout: options.execution_timeout,
            control: cell.control.clone(),
            run,
        });
        shared.wake();
        Ok(Ticket {
            cell,
            shared: shared.clone(),
            queue_deadline: deadline,
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        let shared = &self.owner.shared;
        let retired;
        let mut state = shared.state.lock().expect("executor state poisoned");
        retired = shared.prune(&mut state);
        let _ = &retired;
        Snapshot {
            closed: state.closed,
            active: state.active,
            queued: state.queues.iter().map(VecDeque::len).collect(),
            active_by_lane: state.lanes.clone(),
        }
    }
    pub fn close(&self, mode: CloseMode) {
        self.owner.shared.close(mode);
    }
    /// Wait for queues and actual synchronous operations to empty. Call close
    /// first to prevent new submissions racing with a successful drain.
    pub async fn drain(&self) {
        loop {
            let notified = self.owner.shared.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let snapshot = self.snapshot();
            if snapshot.active == 0 && snapshot.queued.iter().all(|n| *n == 0) {
                return;
            }
            notified.await;
        }
    }
    /// Same scheduler and completion boundary for synchronous callers. A false
    /// return means work still exists, not that running work has been cancelled.
    pub fn drain_blocking(&self, timeout: Duration) -> bool {
        let Some(deadline) = Instant::now().checked_add(timeout) else {
            return false;
        };
        let shared = &self.owner.shared;
        let mut state = shared.state.lock().expect("executor state poisoned");
        loop {
            let retired = shared.prune(&mut state);
            if !retired.is_empty() {
                drop(state);
                drop(retired);
                state = shared.state.lock().expect("executor state poisoned");
                continue;
            }
            if state.active == 0 && state.queues.iter().all(VecDeque::is_empty) {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            state = shared
                .changed
                .wait_timeout(state, deadline - now)
                .expect("executor state poisoned")
                .0;
        }
    }
}

fn work(shared: Arc<Shared>) {
    loop {
        let job = {
            let mut state = shared.state.lock().expect("executor state poisoned");
            'dispatch: loop {
                let retired = shared.prune(&mut state);
                if !retired.is_empty() {
                    drop(state);
                    drop(retired);
                    state = shared.state.lock().expect("executor state poisoned");
                    continue;
                }
                let mut selected = None;
                for offset in 0..shared.schedule.len() {
                    let cursor = (state.cursor + offset) % shared.schedule.len();
                    let class = shared.schedule[cursor];
                    if let Some(index) = state.queues[class]
                        .iter()
                        .position(|j| state.lanes[j.lane] < shared.config.lane_limits[j.lane].get())
                    {
                        let job = state.queues[class]
                            .remove(index)
                            .expect("selected job exists");
                        let mut phase = job.control.phase.lock().expect("job state poisoned");
                        if !matches!(*phase, Phase::Queued) {
                            drop(phase);
                            drop(state);
                            drop(Retired(vec![job]));
                            state = shared.state.lock().expect("executor state poisoned");
                            continue 'dispatch;
                        }
                        if job.deadline.is_some_and(|d| d <= Instant::now()) {
                            *phase = Phase::Expired;
                            job.control.wake();
                            drop(phase);
                            drop(state);
                            drop(Retired(vec![job]));
                            state = shared.state.lock().expect("executor state poisoned");
                            continue 'dispatch;
                        }
                        *phase = Phase::Running(
                            job.execution_timeout
                                .and_then(|d| Instant::now().checked_add(d)),
                        );
                        job.control.wake();
                        drop(phase);
                        state.lanes[job.lane] += 1;
                        state.active += 1;
                        state.cursor = (cursor + 1) % shared.schedule.len();
                        selected = Some(job);
                        break;
                    }
                }
                if let Some(job) = selected {
                    break job;
                }
                if state.closed && state.queues.iter().all(VecDeque::is_empty) {
                    return;
                }
                let next = state
                    .queues
                    .iter()
                    .flatten()
                    .filter_map(|job| job.deadline)
                    .min();
                state = match next {
                    Some(deadline) => {
                        shared
                            .changed
                            .wait_timeout(state, deadline.saturating_duration_since(Instant::now()))
                            .expect("executor state poisoned")
                            .0
                    }
                    None => shared.changed.wait(state).expect("executor state poisoned"),
                };
            }
        };
        let lane = job.lane;
        // Also isolate panics in abandoned result/capture destructors so the
        // worker and lane are always released.
        let _ = catch_unwind(AssertUnwindSafe(job.run));
        let mut state = shared.state.lock().expect("executor state poisoned");
        state.lanes[lane] -= 1;
        state.active -= 1;
        shared.wake();
    }
}

/// Dropping a ticket cancels it if still queued. Once dispatched it only abandons
/// the result. Native execution and lane/worker occupancy continue to completion.
pub struct Ticket<T, E> {
    cell: Arc<Cell<T, E>>,
    shared: Arc<Shared>,
    queue_deadline: Option<Instant>,
}
enum Poll<T, E> {
    Ready(Result<T, RunError<E>>),
    Wait(Option<Instant>),
}
impl<T, E> Ticket<T, E> {
    fn inspect(&self, phase: &mut Phase) -> Poll<T, E> {
        match *phase {
            Phase::Complete => Poll::Ready(
                self.cell
                    .result
                    .lock()
                    .expect("job result poisoned")
                    .take()
                    .expect("completed result exists"),
            ),
            Phase::Cancelled => Poll::Ready(Err(RunError::Cancelled)),
            Phase::Expired => Poll::Ready(Err(RunError::QueueTimeout)),
            Phase::Queued if self.queue_deadline.is_some_and(|d| d <= Instant::now()) => {
                *phase = Phase::Expired;
                self.shared.wake();
                Poll::Ready(Err(RunError::QueueTimeout))
            }
            Phase::Running(Some(deadline)) if deadline <= Instant::now() => {
                Poll::Ready(Err(RunError::ExecutionTimeout))
            }
            Phase::Running(deadline) => Poll::Wait(deadline),
            Phase::Queued => Poll::Wait(self.queue_deadline),
        }
    }
    pub async fn wait(self) -> Result<T, RunError<E>> {
        loop {
            let notified = self.cell.control.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let poll =
                self.inspect(&mut self.cell.control.phase.lock().expect("job state poisoned"));
            match poll {
                Poll::Ready(result) => return result,
                Poll::Wait(Some(deadline)) => {
                    tokio::select! { _ = notified => {}, _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {} }
                }
                Poll::Wait(None) => notified.await,
            }
        }
    }
    /// Do not call on a Tokio runtime worker; use wait(). No runtime is needed
    /// when used from an ordinary synchronous thread.
    pub fn wait_blocking(self) -> Result<T, RunError<E>> {
        let mut phase = self.cell.control.phase.lock().expect("job state poisoned");
        loop {
            match self.inspect(&mut phase) {
                Poll::Ready(result) => return result,
                Poll::Wait(Some(deadline)) => {
                    phase = self
                        .cell
                        .control
                        .changed
                        .wait_timeout(phase, deadline.saturating_duration_since(Instant::now()))
                        .expect("job state poisoned")
                        .0
                }
                Poll::Wait(None) => {
                    phase = self
                        .cell
                        .control
                        .changed
                        .wait(phase)
                        .expect("job state poisoned")
                }
            }
        }
    }
}
impl<T, E> Drop for Ticket<T, E> {
    fn drop(&mut self) {
        let mut phase = self.cell.control.phase.lock().expect("job state poisoned");
        if matches!(*phase, Phase::Queued) {
            *phase = Phase::Cancelled;
            self.shared.wake();
        }
    }
}
