# Step 07d: bounded synchronous database execution

Implemented locally, 2026-10-04. Feature `database-blocking` exposes
`database::blocking`. It requires Tokio's notification/time primitives but no
HTTP, SQLite driver, task module or Tokio runtime for synchronous callers.
Consumer adoption is Pending assessment/canary.

## Configuration and scheduling

`Executor::new(Config)` starts a fixed number of dedicated standard threads.
There are no hidden workload defaults: the caller supplies worker count,
priority weights and per-priority waiting queue capacities, plus lane limits.
Priority/lane indices are application-defined; Pezzottify names are not baked
into the library. Configuration rejects empty classes/lanes and more than 65,536
total weight slots. Worker creation failures close already-started workers.

Dispatch follows weighted round robin. Each priority is FIFO among eligible
jobs; jobs blocked by a saturated lane may be bypassed so they do not reserve
idle workers or stall other lanes. Continuously eligible lower-priority work
receives its weighted turn. Fairness is measured in dispatches, not execution
seconds; the executor cannot preempt long native calls.

`submit(options, closure)` immediately admits or rejects. It never waits for a
full queue: QueueFull, Closed and unknown indices are explicit errors, and the
rejected closure is not retained. Capacity bounds queued jobs separately from
running jobs. Cancelled/expired queue entries are purged. Queue deadlines start
at admission; execution deadlines start when a worker actually dispatches work.
`None` means unlimited. Zero durations expire immediately. Durations overflowing
Instant are rejected. There is no automatic retry.

Closures return typed application results and errors. Native error types are
preserved in RunError::Operation. Panic payloads become RunError::Panicked; worker
and lane occupancy are released and subsequent jobs continue. Panic-abort builds
cannot isolate panics. Application code must not synchronously wait on work that
requires the same occupied worker/lane; the scheduler cannot resolve that cycle.

## Cancellation, timeouts and shutdown

`Ticket::wait` asynchronously observes the result without blocking Tokio workers;
`wait_blocking` uses the identical scheduler from an ordinary synchronous thread.
Do not call blocking wait/drain on a runtime worker. Dropping an undispatched
ticket or its wait future cancels the queued operation. Once dispatched, dropping
the ticket abandons only its result; the operation continues.

QueueTimeout proves that the closure did not dispatch. ExecutionTimeout means
that the caller's runtime budget expired, **not** that native work stopped or
rolled back. Worker and lane capacity stay occupied until the closure actually
returns. A late completed result also reports ExecutionTimeout. A timed-out
write may subsequently commit; never retry it automatically. No interrupt is
invoked implicitly; a driver-specific interrupt can remain an application-owned
explicit capability.

`close(Drain)` closes admission while processing accepted work.
`close(CancelQueued)` cancels queued work and lets running operations finish.
`drain()` and `drain_blocking(timeout)` observe real queued/active completion;
call close first to prevent new submissions racing with drain. Async callers
can bound waiting with their existing Tokio timeout. A drain timeout does not
stop running work. Drop of the last executor closes admission/cancels queues
without blocking or forcibly terminating threads. Explicit drain is required
before tearing down database resources. Completion does not promise thread join,
only that no queued or active database operation remains.

## Verification

Controlled channel barriers test weighted FIFO dispatch, fairness, independent
lanes, bounded admission, queue expiry and cancellation, runtime expiry without
capacity release, late-result deadlines, native errors, panic recovery, sync/async
waiters, close modes, last-owner drop and reentrant cancelled-closure destruction.
A real SQLite fixture holds a transaction beyond its caller's timeout, verifies
that capacity remains occupied and the write is initially invisible, then allows
the transaction to commit exactly once before drain completes.
