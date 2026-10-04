# Step 07c: SQLite backup coordination

Implemented in 0.1.7, 2026-10-04. Feature `database-sqlite-backup` exposes
`database::sqlite::backup`; no HTTP, runtime or native SQLite driver dependency.
Consumer adoption is Pending assessment/canary, independently of 07a/07b/07e.

## Contract

`prepare_checkpoints` handles a Pezzottify-style registry callback. Each report
retains the source, explicit checkpoint mode, native error and SQLite's three
result columns. Prepared, busy, invalid and failed are distinct. Partial passive
checkpoints are busy even when SQLite's busy column is zero. Non-WAL sentinel
counts are unverified/invalid preparation, never interpreted as a completed copy.
Prepared means checkpoint preparation only. The caller controls registry locking
and duplicate entries; reports do not imply a cross-file atomic snapshot.

`run` and `run_async` execute the same explicit per-file pipeline:

1. Acquire the application's coordination guard.
2. Prepare either an explicit checkpoint or an engine-consistent copy.
3. Reserve new, exclusively owned staging.
4. Copy using the chosen strategy.
5. Verify both SQLite integrity and the application's expected schema/markers.
6. Atomically publish without replacing an existing artifact.
7. Clean staging, retaining the coordination guard through completion.

Reports preserve completed phases, failed phase, checkpoint facts, verification
scope, native primary and cleanup errors and retained-staging status. A successful
publish remains Published even when staging cleanup fails: it must not be retried
as though publication never happened. Failure before reservation cannot clean
someone else's output. Multi-file callers aggregate individual reports explicitly.

## Adapter responsibilities

`Adapter` serves synchronous drivers, and `AsyncAdapter` uses Send futures for
native async drivers such as SQLx. Core coordination has no filesystem or driver
side effects. Adapters must meet these requirements:

- Resolve canonical aliases and symlinks, enforce permissions and deployment
  fencing, and reject destinations aliasing the source.
- For checkpoint/raw copy, hold a tested coordination protocol preventing writes
  and competing checkpoints during copying. A checkpoint alone does not make
  raw copying safe. For consistent copy, use an engine-supported snapshot method
  such as SQLite backup or VACUUM INTO; do not substitute a main-file copy.
- Reserve staging without overwriting existing output. Reservation failure must
  clean any output it created before returning an error.
- Verify the concrete application schema/ledger, not merely that tables exist.
- Publish atomically with no replacement. Publication errors mean no publication
  occurred and staging is still owned. Keep staging available for cleanup after
  publication success; cleanup must never delete the published artifact.

No callback is retried. Panics and abruptly abandoned async futures are not
converted into normal reports: adapters must use RAII fencing, and callers must
own and await backup tasks through shutdown. An abandoned copy can leave staging
that requires application reconciliation. There is no automatic restore.
Retention, encryption, external payloads and cross-file consistency remain local.

Use `database::blocking::Executor` for synchronous adapters called from async
handlers. A caller timeout can leave the backup operation running, just like any
other synchronous write; drain it before releasing application-level ownership.

## Verification

Real SQLite fixtures cover checkpoint copying, held WAL reader/busy results,
engine-consistent copying with a concurrent reader and committed WAL data,
integrity/schema rejection, collisions, source protection, failures at every
phase, cleanup failure before/after publication and per-file partial outcomes.
Async fixtures exercise the same adapter pipeline and failure cleanup. These are
shared contract tests; no production service is counted as adopted yet.
