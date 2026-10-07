# simple-server

A small, modular Rust library with owned server interfaces. Axum remains an
internal HTTP implementation; consumers use the library's routing, handler,
extractor, response, middleware and serving contracts. The transitional Axum
re-export has been removed for all 17 active migrated services. Quentin Torrentino was
explicitly excluded and must migrate before building against this revision. See the
[design and completion criteria](docs/design.md#end-goal-completely-abstract-axum-away).

**Status:** Axum dependency centralization and opt-in lifecycle, logging,
correlation, HTTP tracing, body limits, response-header helpers, CORS, health checks, task ownership, scheduling, execution policies, combined auth and rate limiting are implemented. All 17 inventoried products have adopted lifecycle helpers locally
with documented application scopes. Crumbles and SCT include those migrations on local `master`. Public crates.io releases are available; backup coordination and bounded database workers are new in 0.1.7.

Open the [HTML migration matrix](docs/migration-status.html) in a browser for adoption status
across 17 active services plus excluded Quentin Torrentino and the completed steps for each one.

All consumer migrations follow the [reusable worktree and verification workflow](docs/consumer-migration-workflow.md),
including applicability checks and updates to both adoption trackers.

## Shared engine development

An unpublished shared-engine implementation is under development. The opt-in
`runtime`, `client`, `process`, and `sqlite-client` features use a Rust-built
`libsimple_server_engine.so.1` through a C ABI. The public HTTP server and existing
source lifecycle/scheduling modules still use the source backend. This is not yet the
planned complete engine release, and no production consumer has migrated.

The unpublished migration also owns the public networking and task-error
contracts: `http::bind` returns `net::TcpListener`, HTTP/TLS serving accepts that
listener, and Unix serving accepts `net::UnixListener`. Generic bind helpers use
`net::ToSocketAddrs`; common address values are unchanged. Application-configured
standard-library sockets can be imported with `from_std` and exported with
`into_std`. Bounded-batch callbacks use `task_scheduling::JoinError` (including
panic payloads and an owned `BatchTaskId`); delayed admission uses
`rate_limit::AcquireError`. Callers explicitly naming the former Tokio types
must update those annotations. These source adapters still require Tokio;
wrapping their public contracts does not yet move execution into the engine or
reduce the default consumer dependency graph.

Engine tasks can now be collected with `runtime::JoinSet`: join by completion,
retain task IDs through errors, abort and drain, or explicitly detach. Runtime
handles support spawning from other threads without exposing Tokio. These APIs
use the existing prebuilt engine and add no consumer dependencies.

Bounded task ownership is available through `engine_tasks::{TaskSet, WorkTracker}`
with `default-features = false, features = ["engine-tasks"]`. It preserves
admission limits, cooperative shutdown, blocking jobs, panic outcomes and
resumable draining. Dropping a set requests shutdown and detaches remaining work;
only draining proves completion. Deadlines and task timestamps use
`simple_server::time::Instant`. This selection has seven normal/build dependency
packages beyond simple-server, with no Tokio. Existing `tasks` callers keep the
source runtime and standard-library Instant API, even when both features are
enabled. Both modules share the ownership algorithm; the engine is supplied
separately and no compilation or execution speedup has been benchmarked.

For bounded batches and durable-work polling alone, use
`default-features = false, features = ["task-drivers"]` and the
`simple_server::task_drivers::{run_bounded_batch, run_poll_worker}` APIs inside an
owned engine runtime. This feature has seven normal/build dependency packages
(excluding simple-server), compared with 30 for source `task-scheduling` without
default features. It does not compile Tokio. These counts describe the Rust
consumer graph; the engine is supplied separately, and no timing speedup has
been measured. Callers choose the engine backend through `task_drivers`;
existing `task_scheduling` callers retain their source runtime even when both
features are enabled. The modules share their driver algorithms and behavioral
tests.

The full scheduler is available through `engine_scheduling` with
`default-features = false, features = ["engine-scheduling"]`. It includes
fixed-delay/fixed-rate and cron timing, manual/event admission, resource limits,
retry/budget/circuit policies, pause/restore, and explicit shutdown. Jobs and
timers run in the engine; callers use `engine_tasks` task types and the shared
`task_policies` configuration. Source `task_scheduling` remains source-backed.
The standalone consumer has 26 dependencies beyond simple-server. This includes
Tokio **only for synchronization and macros**, with no Tokio executor, networking,
signals or timers in its normal/build graph. A check guards that feature boundary.
Calendar parsing, policy logic and small synchronization helpers still compile
downstream; the native artifact is unchanged. Public HTTP execution migration
remains pending.

Shutdown coordination is available through `engine_lifecycle` with
`default-features = false, features = ["engine-lifecycle"]`. It preserves borrowed
service futures and one deadline shared by draining and cleanup. Explicit
`Signals::install()` registers SIGINT/SIGTERM in the engine. This selection has
12 normal/build dependencies beyond simple-server and no Tokio dependency.
The existing `lifecycle` module keeps its source runtime; both modules share
the coordination algorithm and runtime-independent `Shutdown` notification.
`time::{Instant, Sleep, sleep_until, timeout_at}` use the engine clock, including
paused time. Timers fix their deadline when constructed. These additions require
a rebuilt development engine; older artifacts lack the appended clock ABI.

For local development, run `bash scripts/check-engine`. It builds the engine
separately, then verifies the bindings, C header, client/SQLite/process APIs and
a standalone consumer without heavy runtime dependencies. To use an existing
build, set `SIMPLE_SERVER_ENGINE_DIR` to the directory containing
`libsimple_server_engine.so`. See [engine artifact instructions](docs/publishing.md#shared-engine-artifacts-unpublished).

The public `engine_web` module (`default-features = false, features = ["engine-web"]`)
now exposes engine TCP listeners, route/method assembly, raw request handlers,
streaming bodies and explicit graceful shutdown. Handlers accept `Request` and
return `Response`; application state can use closures or `with_state`. Route construction is
fallible, and handler registration requires the caller's engine runtime. Request
extensions provide `RequestMetadata` with peer, matched/original paths and path
parameters. Bodies support collection limits, lazy streams and trailer frames.
See the runnable [public HTTP consumer](tests/fixtures/engine_web/src/main.rs).
With the engine client enabled, its graph has 30 dependencies beyond simple-server,
without Axum, Tokio, Hyper, Reqwest, SQLx or Rustls. The `.so` is supplied separately;
this is not a measured performance claim.

`Router<S>` and `MethodRouter<S>` describe state still to be supplied. Use
`with_state(state)?` to bind existing handlers; only `Router<()>` can serve.
Binding a method group first preserves its local state when the enclosing router
later receives different state. Nested/merged routers, typed fallbacks and
`MethodRouter::any_handler` follow the same rule. State stays in host callbacks;
it never crosses the ABI. Construction and binding are fallible and validate
native route/method conflicts before serving.

Typed engine handlers use `MethodRouter::on_handler(method, handler)` or
`on_state(method, state, handler)`; raw `on` closures retain their existing API.
The shared handler machinery supports zero to sixteen arguments, with body
extraction only in the final position. Engine `State`, `Path`, `Query`, `Json`, optional
JSON, `Form`, `Extension`/optional Extension, strings/bytes, raw query and
connection/route metadata extractors are available alongside the shared custom
`Extract` contract. Body-consuming
extractors default to 2 MiB; a `BodyLimit` request extension overrides that limit.
Common text/binary/JSON/HTML/Form, status, HeaderMap, tuple and Result responses work
without compiling a source HTTP framework. Header arrays and
`response::Redirect` are also supported. Arrays replace repeated values; use
`HeaderMap::append` when repetition is intentional. Invalid headers or redirect
locations produce the source-compatible plain-text 500 response. Forms preserve source behavior: GET
reads the query; other methods, including HEAD, read the bounded body. Extensions
are host-local values installed before typed extraction; they do not cross the ABI.
Router middleware and compatible Tower layers can install host extensions.

`engine_web::middleware` provides host service middleware: `from_fn`,
`from_fn_with_state`, `Next`, `map_response`, and `handler_service` for binding
state to a typed handler. Use its re-exported `Layer` trait to wrap a service,
then register it with `on_service`, `any_service`, `fallback_service` or
`nest_service`. Request and response extensions stay in the host chain; bodies
remain streaming. These layers apply only to that service. They do not intercept
native 404/405 responses or wrap the whole router. The sole added dependency is
`tower-layer` (an interface crate); service-local layers need no new engine artifact.

`Router::layer(middleware::from_fn(...))?`, `from_fn_with_state` and
`map_response` apply async middleware to existing routes and their fallback,
including native 404/405 responses. Routes added afterwards are not wrapped.
Nested routing, typed state, headers and host extensions survive the continuation;
bodies remain streaming. `RequestMetadata` is refreshed from native routing at
each callback. Router layers require a rebuilt engine with Tower bridge support;
older artifacts reject construction. `Router::route_layer` wraps matched paths
(including their 405) and leaves the 404 fallback alone. `MethodRouter::layer`
wraps its existing methods and fallback, while `MethodRouter::route_layer` wraps
only existing matched methods, leaving 405 responses alone. Later-added routes
or methods are outside these layers. Empty route-layer applications are rejected
at construction. All four APIs accept Tower `Layer<engine_web::Route>` with a
cloneable, Send + Sync, infallible service returning a streaming byte body.
This includes compatible third-party layers and the owned async helpers above.
The engine constructs the service per native route and clones it per request,
awaiting readiness on that same clone. Pending typed state delays construction
until binding. Custom layers need only the re-exported `Layer` and `Service`
contracts; no Tower utilities are added to the host dependency graph.
Layers tied to a backend-specific body or runtime still need adaptation.

`Path<T>` reads already-decoded engine captures using source-compatible scalar,
tuple, struct, map, sequence and unit-enum semantics. Invalid UTF-8 and parsing
errors retain the existing rejection status and text.

Tower services can register through `MethodRouter::on_service`,
`MethodRouter::any_service`, `Router::fallback_service` and `Router::nest_service`.
Implement the reexported
`engine_web::Service<Request>` trait with infallible response errors, or adapt
errors into responses first. Each request waits for readiness and calls the same
service clone; response bodies remain lazy. This adds only `tower-service` to the
consumer graph, without Tower utilities or a Tokio runtime.

Nested services receive a prefix-stripped URI while `RequestMetadata` retains the
original URI and parent captures. Mounting at `/` is rejected; use a fallback
service for root dispatch. `nest_service` requires a rebuilt engine with the
`router_nest_service` operation; older libraries return a construction error.

`engine_web::sse::{Sse, Event, EventDataWriter, KeepAlive}` supports lazy SSE
responses, JSON events and opt-in idle heartbeats using engine timers. It is
included in `engine-web`; do not enable the source backend's `sse` feature for
an engine-only consumer. Ready events, errors and completion take priority over
heartbeats. Keepalive timers start at response conversion inside an engine
runtime; IDs, subscriptions, authorization and replay remain application-owned.

`engine_web::multipart::{Multipart, OwnedMultipart}` parses uploads inside the
engine. Borrowed fields enforce exclusivity at compile time; owned fields enforce
it at runtime and may outlive their reader. Metadata and streamed chunks cross
the body ABI; only `bytes` and `text` collect fields. The default total request
limit is 2 MiB, overridden with a `BodyLimit` extension. Multipart requires an
updated engine artifact with multipart operations; older artifacts return an
extraction error. No multipart parser dependency is added to consumers.

On Unix, `engine_web::unix::{bind, serve, UnixListener}` binds and serves named
Unix sockets inside the engine. Paths retain non-UTF-8 bytes; existing paths are
never removed or replaced. Applications own permissions, stale-socket handling
and cleanup after shutdown. Serving drains active responses on explicit shutdown
and reports no TCP peer address for Unix requests. This requires an engine with
Unix listener operations. Externally supplied listener file descriptors are not
supported by this adapter yet.

`engine_web::unix::UnixClient::new()?` creates an engine-owned HTTP/1 connection
pool shared by clones. `request(socket_path, request)` streams both bodies and
trailers, retaining methods, paths/queries, headers and HTTP versions. It matches
the source client's nonempty UTF-8 socket-path and origin-form path requirements.
Errors distinguish invalid requests from transport failures. Callers own deadlines
and forwarding policy; arbitrary Rust extensions and protocol upgrades do not
cross this ABI. The client requires a newly built engine; older artifacts reject
client creation.

For applications retaining Tokio-backed database or client libraries, optional
`engine-tokio` captures their runtime at handler/layer registration and restores
it while polling and destroying callbacks and streaming producers. Keep that
application runtime alive until HTTP drains. `runtime::Runtime::with_current`
constructs engine resources from an external executor; `Runtime::scope` polls a
future there with the engine context. Use a multithread engine runtime so its
I/O drivers run independently; this does not drive a current-thread engine.

`engine-tracing` provides `engine_web::tracing` with the same observer and lazy
body lifecycle as the source facade. Spans/subscribers and `correlation-core`
request scopes survive callbacks without compiling Axum. `correlation-core`
uses executor-independent task-local scopes and does not compile Tokio.
`Router::fallback_static_dir(path)?` serves trusted disk roots inside the engine,
including GET/HEAD, ranges, conditionals and directory redirects. These APIs
require a rebuilt engine with native operation context and static-directory support.

This API is not yet interchangeable with `web`: TLS serving
and WebSocket
adapters still need migration. The existing `web` API and default backend remain
source-based. ScT's production server entry points now use the engine on its
local development branch, with verified HTTP/database/shutdown behavior and a
working local Docker image. The subsequent runtime migration removes Tokio,
SQLx, Reqwest, Axum and Hyper from the production server graph. See the
[runtime migration evidence](docs/migration-status.md#sct-runtime-migration--2026-10-06).
The optional `oidc` feature also moves discovery, authorization-code exchange and
ID-token verification into the engine; applications retain login-flow storage
and sessions. See [the OIDC contract](docs/engine-application-runtime.md#openid-connect).
The canary has not been published or deployed.

The engine contains TCP HTTP/1 and HTTP/2 transport, streaming bodies and
basic route assembly. The standalone fixture's `http` binary exercises this
through internal bindings with the same 22-package consumer graph. Adapting the
public `Router`, middleware, protocol and listener APIs remains in progress.

## Owned HTTP API

`simple-server` pins its internal Axum implementation to **0.8.9**. The public
crates.io package is named `lelloman-simple-server`; keep the `simple-server`
dependency alias to preserve existing `simple_server` imports:

```toml
[dependencies]
simple-server = { package = "lelloman-simple-server", version = "=0.1.7", features = ["web", "ws", "multipart"] }
```

Cargo downloads source and compiles it using your selected features. No sibling
checkout, Fucina credentials, or private network access is required. See
[publishing and consuming](docs/publishing.md) for release instructions.


```rust
use simple_server::web::{Router, routing::get};

let app: Router = Router::new().route("/health", get(|| async { "ok" }));
```

The default `http` feature enables the internal HTTP backend. `web` adds owned
routing and serving; `ws`, `multipart`, `macros`, and `http2` enable optional
capabilities. With `default-features = false`, the backend is absent unless an
HTTP feature is selected. No feature starts a runtime or installs signals.

For coordinated development, a consumer may instead use a sibling path dependency
(for example, `path = "../../simple-server"` from a nested server crate). Such a
consumer must include the sibling in CI and Docker build contexts and record the
reviewed `simple-server` commit used by those builds. A path dependency's Cargo
lockfile does not pin the sibling's source revision.

Independent service lockfiles can select different transitive dependency
versions. Use a reviewed shared-library revision for each service build.

## Initial scope

The optional [`web` module](docs/web-core.md) provides shared routers, function
handlers, state/path/query/JSON extraction and response/body types. Pezzottify's
embedding endpoints were the first canary. The completed service migrations use
owned routers directly; the temporary `web-compat` feature has been removed.

The optional [`extract` module](docs/request-extraction.md) provides a shared
request-extraction trait and `Extract<T>` handler arguments. Applications own
required/optional policies and rejection responses; the owned `web` handler
layer adapts them to the internal transport without exposing backend traits.

Lifecycle, logging, request correlation and HTTP tracing helpers are available now.

[Step 03: observability](docs/step-03-observability.md) is split into independently
adoptable logging setup (03a), request correlation (03b), and HTTP tracing (03c).
[03a: logging setup](docs/step-03a-logging.md) is implemented, with verified
adoption and compatibility exceptions in the [migration matrix](docs/migration-status.html).
[03b: request correlation](docs/step-03b-correlation.md) is implemented;
[03c: HTTP tracing](docs/step-03c-http-tracing.md) provides optional safe request
spans and response-body lifecycle events via `http-tracing`. Consumer adoption is
tracked separately for each module.

- Listener setup, shutdown signals, graceful shutdown, and shutdown deadlines.
- Structured logging, request IDs, and HTTP tracing.
- Health and readiness endpoint plumbing with application-provided checks.
- Optional rate limiting with application-defined keys and route placement.

The optional `database-sqlite` feature provides an explicit, driver-independent
SQLite connection policy and live-setting verification; see
[Step 07a](docs/step-07a-database-connections.md). The independent
`database-migrations` feature provides a read-only preflight report over an
application's existing migration ledger; see [Step 07b](docs/step-07b-database-migrations.md).
The optional `database-sqlite-schema` feature adds schema descriptions, inspectable
creation plans and explicit validation reports; the published 0.1.1 adds
AUTOINCREMENT, FTS5 and trigger creation through an additive API; see [Step 07e](docs/step-07e-sqlite-schema.md).
Backup helpers remain future work shaped by real service integrations.
Authentication and authorization share the optional `auth` module.

Applications own their routes, state, configuration loading, database setup,
authorization rules, background jobs, and domain logic. Capabilities should be
optional, configuration explicit, and dependency upgrades independently adopted.

See [the design outline](docs/design.md) for boundaries and the adoption plan.
The [Step 02 lifecycle contract](docs/step-02-lifecycle.md) describes the scope,
shutdown behavior, validation requirements, and remaining adoption work.

## CORS

Enable the optional `cors` feature for explicit `CorsConfig` policies. It works
without Axum or default features. No cross-origin permissions are enabled by
default; applications choose origins, credentials, headers and layer placement.
`build()` rejects invalid wildcard/credential combinations before serving. See
the [04c contract](docs/step-04c-cors.md).

## Response headers

Enable the optional `response-headers` feature for explicit `insert_if_absent`,
`replace`, and `merge_vary` operations. This module also works with default
features disabled and has no Axum dependency. Applications own cache policy,
header values and route placement; the helpers never access response bodies.
See the [04b contract](docs/step-04b-response-headers.md).

## Lifecycle

Enable `lifecycle` explicitly. Core coordination also works with
`default-features = false, features = ["lifecycle"]`; HTTP adapters require both
`http` and `lifecycle`. Applications retain their Tokio runtime and `main()`.

```rust,no_run
use std::{io, time::Duration};
use simple_server::{http, web::{self, Router, routing::get}};
use simple_server::lifecycle::{Lifecycle, ShutdownOptions, Signals};

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signals = Signals::install()?;
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(30),
    });
    let listener = http::bind("127.0.0.1:3000").await?;
    let app = Router::new().route("/", get(|| async { "hello" }));
    lifecycle.service("http", web::serve(listener, app, lifecycle.shutdown()))?;
    lifecycle.run(signals.wait(), async { Ok::<_, io::Error>(()) }).await?;
    Ok(())
}
```

See the runnable [example](examples/lifecycle.rs). `Shutdown::request()` also
triggers shutdown, and callers can supply their own trigger future instead of
OS signals. Standalone `Shutdown` and `web::serve` do not require the coordinator.
Use `web::serve_with_connect_info` when handlers need the direct TCP peer address.

The coordinator drains registered futures concurrently, then polls application
cleanup with the remaining budget. Service failures, early service exits, and
timeouts return `LifecycleError` with a detailed report. Zero permits no draining
wait. Panics propagate; dropping the coordinator does not run cleanup.

Deadlines bound cooperative waiting. They do not forcibly stop detached tasks,
blocking work, or upgraded WebSocket connections. Those require application-owned
cancellation and tracking. Signal handlers are process-wide and installed only
through `Signals::install()`; dropping them does not restore default OS behavior.

## Development

Run the complete check sequence with `bash scripts/check`, or run individual
checks:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features --features lifecycle
cargo test --features lifecycle
cargo check --no-default-features
cargo doc --no-deps --all-features
```

The crate is imported as `simple_server`. Pezzottify and Favzetto are the first
design consumers; dependency centralization proceeds service by service.

## Repository

Fucina is the canonical repository. This project's public GitHub repository is
a one-way publication mirror; development changes are integrated in Fucina.

## Request correlation (Step 03b)

Enable the optional `correlation` feature for generated request IDs, validated
caller IDs by explicit opt-in, request extensions, scoped access, and response
headers. It does not require shared logging or lifecycle setup. Applications
retain error bodies, tracing spans and audit policy. See the
[correlation contract and usage](docs/step-03b-correlation.md).

## Logging (Step 03a)

Enable `logging` explicitly; it works without default features or a runtime.
Applications supply configuration and call the initializer during startup:

```rust
use simple_server::logging::{LogFormat, LoggingOptions, try_init};

let mut options = LoggingOptions::new("info,my_service=debug");
options.format = LogFormat::Json;
try_init(options)?;
```

Defaults are text on stderr, automatic terminal styling, and visible targets.
Compact formatting and optional close/full span events are also available.
Use `try_init_reloadable(options)` when runtime filter changes are needed; its
cloneable handle provides `set_filter` and `current_filter`. Formatting and output
remain fixed, and invalid updates leave the active filter unchanged.
Invalid filters and repeated global initialization return errors. The module
neither installs a `log` facade bridge nor reads configuration from the environment.
See the [03a contract](docs/step-03a-logging.md) and
[standalone example](examples/logging.rs) for output and compatibility details.

## Body limits (Step 04a)

Enable `body-limit` for `simple_server::body_limit::BodyLimit::max(bytes)` and
apply it explicitly to the intended routes. It preserves existing extractor
limits and rejection responses without requiring lifecycle or observability.
Raw request-body readers are not automatically limited; multipart file/part
policies remain application-owned. See the [contract](docs/step-04a-body-limits.md).

## Health and readiness

The optional `health` feature runs application-owned checks without enabling
Axum or a runtime. See the [Step 05 contract](docs/step-05-health.md).
`Probe::liveness()` checks no dependencies; readiness requires an explicit check:

```rust
use simple_server::health::{Check, Probe};

let readiness = Probe::readiness(Check::new("database", || async {
    // Run the application's real dependency check here.
    Ok::<(), std::io::Error>(())
}));
// readiness.run().await returns the first original error and its check name.
// readiness.endpoint(render) exposes a Tower service using your HTTP response.
```

Checks run in order on every request and stop at the first error. Applications
retain response schemas, status codes, timeouts and lifecycle policy. Mount the
endpoint behind GET/HEAD routing and existing access controls. The canary uses
`get_service` through the transitional router; the health API itself exposes no
Axum types.

[Step 06](docs/step-06-background-tasks.md) adds optional task ownership through
`tasks`, bounded scheduling through `task-scheduling`, and optional budgets, retries,
circuit breakers and control snapshots through `task-policies`. All work without HTTP
when default features are disabled. Consumer adoption is tracked independently from library availability.


Applications retaining their own durable scheduler can use
`task_scheduling::ExecutionCapacity` for shared global/resource-pool limits,
`Schedule` for recurrence, and the task/policy primitives independently. Keep
execution permits until blocking work actually finishes, even after cancellation.

For live cron controls without handing over execution, use `CronRegistry`.
It supports registration/replacement, enable/disable, removal, inspection and
per-entry skip/catch-up policies. Applications drive bounded `poll_due` batches
or select on the cancellation-safe `next_due()` future with their own admin
channel and shutdown. Manual execution, concurrency and persistence remain
application-owned. See [the dynamic cron example](examples/dynamic_cron.rs).

## Authentication and authorization

Enable `auth` for application-owned identity verification and access checks in one
module. `Access` and `AsyncAccess` support synchronous and asynchronous flows;
`HeaderCredential` provides explicit parsing rules and `AuthLayer` supplies a
body-preserving Tower gate with application-owned failure responses. The feature
works without default features, Axum or Tokio. See the [auth contract](docs/step-08-auth.md)
for ownership boundaries, provider integration and verification evidence.

Cookie/header authentication can use `auth::AuthLayer::credentials` with ordered
`CredentialSources`, explicit required/optional mode and an application verifier.
Verified `Identity` values retain their credential source. See the
[cookie/header contract](docs/step-08-auth.md#cookie-and-header-credentials--2026-09-25)
for fallback, parsing and error behavior. Optional `auth-cookies` adds decoded
cookie compatibility and cookie value types. `AuthLayer::authenticate` supports
existing lazy extraction boundaries without a global middleware change.

## Rate limiting

The optional `rate-limit` feature provides explicit replenishing and fixed-window
budgets, outcome-driven cooldown counters, keyed stores, rate/concurrency admission
and ordered sync/async policy callbacks. An HTTP layer preserves streaming and
holds admission guards until completion or drop. No Axum, Tokio, database or auth
provider is required; services own identity, proxy trust, route policy and errors.
Caller-owned grouped windows, calendar counters, persisted polling gates and
rolling-window/snapshot policies preserve durable service accounting. The separate
`rate-limit-async` feature opts into Tokio FIFO admission with delayed permit
release; it preserves burst-and-hold pacing without changing the base feature.
See the [Step 10 contract](docs/step-10-rate-limiting.md).

With `web` + `ws`, [`web::ws`](docs/web-core.md#owned-websockets) supplies owned
WebSocket upgrades, sockets, messages, close frames and errors, including split
read/write streams, subprotocol negotiation and configurable transport limits.
The temporary compatibility upgrade API has been removed.

The optional `sse` feature enables `web::sse::{Sse, Event, KeepAlive}` and `web`.
It supplies owned event/JSON builders and idle
keepalives over lazy, fallible streams, preserving disconnect cancellation.
See the [SSE contract](docs/step-11-sse.md) for framing, defaults and migration.

The optional `test-harness` feature provides `testing::TestServer` for owned-router
in-process tests and real loopback fixtures, bounded HTTP requests/responses,
assertions and multipart uploads. `test-harness-ws` adds a client using owned
WebSocket messages. Enable these in dev-dependencies; see the
[test harness contract](docs/test-harness.md).

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Optional disk static files: enable `static-files` and use
`web::static_files::{StaticDir, StaticFile}` for directory/single-file serving,
explicit SPA/error-page fallback and opt-in precompressed assets. See
[the static-file contract](docs/static-files.md).

Optional mutable HTTP cookies: enable `cookies` and use
`web::cookies::{Cookies, CookieManagerLayer, Cookie, SameSite}`. The shared jar
tracks request mutations and appends response deltas; cookie attributes and
session/CSRF policy remain application-owned. See [the cookie contract](docs/cookies.md).
Available starting with release 0.1.4.

Optional streaming gzip responses: enable `compression` and explicitly install
`web::compression::CompressionLayer`. Owned service/body boundaries support
gzip toggling, quality, minimum known size and custom metadata predicates;
encoding negotiation and stream cancellation remain lazy. See the
[compression contract](docs/compression.md).

Unix domain socket serving and streaming HTTP clients are available through the
opt-in `unix-http` feature on Unix platforms. See [Unix HTTP transport](docs/unix-http.md)
for lifecycle, path, header and streaming contracts.

## SQLite backup and synchronous execution

The optional `database-sqlite-backup` feature coordinates explicit checkpoint
preparation or consistent staged copies, verification, no-replace publication
and failure cleanup through application-owned sync/async adapters. See the
[07c contract](docs/step-07c-sqlite-backup.md).

The independent `database-blocking` feature provides dedicated bounded worker
threads with configurable weighted priorities, queue budgets and lane limits.
Sync and async callers share one scheduler. Runtime timeout does not interrupt
native operations or release their capacity early; close/drain observes actual
completion. See the [07d contract](docs/step-07d-database-blocking.md).

Both APIs are new in 0.1.7. Consumer adoption is tracked independently.

### Engine-owned application runtime and storage

Use `#[simple_server::main]` for the service entry point, `runtime` for tasks,
`time` for timers, `engine-lifecycle` for shutdown and `engine-tasks` for owned
workers. An application using only these interfaces needs no Tokio dependency.

`postgres-client` provides `database::postgres`: typed parameters, owned rows,
pools, transactions and forward migrations executed by SQLx inside the engine.
ScT keeps its SQL and migration definitions; the driver stays prebuilt. Supported
parameters/columns include text, integers, booleans, finite floating-point numbers,
byte arrays, text arrays and JSON. SQL NULL and JSON null are distinct parameters.
Unsupported column types can be present in a row but return a decoding error
when accessed. Dropped transactions roll back, including when the host uses an
external executor. Migration execution retains native SQLx locking/checksums.
The filename loader supports forward numbered `.sql` migrations only.

`engine-io` exposes futures I/O traits, asynchronous files scheduled on the
engine's blocking workers, and executor-independent synchronization primitives.
`process::ManagedCommand` (with `process,engine-io`) supports explicit stdio,
kill-on-drop and child reaping; `process::Command` remains the native buffered
output interface. `client` supports streaming responses and connection/read/total
timeouts, redirects disabled explicitly, and HTTPS-only policy.

The optional `zstd` feature provides synchronous buffer compression/decompression
through the engine, including an explicit decoded-output limit. No host Zstd
crate or runtime is needed. It requires the Zstd-enabled engine source pinned by
the consumer; older published engines do not implement these commands.
See [codec semantics and reuse scope](docs/engine-application-runtime.md#zstd-buffer-codecs).

For independent Tokio-based **test fixtures**, `#[simple_server::test(host_runtime = true)]`
requires Tokio as a dev-dependency and polls the test in an engine scope. Spawned
fixture tasks must explicitly carry `Runtime::try_current()?.scope(future)`.
Production entry points should use the engine directly, not this test bridge.
