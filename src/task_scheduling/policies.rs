use super::*;
use crate::{
    task_policies::{CircuitOutcome, CircuitSnapshot},
    tasks::{CancellationReason, TaskExit},
};

/// Control state only. No payloads, queues, active claims or retry attempts.
/// Restore against the same job/schedule configuration before calling `run`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchedulerSnapshot {
    pub schedules: BTreeMap<String, Vec<Option<SystemTime>>>,
    pub pause: PauseState,
    pub circuits: BTreeMap<String, CircuitSnapshot>,
}
impl<P, E> Job<P, E> {
    pub fn with_policy(mut self, policy: ExecutionPolicy<E>) -> Self {
        self.policy = policy;
        self
    }
}
impl<P> SchedulerHandle<P> {
    pub async fn set_pause(
        &self,
        scope: PauseScope,
        paused: bool,
        cancel_running: bool,
    ) -> Result<Result<(), ConfigError>, Rejection> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Pause {
                scope,
                paused,
                cancel_running,
                reply,
            })
            .await
            .map_err(|_| Rejection::Closed)?;
        response.await.map_err(|_| Rejection::Closed)
    }
    pub async fn snapshot(&self) -> Result<SchedulerSnapshot, Rejection> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Snapshot { reply })
            .await
            .map_err(|_| Rejection::Closed)?;
        response.await.map_err(|_| Rejection::Closed)
    }
}
impl<P: Send + Sync + 'static, E: Send + 'static> Scheduler<P, E> {
    pub fn snapshot(&self) -> SchedulerSnapshot {
        if let Some(restored) = &self.restored {
            return restored.clone();
        }
        let wall = SystemTime::now();
        let mono = Instant::now();
        let schedules = self
            .jobs
            .iter()
            .map(|(id, registered)| {
                let cursors = if !self.initialized {
                    registered
                        .job
                        .config
                        .schedules
                        .iter()
                        .map(|schedule| schedule.first_after(wall, 0.0))
                        .collect()
                } else {
                    registered
                        .cursors
                        .iter()
                        .map(|cursor| match cursor.monotonic {
                            Some(at) => wall.checked_add(at.saturating_duration_since(mono)),
                            None => cursor.wall,
                        })
                        .collect()
                };
                (id.clone(), cursors)
            })
            .collect();
        SchedulerSnapshot {
            schedules,
            pause: self.pause.clone(),
            circuits: self
                .breakers
                .iter()
                .map(|(id, breaker)| (id.clone(), breaker.snapshot()))
                .collect(),
        }
    }
    /// Validate atomically; never restore partially if a job or pool is unknown.
    pub fn restore(&mut self, snapshot: SchedulerSnapshot) -> Result<(), ConfigError> {
        if self.initialized {
            return Err(ConfigError("restore must precede run".into()));
        }
        if snapshot.schedules.len() != self.jobs.len()
            || snapshot.schedules.iter().any(|(id, cursors)| {
                self.jobs
                    .get(id)
                    .is_none_or(|r| r.job.config.schedules.len() != cursors.len())
            })
        {
            return Err(ConfigError(
                "snapshot jobs/schedules differ from registration".into(),
            ));
        }
        if snapshot
            .pause
            .jobs
            .iter()
            .any(|id| !self.jobs.contains_key(id))
            || snapshot
                .pause
                .pools
                .iter()
                .any(|id| !self.limits.pools.contains_key(id))
        {
            return Err(ConfigError("snapshot pauses unknown job or pool".into()));
        }
        if snapshot.circuits.len() != self.breakers.len()
            || snapshot
                .circuits
                .keys()
                .any(|id| !self.breakers.contains_key(id))
        {
            return Err(ConfigError("snapshot circuit configuration differs".into()));
        }
        let mut breakers = BTreeMap::new();
        for (id, state) in &snapshot.circuits {
            let policy = self.jobs[id].job.policy.circuit.clone().unwrap();
            breakers.insert(
                id.clone(),
                CircuitBreaker::restore(policy, state.clone())
                    .map_err(|e| ConfigError(e.to_string()))?,
            );
        }
        self.breakers = breakers;
        self.pause = snapshot.pause.clone();
        self.restored = Some(snapshot);
        Ok(())
    }
    pub(super) fn restore_cursors(&mut self) {
        let Some(snapshot) = self.restored.take() else {
            return;
        };
        let wall = SystemTime::now();
        let mono = Instant::now();
        for (id, cursors) in snapshot.schedules {
            let registered = self.jobs.get_mut(&id).unwrap();
            for (index, saved) in cursors.into_iter().enumerate() {
                let schedule = &registered.job.config.schedules[index];
                // Never replay an abandoned fixed-delay run or historical occurrence.
                let next = match saved {
                    Some(at) if at > wall => Some(at),
                    Some(at) if !matches!(schedule, Schedule::FixedDelay { .. }) => {
                        schedule.after_tick(at, wall)
                    }
                    _ if matches!(schedule, Schedule::FixedDelay { .. }) => {
                        schedule.after_completion(wall, self.rng.r#gen())
                    }
                    _ => None,
                };
                registered.cursors[index] = Self::cursor(schedule, next, wall, mono);
            }
        }
    }
    pub(super) fn policy_rejection(&self, job: &str) -> Option<Rejection> {
        if self
            .pause
            .is_paused(job, self.jobs[job].job.config.resource_pool.as_deref())
        {
            Some(Rejection::Paused)
        } else if self
            .breakers
            .get(job)
            .is_some_and(|b| !b.can_admit(SystemTime::now()))
        {
            Some(Rejection::CircuitOpen)
        } else {
            None
        }
    }
    pub(super) fn set_pause(
        &mut self,
        scope: PauseScope,
        paused: bool,
        cancel_running: bool,
        observer: &mut impl FnMut(Event<E>),
    ) -> Result<(), ConfigError> {
        match &scope {
            PauseScope::Job(id) if !self.jobs.contains_key(id) => {
                return Err(ConfigError("unknown paused job".into()));
            }
            PauseScope::Pool(id) if !self.limits.pools.contains_key(id) => {
                return Err(ConfigError("unknown paused pool".into()));
            }
            _ => {}
        }
        self.pause.set(&scope, paused);
        if paused && cancel_running {
            let matches = |job: &str| match &scope {
                PauseScope::Global => true,
                PauseScope::Job(id) => id == job,
                PauseScope::Pool(pool) => {
                    self.jobs[job].job.config.resource_pool.as_ref() == Some(pool)
                }
            };
            let tasks: Vec<_> = self
                .running
                .iter()
                .filter(|(_, run)| matches(&run.job))
                .map(|(&task, _)| task)
                .collect();
            let retries: Vec<_> = self
                .queue
                .iter()
                .filter(|run| run.reserved && matches(&run.job))
                .map(|run| run.id)
                .collect();
            for task in tasks {
                self.tasks
                    .request_cancel(task, CancellationReason::Explicit);
            }
            for run in retries {
                self.cancel(run, observer);
            }
        }
        observer(Event::StateChanged(self.snapshot()));
        Ok(())
    }
    pub(super) fn policy_tick(&mut self, observer: &mut impl FnMut(Event<E>)) {
        let now = Instant::now();
        for (&task, run) in &mut self.running {
            if run.exceeded {
                continue;
            }
            let budget = self.jobs[&run.job].job.policy.budget;
            if let Some(timing) = self.tasks.timing(task)
                && let Some(deadline) = timing
                    .started
                    .and_then(|start| budget.runtime_deadline(start))
                && deadline <= now.into_std()
                && timing.finished.is_none_or(|finish| finish >= deadline)
            {
                run.exceeded = true;
                if timing.finished.is_none() {
                    self.tasks
                        .request_cancel(task, CancellationReason::RuntimeBudget);
                }
                observer(Event::RuntimeExceeded {
                    job_id: run.job.clone(),
                    run_id: run.id,
                    attempt: run.attempt,
                });
            }
        }
        let mut i = 0;
        while i < self.queue.len() {
            let run = &self.queue[i];
            let budget = self.jobs[&run.job].job.policy.budget;
            if budget
                .queue_deadline(run.eligible.into_std())
                .is_some_and(|deadline| deadline <= now.into_std())
            {
                let run = self.queue.remove(i).unwrap();
                self.advance_delay(&run);
                observer(Event::QueueExpired {
                    job_id: run.job,
                    run_id: run.id,
                    attempt: run.attempt,
                });
                observer(Event::StateChanged(self.snapshot()));
            } else {
                i += 1;
            }
        }
    }
    pub(super) fn policy_wake(&self) -> Instant {
        let now = Instant::now();
        let mut next = now + Duration::from_secs(1);
        for (&task, run) in &self.running {
            if !run.exceeded
                && let Some(timing) = self.tasks.timing(task)
                && timing.finished.is_none()
                && let Some(deadline) = timing.started.and_then(|start| {
                    self.jobs[&run.job]
                        .job
                        .policy
                        .budget
                        .runtime_deadline(start)
                })
            {
                next = next.min(deadline.into());
            }
        }
        for run in &self.queue {
            if run.eligible > now {
                next = next.min(run.eligible);
            }
            if let Some(deadline) = self.jobs[&run.job]
                .job
                .policy
                .budget
                .queue_deadline(run.eligible.into_std())
            {
                next = next.min(deadline.into());
            }
        }
        let wall = SystemTime::now();
        for breaker in self.breakers.values() {
            if let Some(until) = breaker.snapshot().open_until
                && let Ok(remaining) = until.duration_since(wall)
                && !remaining.is_zero()
                && let Some(at) = now.checked_add(remaining)
            {
                next = next.min(at);
            }
        }
        next
    }
    pub(super) fn complete_with_policy(
        &mut self,
        mut run: Run<P>,
        completion: TaskCompletion<E>,
        observer: &mut impl FnMut(Event<E>),
    ) {
        let policy = &self.jobs[&run.job].job.policy;
        if !run.exceeded
            && let (Some(start), Some(finish), Some(budget)) = (
                completion.timing.started,
                completion.timing.finished,
                policy.budget.max_runtime,
            )
            && finish.saturating_duration_since(start) >= budget
        {
            run.exceeded = true;
            observer(Event::RuntimeExceeded {
                job_id: run.job.clone(),
                run_id: run.id,
                attempt: run.attempt,
            });
        }
        let interrupted = matches!(
            completion.cancellation,
            Some(CancellationReason::Explicit | CancellationReason::Shutdown)
        );
        if let Some(permit) = run.circuit.take()
            && let Some(breaker) = self.breakers.get_mut(&run.job)
        {
            let outcome = if interrupted || matches!(completion.exit, TaskExit::Aborted) {
                CircuitOutcome::Ignored
            } else if run.exceeded {
                CircuitOutcome::Failure
            } else {
                match &completion.exit {
                    TaskExit::Finished(Ok(())) => CircuitOutcome::Success,
                    TaskExit::Finished(Err(_)) | TaskExit::Panicked(_) => CircuitOutcome::Failure,
                    TaskExit::Aborted => CircuitOutcome::Ignored,
                }
            };
            breaker.record(permit, outcome, SystemTime::now());
        }
        let retry =
            if !self.admission_closed() && !run.exceeded && completion.cancellation.is_none() {
                match &completion.exit {
                    TaskExit::Finished(Err(error)) => {
                        policy.retry_delay(error, run.attempt, self.rng.r#gen())
                    }
                    _ => None,
                }
            } else {
                None
            };
        observer(Event::Completed {
            job_id: run.job.clone(),
            run_id: run.id,
            attempt: run.attempt,
            completion,
            runtime_exceeded: run.exceeded,
            will_retry: retry.is_some(),
        });
        if let Some(delay) = retry {
            run.attempt += 1;
            run.eligible = Instant::now() + delay;
            run.started_reported = false;
            run.exceeded = false;
            run.circuit = None;
            observer(Event::RetryScheduled {
                job_id: run.job.clone(),
                run_id: run.id,
                next_attempt: run.attempt,
                delay,
            });
            self.queue.push_back(run);
        } else {
            self.advance_delay(&run);
        }
        observer(Event::StateChanged(self.snapshot()));
    }
}
