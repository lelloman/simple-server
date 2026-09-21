//! Application-owned, bounded scheduler. Registration is static; execution is
//! driven by `run`, not by a hidden supervisor. Observers own reporting/storage.
mod schedule;
use crate::{
    lifecycle::Shutdown,
    tasks::{ShutdownBehavior, TaskCompletion, TaskContext, TaskId, TaskSet, WorkInfo},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
pub use schedule::{CronSchedule, FirstRun, Schedule};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    future::Future,
    num::NonZeroUsize,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError(pub String);
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ConfigError {}

/// Every queue and execution limit is explicit. Resource pool names are product-owned.
#[derive(Clone, Debug)]
pub struct SchedulerLimits {
    pub max_running: NonZeroUsize,
    pub max_in_flight: NonZeroUsize,
    pub command_capacity: NonZeroUsize,
    pub pools: BTreeMap<String, NonZeroUsize>,
}
#[derive(Clone, Debug)]
pub struct JobConfig {
    pub schedules: Vec<Schedule>,
    pub events: Vec<String>,
    pub resource_pool: Option<String>,
    pub max_running: NonZeroUsize,
    pub max_pending: usize,
    pub shutdown: ShutdownBehavior,
}
impl Default for JobConfig {
    fn default() -> Self {
        Self {
            schedules: Vec::new(),
            events: Vec::new(),
            resource_pool: None,
            max_running: NonZeroUsize::new(1).unwrap(),
            max_pending: 0,
            shutdown: ShutdownBehavior::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trigger {
    Manual,
    Event(String),
    Scheduled(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RunId(pub u64);
pub struct JobContext<P> {
    pub task: TaskContext,
    pub run_id: RunId,
    pub attempt: u32,
    pub trigger: Trigger,
    pub parameters: Option<Arc<P>>,
}
type JobFuture<E> = Pin<Box<dyn Future<Output = Result<(), E>> + Send>>;
enum Factory<P, E> {
    Async(Arc<dyn Fn(JobContext<P>) -> JobFuture<E> + Send + Sync>),
    Blocking(Arc<dyn Fn(JobContext<P>) -> Result<(), E> + Send + Sync>),
}
impl<P, E> Clone for Factory<P, E> {
    fn clone(&self) -> Self {
        match self {
            Self::Async(f) => Self::Async(f.clone()),
            Self::Blocking(f) => Self::Blocking(f.clone()),
        }
    }
}
pub struct Job<P, E> {
    pub id: String,
    pub config: JobConfig,
    factory: Factory<P, E>,
}
impl<P, E> Job<P, E> {
    pub fn new<F, Fut>(id: impl Into<String>, config: JobConfig, job: F) -> Self
    where
        F: Fn(JobContext<P>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
    {
        Self {
            id: id.into(),
            config,
            factory: Factory::Async(Arc::new(move |ctx| Box::pin(job(ctx)))),
        }
    }
    pub fn blocking(
        id: impl Into<String>,
        config: JobConfig,
        job: impl Fn(JobContext<P>) -> Result<(), E> + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            config,
            factory: Factory::Blocking(Arc::new(job)),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    UnknownJob,
    Busy,
    Full,
    Paused,
    CircuitOpen,
    Closed,
    IdsExhausted,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    pub job_id: String,
    pub result: Result<RunId, Rejection>,
}
#[derive(Debug)]
pub enum Event<E> {
    Admitted {
        job_id: String,
        run_id: RunId,
        trigger: Trigger,
    },
    Started {
        job_id: String,
        run_id: RunId,
        attempt: u32,
    },
    Completed {
        job_id: String,
        run_id: RunId,
        attempt: u32,
        completion: TaskCompletion<E>,
        runtime_exceeded: bool,
        will_retry: bool,
    },
    Rejected {
        job_id: String,
        trigger: Trigger,
        reason: Rejection,
    },
    Cancelled {
        job_id: String,
        run_id: RunId,
    },
}
enum Command<P> {
    Trigger {
        job: String,
        parameters: Option<Arc<P>>,
        reply: oneshot::Sender<Admission>,
    },
    Emit {
        event: String,
        parameters: Option<Arc<P>>,
        reply: oneshot::Sender<Vec<Admission>>,
    },
    Cancel {
        run: RunId,
        reply: oneshot::Sender<bool>,
    },
}
/// Bounded command ingress. Dropping a response future does not cancel accepted work.
pub struct SchedulerHandle<P> {
    sender: mpsc::Sender<Command<P>>,
}
impl<P> Clone for SchedulerHandle<P> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
impl<P> SchedulerHandle<P> {
    pub async fn trigger(
        &self,
        job: impl Into<String>,
        parameters: Option<P>,
    ) -> Result<Admission, Rejection> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Trigger {
                job: job.into(),
                parameters: parameters.map(Arc::new),
                reply,
            })
            .await
            .map_err(|_| Rejection::Closed)?;
        response.await.map_err(|_| Rejection::Closed)
    }
    pub async fn emit(
        &self,
        event: impl Into<String>,
        parameters: Option<P>,
    ) -> Result<Vec<Admission>, Rejection> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Emit {
                event: event.into(),
                parameters: parameters.map(Arc::new),
                reply,
            })
            .await
            .map_err(|_| Rejection::Closed)?;
        response.await.map_err(|_| Rejection::Closed)
    }
    pub async fn cancel(&self, run: RunId) -> Result<bool, Rejection> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Cancel { run, reply })
            .await
            .map_err(|_| Rejection::Closed)?;
        response.await.map_err(|_| Rejection::Closed)
    }
}
struct Cursor {
    wall: Option<SystemTime>,
    monotonic: Option<Instant>,
}
struct Registered<P, E> {
    job: Job<P, E>,
    cursors: Vec<Cursor>,
}
struct Run<P> {
    id: RunId,
    job: String,
    trigger: Trigger,
    parameters: Option<Arc<P>>,
    attempt: u32,
}

/// All owned tasks remain inspectable if the `run` future is cancelled by an
/// outer lifecycle deadline. Dropping the scheduler instead abandons ownership.
pub struct Scheduler<P: Send + Sync + 'static, E: Send + 'static> {
    limits: SchedulerLimits,
    jobs: BTreeMap<String, Registered<P, E>>,
    queue: VecDeque<Run<P>>,
    running: BTreeMap<TaskId, Run<P>>,
    tasks: TaskSet<E>,
    sender: mpsc::Sender<Command<P>>,
    receiver: mpsc::Receiver<Command<P>>,
    next: u64,
    initialized: bool,
    stopping: bool,
    rng: StdRng,
}
impl<P: Send + Sync + 'static, E: Send + 'static> Scheduler<P, E> {
    pub fn new(limits: SchedulerLimits) -> Result<Self, ConfigError> {
        Self::with_seed(limits, rand::random())
    }
    /// Deterministic jitter for reproducible tests; not a security RNG contract.
    pub fn with_seed(limits: SchedulerLimits, seed: u64) -> Result<Self, ConfigError> {
        if limits.max_running > limits.max_in_flight {
            return Err(ConfigError("running limit exceeds in-flight limit".into()));
        }
        if limits.pools.keys().any(|name| name.is_empty()) {
            return Err(ConfigError("empty resource pool".into()));
        }
        let (sender, receiver) = mpsc::channel(limits.command_capacity.get());
        Ok(Self {
            tasks: TaskSet::new(limits.max_running),
            limits,
            jobs: BTreeMap::new(),
            queue: VecDeque::new(),
            running: BTreeMap::new(),
            sender,
            receiver,
            next: 0,
            initialized: false,
            stopping: false,
            rng: StdRng::seed_from_u64(seed),
        })
    }
    pub fn handle(&self) -> SchedulerHandle<P> {
        SchedulerHandle {
            sender: self.sender.clone(),
        }
    }
    pub fn register(&mut self, job: Job<P, E>) -> Result<(), ConfigError> {
        if self.initialized {
            return Err(ConfigError(
                "registration is closed after run starts".into(),
            ));
        }
        if job.id.is_empty() || self.jobs.contains_key(&job.id) {
            return Err(ConfigError("empty or duplicate job id".into()));
        }
        if let Some(pool) = &job.config.resource_pool
            && !self.limits.pools.contains_key(pool)
        {
            return Err(ConfigError(format!("unknown pool {pool}")));
        }
        for schedule in &job.config.schedules {
            schedule.validate()?;
        }
        if job
            .config
            .max_running
            .get()
            .checked_add(job.config.max_pending)
            .is_none()
        {
            return Err(ConfigError("job capacity overflows".into()));
        }
        self.jobs.insert(
            job.id.clone(),
            Registered {
                job,
                cursors: Vec::new(),
            },
        );
        Ok(())
    }
    pub fn unfinished(&self) -> Vec<WorkInfo> {
        self.tasks.unfinished()
    }
    pub fn in_flight(&self) -> usize {
        self.running.len() + self.queue.len()
    }
    pub fn abort_async(&mut self, task: TaskId) -> Result<(), crate::tasks::AbortError> {
        self.tasks.abort_async(task)
    }
    fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        let wall = SystemTime::now();
        let mono = Instant::now();
        for registered in self.jobs.values_mut() {
            registered.cursors = registered
                .job
                .config
                .schedules
                .iter()
                .map(|schedule| {
                    let next = schedule.first_after(wall, self.rng.r#gen());
                    Self::cursor(schedule, next, wall, mono)
                })
                .collect();
        }
    }
    fn cursor(
        schedule: &Schedule,
        next: Option<SystemTime>,
        wall: SystemTime,
        mono: Instant,
    ) -> Cursor {
        Cursor {
            wall: next,
            monotonic: if matches!(schedule, Schedule::Cron(_)) {
                None
            } else {
                next.and_then(|time| {
                    mono.checked_add(time.duration_since(wall).unwrap_or_default())
                })
            },
        }
    }
    fn can_start(&self, job: &str) -> bool {
        let config = &self.jobs[job].job.config;
        if self.running.len() >= self.limits.max_running.get() {
            return false;
        }
        if self.running.values().filter(|run| run.job == job).count() >= config.max_running.get() {
            return false;
        }
        if let Some(pool) = &config.resource_pool
            && self
                .running
                .values()
                .filter(|run| self.jobs[&run.job].job.config.resource_pool.as_ref() == Some(pool))
                .count()
                >= self.limits.pools[pool].get()
        {
            return false;
        }
        true
    }
    fn admit(
        &mut self,
        job: String,
        trigger: Trigger,
        parameters: Option<Arc<P>>,
        observer: &mut impl FnMut(Event<E>),
    ) -> Admission {
        let reason = if self.stopping {
            Some(Rejection::Closed)
        } else if !self.jobs.contains_key(&job) {
            Some(Rejection::UnknownJob)
        } else if self.in_flight() >= self.limits.max_in_flight.get() {
            Some(Rejection::Full)
        } else if !self.can_start(&job)
            && self.queue.iter().filter(|run| run.job == job).count()
                >= self.jobs[&job].job.config.max_pending
        {
            Some(Rejection::Busy)
        } else {
            None
        };
        if let Some(reason) = reason {
            observer(Event::Rejected {
                job_id: job.clone(),
                trigger,
                reason,
            });
            return Admission {
                job_id: job,
                result: Err(reason),
            };
        }
        let Some(next) = self.next.checked_add(1) else {
            return Admission {
                job_id: job,
                result: Err(Rejection::IdsExhausted),
            };
        };
        let id = RunId(self.next);
        self.next = next;
        observer(Event::Admitted {
            job_id: job.clone(),
            run_id: id,
            trigger: trigger.clone(),
        });
        self.queue.push_back(Run {
            id,
            job: job.clone(),
            trigger,
            parameters,
            attempt: 1,
        });
        self.dispatch(observer);
        Admission {
            job_id: job,
            result: Ok(id),
        }
    }
    fn dispatch(&mut self, observer: &mut impl FnMut(Event<E>)) {
        while let Some(index) = self.queue.iter().position(|run| self.can_start(&run.job)) {
            let run = self.queue.remove(index).unwrap();
            let registered = &self.jobs[&run.job];
            let factory = registered.job.factory.clone();
            let parameters = run.parameters.clone();
            let trigger = run.trigger.clone();
            let id = run.id;
            let attempt = run.attempt;
            let context = move |task| JobContext {
                task,
                run_id: id,
                attempt,
                trigger,
                parameters,
            };
            let task = match factory {
                Factory::Async(job) => {
                    self.tasks
                        .spawn(&run.job, registered.job.config.shutdown, move |task| {
                            job(context(task))
                        })
                }
                Factory::Blocking(job) => self.tasks.spawn_blocking(
                    &run.job,
                    registered.job.config.shutdown,
                    move |task| job(context(task)),
                ),
            }
            .expect("scheduler owns runtime and enforces task capacity");
            observer(Event::Started {
                job_id: run.job.clone(),
                run_id: run.id,
                attempt: run.attempt,
            });
            self.running.insert(task, run);
        }
    }
    fn advance_delay(&mut self, run: &Run<P>) {
        if let Trigger::Scheduled(index) = run.trigger {
            let registered = self.jobs.get_mut(&run.job).unwrap();
            let schedule = &registered.job.config.schedules[index];
            if matches!(schedule, Schedule::FixedDelay { .. }) {
                let now = SystemTime::now();
                let next = schedule.after_completion(now, self.rng.r#gen());
                registered.cursors[index] = Self::cursor(schedule, next, now, Instant::now());
            }
        }
    }
    fn completed(&mut self, completion: TaskCompletion<E>, observer: &mut impl FnMut(Event<E>)) {
        let run = self
            .running
            .remove(&completion.task.id)
            .expect("owned scheduler task");
        self.advance_delay(&run);
        observer(Event::Completed {
            job_id: run.job,
            run_id: run.id,
            attempt: run.attempt,
            completion,
            runtime_exceeded: false,
            will_retry: false,
        });
    }
    fn due(&mut self, observer: &mut impl FnMut(Event<E>)) {
        let wall = SystemTime::now();
        let mono = Instant::now();
        let mut due = Vec::new();
        for (id, registered) in &mut self.jobs {
            for (index, (schedule, cursor)) in registered
                .job
                .config
                .schedules
                .iter()
                .zip(&mut registered.cursors)
                .enumerate()
            {
                let ready = if matches!(schedule, Schedule::Cron(_)) {
                    cursor.wall.is_some_and(|t| t <= wall)
                } else {
                    cursor.monotonic.is_some_and(|t| t <= mono)
                };
                if ready {
                    let next = if matches!(schedule, Schedule::FixedRate { .. }) {
                        // Derive the wall sample from monotonic elapsed time, not wall-clock changes.
                        let elapsed = mono.saturating_duration_since(cursor.monotonic.unwrap());
                        let previous = cursor.wall.unwrap();
                        schedule.after_tick(previous, previous.checked_add(elapsed).unwrap_or(wall))
                    } else {
                        schedule.after_tick(cursor.wall.unwrap(), wall)
                    };
                    let mapped_wall = if matches!(schedule, Schedule::FixedRate { .. }) {
                        let previous = cursor.wall.unwrap();
                        let elapsed = mono.saturating_duration_since(cursor.monotonic.unwrap());
                        next.and_then(|next| next.duration_since(previous).ok())
                            .and_then(|distance| wall.checked_add(distance.saturating_sub(elapsed)))
                    } else {
                        next
                    };
                    *cursor = Self::cursor(schedule, mapped_wall, wall, mono);
                    due.push((id.clone(), index));
                }
            }
        }
        for (id, index) in due {
            let admission = self.admit(id.clone(), Trigger::Scheduled(index), None, observer);
            if admission.result.is_err() {
                let run = Run {
                    id: RunId(0),
                    job: id,
                    trigger: Trigger::Scheduled(index),
                    parameters: None,
                    attempt: 0,
                };
                self.advance_delay(&run);
            }
        }
    }
    fn next_wake(&self) -> Instant {
        let now = Instant::now();
        let wall = SystemTime::now();
        // Recheck UTC schedules after clock adjustments, without tying interval clocks to UTC.
        let mut next = now + Duration::from_secs(1);
        for registered in self.jobs.values() {
            for cursor in &registered.cursors {
                let at = cursor.monotonic.or_else(|| {
                    cursor
                        .wall
                        .and_then(|t| now.checked_add(t.duration_since(wall).unwrap_or_default()))
                });
                if let Some(at) = at {
                    next = next.min(at);
                }
            }
        }
        next
    }
    fn cancel(&mut self, id: RunId, observer: &mut impl FnMut(Event<E>)) -> bool {
        if let Some(index) = self.queue.iter().position(|run| run.id == id) {
            let run = self.queue.remove(index).unwrap();
            self.advance_delay(&run);
            observer(Event::Cancelled {
                job_id: run.job,
                run_id: id,
            });
            return true;
        }
        if let Some((&task, _)) = self.running.iter().find(|(_, run)| run.id == id) {
            return self
                .tasks
                .request_cancel(task, crate::tasks::CancellationReason::Explicit);
        }
        false
    }
    fn stop(&mut self, observer: &mut impl FnMut(Event<E>)) {
        self.stopping = true;
        self.receiver.close();
        self.tasks.request_shutdown();
        while let Some(run) = self.queue.pop_front() {
            observer(Event::Cancelled {
                job_id: run.job,
                run_id: run.id,
            });
        }
        // Discard commands already buffered. Dropping acknowledgements returns Closed.
        while self.receiver.try_recv().is_ok() {}
    }
    /// The observer is synchronous, called outside locks, and must not panic or block.
    /// Per-job failures are events; the application decides whether to request shutdown.
    pub async fn run(
        &mut self,
        shutdown: Shutdown,
        mut observer: impl FnMut(Event<E>),
    ) -> Result<(), ConfigError> {
        self.initialize();
        loop {
            if shutdown.is_requested() && !self.stopping {
                self.stop(&mut observer);
            }
            if self.stopping {
                if let Some(completion) = self.tasks.join_next().await {
                    self.completed(completion, &mut observer);
                    continue;
                }
                return Ok(());
            }
            self.due(&mut observer);
            self.dispatch(&mut observer);
            let wake = self.next_wake();
            tokio::select! {
                _ = shutdown.requested() => self.stop(&mut observer),
                completion = self.tasks.join_next(), if !self.tasks.is_empty() => { if let Some(completion) = completion { self.completed(completion, &mut observer); } },
                command = self.receiver.recv() => { match command {
                    Some(Command::Trigger { job, parameters, reply }) => { let result = self.admit(job, Trigger::Manual, parameters, &mut observer); let _ = reply.send(result); }
                    Some(Command::Emit { event, parameters, reply }) => {
                        let ids: Vec<_> = self.jobs.iter().filter(|(_, r)| r.job.config.events.contains(&event)).map(|(id, _)| id.clone()).collect();
                        let results = ids.into_iter().map(|job| self.admit(job, Trigger::Event(event.clone()), parameters.clone(), &mut observer)).collect(); let _ = reply.send(results);
                    }
                    Some(Command::Cancel { run, reply }) => { let cancelled = self.cancel(run, &mut observer); let _ = reply.send(cancelled); }
                    None => self.stop(&mut observer),
                } },
                _ = tokio::time::sleep_until(wake) => {},
            }
        }
    }
}
