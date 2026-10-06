# Engine-owned application runtime

ScT's production server uses `#[simple_server::main]`. There is one engine-owned
runtime for the application and HTTP serving; there is no application Tokio
runtime and no process-global HTTP-only runtime. The normal/build dependency
graph must exclude Tokio, SQLx, Reqwest, Axum and Hyper. Development fixtures,
standalone client programs and offline recovery are separate graphs.

## Interfaces used by ScT

| Application need | Shared interface | Execution |
| --- | --- | --- |
| Entry point and spawned work | `main`, `runtime`, `engine_tasks` | Engine runtime |
| Deadlines, heartbeat, shutdown | `time`, `engine_lifecycle`, `task_drivers` | Engine timers/signals |
| PostgreSQL | `database::postgres` | Native SQLx pools and transactions |
| OIDC and S3 transport | `client` | Native Reqwest, TLS and streaming |
| Files and streaming | `fs`, `io` (`engine-io`) | Standard file operations on engine blocking workers; futures I/O traits |
| Concurrency limits | `sync` (`engine-io`) | Executor-independent mutexes/semaphores |
| Archive subprocesses | `process::ManagedCommand` | Standard process spawn, engine-timed waiting and blocking reaping |
| Correlation | `correlation-core` | Scoped host context per poll/drop; no Tokio |

Application futures and business logic remain host-owned. The C ABI carries
opaque resources, owned buffers, wakeups and callback functions; Rust runtime
handles, driver types and futures do not cross the ABI as Rust objects.

PostgreSQL resource kinds 18 (pool) and 19 (transaction) belong to the engine.
Pools initialize lazily in native runtime context. Transactions retain that
context for destruction/rollback even when an external executor releases their
host handle. Operations serialize access to each transaction. Parameter types
are explicit, including SQL NULL versus JSON null; result columns preserve name,
value and supported-type status. Accessing an unsupported column type returns a
decoding error instead of fabricating a value. Queries still use bound parameters.

Supported PostgreSQL values are text, signed 16/32/64-bit integers, booleans,
finite floating point, bytes, text arrays and JSON. Forward migrations execute through
native SQLx, retaining its advisory locking, migration ledger and SHA384 checksum
validation. The optional filename loader only accepts forward numbered `.sql`
files. This is not a claim of complete SQLx API/type compatibility.

Engine context must also be restored for callback registration destruction and
retained reply-buffer destruction, not only polling or future cancellation.
A content-length response can finish without an EOF poll; the final body drop
must still be able to schedule release of ScT's database read pin. The
`engine_callback_drop` regression runs without `engine-tokio`.

Filesystem reads preserve pending data across cancellation and smaller subsequent
buffers. Read/write/seek operations coordinate pending work. `sync_all` waits for
pending writes before fsync. A blocking OS read cannot itself be interrupted by
cancelling its future; retain bounded resources and close/kill the producer as
appropriate. Managed child destruction uses the captured engine runtime to reap
children and honors explicitly requested kill-on-drop.

## Verification and packaging

`tests/engine_postgres.rs` covers typed values, NULLs, transaction cancellation,
rollback from an external executor, and changed-migration rejection. Set
`SIMPLE_SERVER_TEST_POSTGRES` to a disposable database and run its ignored tests.
`tests/engine_files.rs` covers durable files, cancelled reads, and child reaping.
`tests/engine_callback_drop.rs` verifies asynchronous cleanup on body destruction.

The library and engine must be rebuilt/pinned together for these new commands.
ScT's `simple-server.rev` selects the source exported into Docker; the engine image
is reused by its builder and runtime stages. No artifacts are published by the
local migration. See the central migration trackers for actual checks, revisions,
measurements and remaining qualification limits.
