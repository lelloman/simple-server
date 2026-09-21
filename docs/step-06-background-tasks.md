# Step 06: background tasks

Three optional stages: 06a ownership (`tasks`), 06b scheduling
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
continue. Individual failures do not cancel siblings.

`drain_until` uses one absolute monotonic deadline, returns completed outcomes and
unfinished identities, and retains unfinished ownership for a subsequent drain.
Cancelling a drain future retains already collected outcomes. Cancellation requests
are metadata, never proof of termination. Async abortion is explicit. Blocking
work cannot be aborted through this API. Dropping the owner requests shutdown and
detaches unfinished work; it is abandonment, not successful draining.

Compose with Lifecycle using its existing application-owned service futures and
budget; no second grace period or signal handler is installed. Do not close shared
resources while unfinished work still uses them. See `examples/tasks.rs`.

## Remaining stages

06b will provide a runnable scheduler and reusable schedules: manual/events,
fixed-rate/fixed-delay intervals, delayed/immediate first runs, fixed-delay jitter,
UTC cron, bounded admission and application-named resource pools. Applications
supply typed payloads and mandatory observers; no unbounded history is retained.

06c will add optional queue/runtime budgets, classified bounded retries,
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
