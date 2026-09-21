# Step 06: background tasks

Implemented in three optional stages: 06a ownership (`tasks`), 06b scheduling
(`task-scheduling`), 06c execution policies (`task-policies`). Library delivery
only: no consumer has adopted these APIs yet. Applications retain runtime,
signals, process exit policy, storage, durable claims/leases/fencing/recovery,
job logic, configuration and administrative endpoints. No HTTP dependency.

## 06a: implemented ownership contract

`WorkTracker` atomically reserves named external work with a non-cloneable guard.
Closing permanently rejects admission; waiting requires both closure and zero
guards. Reserve before handing callbacks to an external executor, including
pending WebSocket upgrades. Releasing a guard proves scope completion, not success.

`TaskSet<E>` owns async and blocking jobs and retains original errors. Its explicit
nonzero capacity includes completed, unconsumed results. Factories execute inside
the task boundary; panics are named outcomes, not silent failures. `TaskContext`
provides independent cooperative cancellation. `close` stops admission;
`request_shutdown` also notifies CancelOnShutdown jobs. FinishOnShutdown jobs
continue. Individual failures do not cancel siblings. Nested spawns remain application-owned;
this API tracks submitted jobs, not arbitrary child work.

`drain_until` uses one absolute monotonic deadline, returns completed outcomes and
unfinished identities, and retains unfinished ownership for a subsequent drain.
Cancelling a drain future retains already collected outcomes. Cancellation requests
are metadata, never proof of termination. Async abortion is explicit. Blocking
work cannot be aborted through this API. Dropping the owner requests shutdown and
detaches unfinished work; it is abandonment, not successful draining.

Compose with Lifecycle using its existing application-owned service futures and
budget; no second grace period or signal handler is installed. Do not close shared
resources while unfinished work still uses them. See `examples/tasks.rs`.

## 06b: implemented scheduling contract

06b provides a runnable scheduler and reusable schedules: manual/events,
fixed-rate/fixed-delay intervals, delayed/immediate first runs, fixed-delay jitter,
UTC cron, bounded admission and application-named resource pools. Applications
supply typed payloads and mandatory observers; no unbounded history is retained.

## 06c: implemented execution policies

06c adds optional queue/runtime budgets, classified bounded retries,
per-job circuit breakers, pause controls and control-state snapshots. Runtime
expiry must retain execution permits until the task actually finishes. Snapshots
are application-persisted control state, not durable job delivery.

## Design evidence

Pezzottify provides cooperative blocking jobs, resource-class limits, jitter,
execution budgets, persisted controls and history. Its declared cron variants
currently have a stub execution path; working cron examples are Fausto and
Pezzottflix. Meteonesto's leases, fencing and recovery demonstrate why durable
execution remains application-owned. Fausto and LelloStore demonstrate admission
races and reservations made before callback startup.

## Scheduling details

`Scheduler<P, E>` registers async or blocking `Job`s before its first `run`.
`SchedulerHandle<P>` accepts typed manual/event commands through a bounded
channel. Losing an acknowledgement does not cancel accepted work. Per-job
concurrency defaults to one and pending capacity to zero; global running,
in-flight and command capacities are mandatory nonzero values. Named resource
pools restrict execution; the oldest eligible run starts without head-of-line
blocking from saturated pools. Multiple schedules/events share these limits.

Intervals use monotonic clocks. Fixed-rate intervals skip missed ticks;
fixed-delay intervals restart after the scheduled run finishes, with one sampled
positive jitter per occurrence. Manual runs do not reset schedules. Cron uses
UTC, seconds-first six/seven-field expressions, and skips historical replay.
UTC deadlines are checked at least once a second for clock changes. There is no
implicit catch-up burst. Invalid registrations fail before serving work.

The caller drives `run(&mut self, shutdown, observer)`. The observer receives
admission, start, completion, rejected and cancelled events, must not block or
panic, and owns logging/metrics/history. There is no hidden supervisor or history
queue. Failures/panics do not stop siblings. Shutdown drops pending commands,
reports queued cancellations and drains accepted executions according to their
shutdown policy. An outer Lifecycle deadline may drop `run` while the scheduler
owner remains available for inspection and further draining. See
`examples/scheduler.rs` for a compiling composition.

## Policy defaults and execution boundaries

`ExecutionPolicy<E>` defaults to no deadlines, retries or breaker. Enable
`task-policies` independently for `ExecutionBudget`, `RetryPolicy`,
`CircuitBreaker`, `PauseState` and their snapshots. When also enabling
`task-scheduling`, attach policies with `Job::with_policy`.

Queue budgets begin when an attempt becomes eligible for shared capacity and
exclude backoff. Once dispatched, Tokio's own blocking-pool queue is outside the
shared queue budget. Execution timing starts inside the task, immediately before
the factory executes, and ends when that scope exits (including panic/abort).
Actual timestamps prevent delayed observer processing from inventing overruns.
Runtime expiry requests cancellation once and emits `RuntimeExceeded`; ownership,
resource-pool capacity and overlap protection remain held until actual completion.
An eventual application error remains available alongside the overrun flag.

Retries require an explicit classifier, maximum attempts including the first,
nonzero initial delay and cap. Positive jitter is included inside the cap.
Classifiers, like observers, must be brief and non-panicking. Only classified
application errors retry: no automatic retries for panic, abort, cancellation,
queue expiry or runtime overrun. Backoff releases execution capacity but keeps the
logical in-flight/per-job reservation. Shutdown cancels pending retries. Completed
events describe each attempt; `will_retry` distinguishes a terminal outcome.

Circuit breakers count completed failed attempts, panics and runtime overruns;
explicit/shutdown cancellation, abort and queue expiry do not count. Cooldowns
use caller-supplied UTC timestamps so applications can restore them. Clock changes
can therefore change cooldown wall time; runtime/queue budgets remain monotonic.
Exactly one half-open probe is admitted after cooldown. Generation-bound,
consumed permits prevent stale completions from closing a newer circuit.

Global, resource-pool and job pauses compose. Pausing prevents new admission and
queued starts; existing queued work still has its queue budget. `cancel_running`
also requests cancellation of matching executing jobs and cancels their pending
retries, independently of shutdown behavior. Manual commands respect controls.
The scheduler emits control snapshots through `StateChanged`; applications own
the persistence transaction and its failure policy.

## Snapshot boundaries

`Scheduler::snapshot` / `SchedulerHandle::snapshot` export schedule deadlines,
pause state and circuit state. `restore` validates the entire snapshot before
changing anything, must run before scheduling, and freezes registration. Restore
only with the same application-versioned job/schedule configuration; structural
validation does not identify semantic changes to job code or cron expressions.
Before first startup, exported initial interval deadlines use zero jitter; later
snapshots preserve the sampled deadline. A snapshot is not a job-delivery log.

No queues, payloads, active tasks, retry attempts or active half-open probes are
restored. Expired occurrences are skipped. An abandoned fixed-delay run starts a
fresh delay; it is not replayed. Applications serialize library-owned snapshot
types using their own schema (no required serde or storage backend). The
`policy_snapshot` example demonstrates exact timestamp serialization around the
circuit primitive, with no claim of durable execution.

## Validation and delivery

Consumer-shaped contract tests cover callback/upgrade reservations, cooperative
blocking work, claim finalization, resource-pool isolation, bounded ingress,
interval/UTC behavior, cancellation-safe draining, retries and circuit fencing.
Paused Tokio time and deterministic jitter avoid timing-dependent policy tests;
real blocking tests verify executor queue time and late completion observation.
All consumer rows remain Pending, not adopted, until production call sites are
migrated separately. See both migration trackers for staged verification records.
