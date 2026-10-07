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

## OpenID Connect

Enable `oidc` with default features disabled to use `simple_server::oidc::Client`.
The native engine contains `openidconnect` and its verification/crypto dependencies;
only owned configuration strings, authorization-flow material and verified subject
strings cross the existing framed ABI. No OIDC types are re-exported to consumers.
Rebuild the engine from the same pinned source revision as these bindings; older
engines do not implement the new commands. ABI table layout is unchanged.

`Client::discover(Config)` performs discovery and loads provider keys.
`begin()` produces an authorization URL, fresh state, nonce and S256 PKCE verifier.
The application stores those secrets with its browser binding. It must validate
state/browser binding and atomically consume the flow before calling
`finish(code, nonce, verifier)`. `finish` exchanges the code and verifies signature,
issuer, audience, expiration, nonce and the access-token hash when present, then
returns the subject. Associate the subject with the configured issuer, never as a
globally unique account identifier. Sessions, authorization and database ownership
remain with the application. This API covers confidential-client authorization
code flows; it does not add refresh, logout, user-info or automatic JWKS refresh.

Requests retain the ten-second timeout, proxy defaults and disabled redirects.
HTTPS is required; the explicit development option permits HTTP only to localhost,
127.0.0.1 or [::1]. This check covers discovered endpoints and each outgoing
request, including the JWKS fetch during discovery. Credentials in endpoint URLs
are rejected. Errors contain only fixed categories, never provider responses,
codes or secrets; Debug output redacts configuration and flow material.

The host owns a resource before discovery starts, so cancellation releases it.
Clones retain the provider; in-flight operations retain it until completion or
cancellation. There is no global application session state inside the engine.
`tests/engine_oidc.rs` checks discovery policy, generated flows and clone lifetime;
the standalone engine consumer and dependency guard ensure no host OIDC stack.
ScT's real login fixture covers token verification, PKCE, replay, sessions and CSRF.

## Latest-value notifications and missed ticks

`engine-io` exposes `sync::watch::{channel, Sender, Receiver}` without Tokio.
It coalesces sends, tracks each receiver's seen version, unregisters cancelled
waiters, and delivers an unseen final value before asynchronous closure. As in
Tokio, `has_changed` reports closure even when a final value remains unseen.
`borrow_and_update` returns a short-lived locking borrow; do not hold it across
an await or reenter the channel while holding it. This is a deliberately limited
API, not a complete Tokio watch replacement. The existing async-lock mutex and
semaphore exports include owned guards (`MutexGuardArc`, `SemaphoreGuardArc`).

Engine intervals retain an immediate first tick and default Burst behavior.
`MissedTickBehavior::{Burst, Delay, Skip}` controls ticks more than 5ms late.
Skip retains the original schedule while discarding missed ticks; cancellation
of a pending tick preserves its deadline. Weather API uses Skip for publication
polling and popularity persistence. These helpers execute with engine timers and
runtime-independent synchronization; no new native ABI operation is needed.

## Zstd buffer codecs

The `zstd` feature provides synchronous `encode_all`, `decode_all` and
`decode_all_limited` buffer APIs backed by the native engine. It requires no
runtime; async callers schedule large work on blocking workers. Wire payloads
are raw bytes in the existing framed ABI, so no base64/JSON byte arrays or new
ABI table fields are needed. Calls allocate/copy across the ownership boundary;
this is not a zero-copy or streaming API. Decode accepts concatenated/skippable
frames and propagates malformed/truncated-frame errors. The optional limit
bounds expanded output length, not decoder window allocation or CPU time.
Unbounded decode preserves Weather API's existing trusted-publication policy.
Compression level zero uses Zstd's default. The native lock retains the
consumer's zstd 0.13.3 / zstd-safe 7.2.4 / zstd-sys 2.0.16 (Zstd 1.5.7).

Existing consumers that could reuse this include weather-pipeline's buffered
artifact encoding/decoding. Its streaming backup paths and Simple Agents'
streaming archive extraction would require a separate streaming API; they are
not migrated here. PV Estimator's embedded catalogue also uses buffer decoding,
but adopting a dynamic native library there needs a target/packaging assessment.
Observo declares Zstd but this scan found no source call; declaration alone is
not evidence of adoption. No other consumers change in this migration.

## Engine-owned logging

Enable `engine-logging` with default features disabled to keep the formatter,
subscriber, EnvFilter and regex implementation in the prebuilt engine. The host
retains `tracing`, `tracing-core`, a lightweight `tracing-log` adapter and framed
event/span serialization. Install with
`engine_logging::try_init(LoggingOptions, FilterMode)`, then optionally
`engine_logging::init_log_bridge()`. No async runtime is required.

The caller owns environment lookup. Strict rejects malformed/empty filters;
Lossy preserves `EnvFilter::new` (invalid directives discarded, ERROR fallback);
StrictOrInfo preserves `EnvFilter::try_new` with INFO fallback on parse failure.
Text/pretty/compact/JSON, stdout/stderr, ANSI, targets, typed fields, span
parents/records/clones and lifecycle events are supported. Native dispatch stays
on the caller thread. Initialization is once per process; configuration/backend
errors remain distinguishable. Existing global subscribers/loggers are preserved.

This is fixed configuration; source `logging` reload APIs remain unchanged and
are not exposed through the engine. Static callsite metadata is retained for the
process lifetime, matching tracing's static metadata contract. Span references
are released separately. Each enabled event crosses a synchronous framed ABI and
copies/serializes fields before formatting; this reduces downstream compilation,
not necessarily logging CPU time. No runtime speedup is claimed.

The engine requires the logging commands added by this change; older native
artifacts return an initialization error. No ABI table layout change is required.
Applications should pin matching source/artifact revisions. Weather API is the
first migrated consumer; other services are unchanged.

## Engine-owned SHA-256

The optional `hashing` feature exposes `hashing::Sha256` without a host crypto
implementation. The native engine retains SHA-256 from sha2 0.10.9; only framed
bytes, numeric resource IDs and the 32-byte result cross the stable C ABI. No
runtime is required. `Digest` converts to `[u8; 32]`, exposes a byte slice and
supports lowercase hex formatting.

`try_digest`, `try_new`, `try_update` and `try_finalize` return backend errors.
The `std::io::Write` implementation is also fallible. `digest`, `new`, `update`
and `finalize` are convenience methods that panic if the engine cannot perform
the operation; they never substitute an empty/partial digest. Authentication
callers should use the fallible methods and reject on failure. Once an update
fails, the state is poisoned and cannot be finalized or retried.

Incremental updates coalesce small writes into an 8 KiB buffer, preserving their
byte order. `flush` sends pending bytes without finalizing. Finalization consumes
the hasher; drop releases unfinished native state without hashing pending bytes.
Independent hashers do not share digest state or a lock during hashing. The
host does not retain the complete input when used as a streaming writer. Whole
buffer calls and large updates copy their input across the ABI; these APIs are
synchronous and not zero-copy. Use blocking workers for large async workloads.
No runtime throughput improvement is claimed.

The implementation adds commands and resource kind 22 without changing the ABI
table. Use matching pinned engine/source artifacts; older engines return an
unsupported-command error through the fallible APIs. This migration covers
Weather API only, not other services or existing host postgres-client hashing.

## Gateway runtime and client compatibility

The `runtime` feature exports `signal::UnixSignal` / `UnixSignalKind` on Unix.
Install a repeatable Hangup, Interrupt or Terminate receiver inside an engine
runtime; `recv` returns an I/O error if the receiver closes. Cancelled waits can
be followed by another wait. Dropping a receiver releases it but does not restore
the process-wide default signal disposition (matching Tokio). These additive
commands use resource kind 23 without an ABI table change. Existing one-shot
shutdown `Signals` remain unchanged.

`engine-io` also exports runtime-independent `sync::RwLock` and `sync::oneshot`.
`client::ClientBuilder::no_decompression` explicitly disables gzip, Brotli,
deflate and Zstd response decoding, preserving bytes even if native dependency
features later enable those codecs. This is separate from redirect/proxy policy,
which remains configured by the application.

Weather Gateway uses these alongside the existing engine HTTP server/client,
entry point, synchronization, timers, tasks, lifecycle and logging. Access-token
JWT/JWKS verification and issuer/audience/key-refresh policy remain application
code; this is not the authorization-code OIDC helper used by ScT.
