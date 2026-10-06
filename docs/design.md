# Design direction

This document records the agreed end goal and design direction, not a finalized
API specification.

## End goal: completely abstract Axum away

In-scope consumer services use only `simple-server`'s public HTTP
interfaces. Axum may remain the internal implementation, but consumers must not
need to import, name, implement, or understand Axum APIs to build their services.
This includes routing, handlers, state, extractors, responses, errors, middleware,
multipart uploads, streaming, and WebSockets wherever those capabilities are used.

The abstraction is complete when:

- Consumer production code and tests no longer use `simple_server::axum`, direct
  Axum dependencies, or Axum-specific companion APIs.
- Public interfaces expose no Axum types, traits, trait bounds, or errors,
  including through aliases or re-exports.
- Existing service behavior is preserved and verified through the shared APIs.
- The transitional Axum re-export and any temporary Axum escape hatches are
  removed. Missing capabilities are addressed by extending the shared API.

Dependency centralization and completion of the infrastructure modules were
intermediate milestones. The final audit covers the 16 migrated service branches.
Quentin Torrentino remains explicitly excluded; its older checkout still relies
on the removed re-export and must migrate before using this revision. The
abstraction does not require replacing Axum internally.

The [Step 02 lifecycle contract](step-02-lifecycle.md) defines the first capability
extraction, its implemented API, shutdown contract, and pilot acceptance checks.

[Step 03: observability](step-03-observability.md) follows with three independent
modules: logging setup (03a), request correlation (03b), and HTTP tracing (03c).
The [03a contract](step-03a-logging.md) describes the implemented logging module and
Favzetto pilot. The [03b contract](step-03b-correlation.md) defines request IDs
and the Crumbles compatibility pilot. The [03c contract](step-03c-http-tracing.md)
defines request spans and response-body lifecycle events.

[Step 04](step-04-http-policy-assessment.md) separates body limits (04a), response
headers (04b) and CORS configuration (04c). The implemented
[04a contract](step-04a-body-limits.md) preserves extractor-limit semantics.
The [04b contract](step-04b-response-headers.md) defines framework-independent
header defaults, replacement and lossless Vary merging.
The [04c contract](step-04c-cors.md) provides optional CORS configuration with
framework-independent public types and explicit policy.

[Step 05: health and readiness](step-05-health.md) provides optional ordered
application checks and a framework-independent endpoint adapter. Applications
retain dependency policy and response contracts.

## Next milestones (updated 2026-10-04)

All 17 active services have completed their applicable implemented migration
steps through HTTP abstraction and SQLite 07a/07b/07e. Quentin Torrentino remains
excluded. Optional modules are adopted only where existing behavior requires
and matches them; drivers, SQL, domain repositories and transaction ownership
remain application-owned. See the [current matrix](migration-status.html) and
[Torrentino completion](migration-status.md#torrentino-remaining-modules-complete--2026-10-04).

The next increments are implemented and published in crates.io 0.1.7:

1. **07c: backup coordination.** Independent checkpoint-preparation reports and
   guarded staged copying, verification, atomic no-replace publication and
   explicit cleanup through sync/async adapters. The [contract](step-07c-sqlite-backup.md)
   preserves the distinct Pezzottify checkpoint and Crumbles consistent-copy
   strategies. No cross-file atomic snapshot or automatic restore is implied.
2. **07d: bounded synchronous execution.** Dedicated workers with configurable
   weighted priorities, queues, lane limits, deadlines, cancellation and drain.
   The [contract](step-07d-database-blocking.md) retains running capacity after a
   caller timeout and shares scheduling between async and sync callers.

Next: review these APIs against production adapters, canary backup coordination
against Pezzottify and Crumbles, then
canary blocking execution against Pezzottify's existing executor. Consumer
adoption of 07c/07d remains Pending assessment/canary; no service behavior changed
in this library implementation. Future rollout follows the
[consumer workflow](consumer-migration-workflow.md). Original step identifiers
remain stable; 07e was delivered before 07c/07d.

## Composition

Axum remains the internal HTTP implementation. The long-term product API hides
Axum types, traits, extractors, and errors. When a service requires an additional
capability, extend the public API and adapt it to Axum internally.

Migration began by centralizing the dependency and temporarily exposing Axum.
Shared capabilities then replaced those APIs across the 16 in-scope services.

Keep modules independently usable. A service should not need a database,
authentication provider, or background worker to use server lifecycle helpers.
No module requires handing over control of `main()`. An optional lifecycle
coordinator will compose the same public building blocks available to products.

## Responsibilities

| Shared foundation | Application |
| --- | --- |
| Listener setup and shutdown coordination | Startup dependencies and cleanup |
| Logging setup, request IDs, HTTP tracing | Domain events and audit semantics |
| Health and readiness endpoint plumbing | Readiness conditions and dependency checks |
| Optional metrics and middleware helpers | Routes, state, and business behavior |
| Rate-limit policies, key extraction, and middleware | Limit scope, trusted proxies, keys, and storage choice |
| Optional database setup and migration helpers | Database library, schema, queries, and transactions |
| Task ownership, scheduling and execution policies | Background jobs, durable claims and recovery |
| Optional authentication and authorization composition | Identity providers, permission models, and policy |

Shutdown deadlines, long-lived requests, and worker cleanup need explicit
semantics. Readiness must reflect application conditions; process liveness alone
does not establish readiness. Middleware should remain configurable for routes
such as uploads, streaming responses, and WebSockets.

## Adoption

1. Centralize Axum on the exact 0.8.9 pin, migrating services individually from
   their existing 0.7/0.8 dependencies. Check companion crates, feature needs,
   route semantics, WebSockets, and existing tests for each service.
2. Use Pezzottify and Favzetto as the first design consumers. Pezzottify has
   custom rusqlite stores and more complex authentication/workers; Favzetto has
   a SQLx SQLite pool and API-key authentication.
3. Extract lifecycle and entry-point setup, retaining product ownership of
   `main()`. Apply each shared capability incrementally across services.
4. Adopt observability through 03a, 03b, and 03c independently, then follow with
   further HTTP support, health, background tasks, authentication, authorization,
   and rate limiting as requirements are validated. Database helpers are deferred
   until after the consumer Axum-removal milestone.
5. Replace all transitional Axum usage with product-facing interfaces, verify
   the end-goal criteria above across consumers, and remove the Axum re-export.

Version the library so each product can upgrade independently. Repository
creation does not imply a release, a finalized licensing decision, or a
workspace-wide migration.

[Step 06: background tasks](step-06-background-tasks.md) separates task ownership,
scheduling and execution policies. Durable storage and recovery remain application-owned.

[Steps 08/09: auth](step-08-auth.md) combine identity verification and access checks
in one optional, Axum-independent module with application-owned providers and policy.

[Step 10: rate limiting](step-10-rate-limiting.md) supplies optional budgets,
outcome counters, bounded storage and streaming-preserving HTTP admission.


**Step 11 — Routing / HTTP core:** the [shared foundation](web-core.md) is implemented.
Pezzottify was the initial canary; all 16 in-scope services now use owned
routing, protocol, middleware, serving and test interfaces. The transitional
Axum re-export has been removed. Quentin Torrentino remains excluded.

## Shared-engine HTTP boundary (development)

The unpublished engine contains TCP binding, HTTP/1 and HTTP/2 serving, route
assembly and asynchronous host-handler dispatch. These internal bindings are
exercised by `tests/fixtures/engine_consumer/src/bin/http.rs`; the public web API
still uses the source backend until the remaining adapters have feature parity.

Requests cross the C ABI as framed metadata: method, URI, HTTP version, binary
header values, peer address, matched/original paths, decoded path parameters and
a borrowed body ID. No body is collected during dispatch. Path extraction
failures include their original rejection status and message.

| Resource | Ownership |
| --- | --- |
| Listener (kind 7) | Binding returns an owned registration. Constructing a serve operation consumes it after validating the handler/router and shutdown callbacks. |
| Incoming body (kind 6) | Dispatch owns a temporary registration. `server_body_clone` creates an independent registration for a host body that must survive dispatch, including an echo response. Each frame is polled lazily under the shared body lock. |
| Host callback (kind 5) | The engine acquires a reference before the host releases its registration. `Reply::keep_alive` holds response-stream callbacks until the engine finishes interpreting the reply. |
| Router/method group (kinds 8/9) | Assembly returns new immutable registrations. Each owns references to its handlers. Releasing the original host callbacks does not invalidate the routes. |

Response body callbacks produce data, trailers, end-of-stream or error frames.
The engine polls one future at a time, preserves backpressure and drops pending
host futures on connection cancellation. Producer-owned reply buffers remain
alive through decoding; releasing the buffer before acquiring referenced
callbacks could invalidate them before the engine can use them.

The explicit shutdown callback stops acceptance and drains active responses.
Already-requested shutdown is checked before serving. Dropping the serve future
alone retains Axum's existing cancellation semantics and does not guarantee
termination of spawned connections; lifecycle integration must use explicit
shutdown and its application-owned deadline.

Current route operations support method registration, merge, nesting, fallback
and method fallbacks. Tower layer/service composition,
owned extractors, WebSocket/multipart/SSE adapters, TLS/Unix transport and test
harness migration are still pending. Internal wire commands are developmental;
no stable engine artifact has been published.

## Shared-engine lifecycle boundary (development)

`engine-lifecycle` supplies the `engine_lifecycle` coordinator without Tokio in
the consumer dependency graph. Source `lifecycle` and engine coordination share
the same algorithm and runtime-independent sticky shutdown token. Each module
selects its own clock and signal implementation; enabling both features does
not change source callers' runtime requirements. Services may borrow application
state. Draining and cleanup consume one budget beginning at shutdown, and
dropping coordination cancels its futures without running cleanup.

The appended `clock_now` and `clock_valid` ABI functions exchange signed seconds
plus normalized nanoseconds relative to a native process-local monotonic origin.
Rust `Instant` layouts never cross the boundary. A runtime pointer selects its
clock (including paused time); null selects the real clock. Timestamps must not
be persisted or compared across clock domains. `Sleep` and timeouts create their
deadline at construction; `sleep_until` and `timeout_at` accept absolute engine
deadlines. Interval/reset and standard-library Instant conversion are not yet
provided.

Explicit `shutdown_signals` registration returns resource kind 10. Its wait
operation returns interrupt or terminate and owns a reference until completion
or cancellation. Dropping the registration releases its receivers; this does
not restore process-wide default signal handlers. No signal registration is
installed merely by creating a runtime or coordinator. Full task supervisors,
the scheduler and public web execution still require migration at that checkpoint.

## Shared-engine task ownership (development)

`engine-tasks` provides `engine_tasks::TaskSet` and `WorkTracker` using the existing
engine task collection and clocks. Source `tasks` and engine ownership share one
algorithm, with runtime and clock adapters supplied by their parent modules.
Source callers retain standard-library Instant arguments and timestamps; engine
callers use the owned engine Instant, including paused time. Feature unification
does not change either module's runtime requirement.

Unconsumed completions occupy admission slots. A cancellation request records its
first reason independently of the actual task result. Draining closes admission,
prefers an already-ready completion over deadline expiration, and retains owned
work and collected outcomes if the wait times out or is cancelled. Blocking jobs
cannot be explicitly aborted. Dropping a set requests cooperative shutdown and
detaches unfinished tasks; it does not claim completion or stop execution.

External work tracking uses a sticky completion notification that fires only
after admission is closed and the last guard is dropped. Closing and acquiring
remain serialized by the state lock; waking happens outside that lock. An open
empty tracker never reports completion. No Tokio synchronization primitive or
new ABI entry is needed by the engine consumer. Full scheduling and public web
execution remain pending.

## Shared-engine scheduler (development)

`engine_scheduling` selects engine tasks, clocks and polling drivers while
sharing the existing scheduler, cron registry, capacity and selection algorithms
with source `task_scheduling`. It includes execution policies via the pure
`task-policy-core` feature. Existing `task-policies` continues to enable source
task ownership plus those policy definitions. When features are unified, policy
APIs become available on both schedulers; source execution stays on its runtime.

The wrapper retains Tokio channels, watch notifications, semaphores and selection
macros. These require no Tokio runtime. The normal/build consumer graph permits
only `sync`, `macros` and the implicit `tokio-macros` feature; it rejects Tokio
executor, time, networking and signal features. Source-facing modules explicitly
enable their former Tokio feature set through `source-runtime`. Applications
enabling source modules alongside engine scheduling still compile those source
runtime features.

Task timestamps are converted into the selected scheduler clock before budget
comparisons. Engine timestamps never convert through standard-library Instant.
Wall-clock cron and circuit-breaker behavior remains separate from monotonic
intervals, queue/runtime budgets and retries. Snapshots still contain control
state, not queued payloads or abandoned work. The engine builder currently lacks
a blocking-thread-limit option: the existing single-thread blocking-queue delay
test remains source-only, while engine blocking shutdown and runtime budgets
are covered by shared contracts. No ABI or native artifact change is required.

## Public engine HTTP core (development)

`engine_web` exposes the existing native transport through owned, fallible
`Router`, `MethodRouter`, `TcpListener`, `Request`, `Response` and `Body` APIs.
Handlers take one raw request and return a response. Application state is captured
by closures. This explicit module coexists with source `web`; enabling its feature
does not migrate existing web callers or switch their runtime. Handler registration
and serving use the application-owned engine runtime.

Route resources are immutable and reference-counted. Derived routers retain their
handlers independently of earlier registrations; cloning does not mutate either
route collection. Methods, merge, nesting and router/method fallbacks delegate to
the engine. Invalid registration returns an error; native panic containment keeps
the host alive. No resource ID or byte protocol is exposed by this public API.

Incoming metadata preserves method, URI, HTTP version, binary/duplicate headers,
peer address, matched/original paths, decoded path parameters and decoding errors.
An incoming body acquires an independent registration before the borrowed request
callback ends. A handler can return that body for a streaming echo. Each body frame
is polled lazily, retaining data/trailers/errors and backpressure. Outgoing response
callbacks remain alive until the engine acquires ownership, through `Reply`.
Response status, headers and body cross the boundary; arbitrary extensions and
explicit response-version overrides are not transported.

`serve` consumes its non-cloneable listener. Explicit shutdown stops acceptance
and drains active responses; an application deadline is separate. Dropping the
serve future alone retains the native server's cancellation semantics and does
not promise to stop all connections. Dropped client connections cancel pending
response work and release host bodies. Collection always requires a byte limit.

Tower middleware,
TLS/Unix listeners, protocol upgrades and the public test harness remain parity
work. No stable release or production adoption is implied. This checkpoint needs
no engine ABI or native implementation change.

## Typed engine handlers and extraction (development)

Engine typed registration uses `on_handler` with the group's missing state, `on_state` with
per-handler state, and `fallback_handler`. The raw request closure APIs retain
their parameter inference. Both backends instantiate the existing handler
algorithm, which extracts arguments left-to-right and permits only the final
argument to consume the body. Compile-fail examples guard this ordering and
the prohibition on multiple body consumers.

`FromRequestParts`, custom `Extract` and rejection responses are shared. The
runtime-independent HeaderMap/URI/Method and Result-capture implementations now
live with that contract instead of inside source `web`. Engine State/substate,
Query, JSON/optional JSON, raw query, matched path and direct TCP peer extraction
operate on the owned request boundary. Typed path extraction is described below.

String, Bytes, JSON and body-based Form extraction collect at most 2 MiB unless
the request has a `BodyLimit(usize)` extension. Raw requests and lazy bodies are unaffected. Missing
Content-Type alone makes optional JSON absent; declared invalid JSON still rejects.
MIME parsing and path-aware Serde errors preserve source rejection status, headers
and text. Body read failures map to 400; collection-limit failures map to 413.
JSON syntax failures map to 400, schema failures to 422 and unsupported content
types to 415. Head extraction failure returns before polling the body.

Common response conversion covers text/binary bodies, JSON, HTML, status codes,
HeaderMap, status/header tuples, Result and custom rejection responses. JSON
serialization failure returns a plain-text 500 response. Header arrays and
redirects are now implemented as described below.
Response extensions still remain host-side and do not cross the existing ABI.
The native engine and ABI do not change in this checkpoint.

## Engine forms and request extensions (development)

`Form<T>` uses GET queries and otherwise reads the bounded body, including for
HEAD. This follows the actual source backend, whose former wrapper comment
incorrectly described HEAD as reading the query. Deserialization errors retain
400 for GET/HEAD and 422 for other methods. The current content-type prefix check,
path-aware Serde diagnostics, repeated-field behavior and percent/plus decoding
match the source backend. Missing or unsupported content types return 415 without
polling the body. Form serialization emits URL-encoded responses or a plain-text
500 when serialization fails.

`Extension<T>` and `Option<Extension<T>>` clone typed values from the host request.
Missing required values preserve the source rejection exactly (including its
legacy Axum reference); only absence makes the optional form return None.
Request extensions are local Rust values, not wire data. A raw handler or custom
head extractor can install them before invoking typed extraction. This does not
provide Tower extension layers, engine-side extension storage or a generic router
middleware pipeline. No native code, ABI or dependency additions are required.

## Typed engine path extraction (development)

`engine_web::Path<T>` reads the owned `RequestMetadata` capture list. Matching and
percent decoding stay in the native engine; the host never decodes a second time
or changes literal plus signs into spaces. Captures retain their ordering across
nested routes and repeated names. Extraction borrows metadata, so multiple path
extractors can read the same request without consuming captures or the body.

The private Serde deserializer is adapted from Axum 0.8.9's MIT-licensed path
implementation; its complete copyright and permission notice are retained in
`src/engine_web/path_de.rs`. It uses existing Serde and owned strings, with no
Axum types or dependency added to the engine consumer. This includes scalars,
newtypes, tuples, structs, maps, sequences, unit enums and custom string visitors.
Unsupported shapes and wrong capture counts preserve source 500 responses;
invalid values and UTF-8 preserve 400 responses and exact diagnostics. The native
wire's raw UTF-8 error receives the typed Path `Invalid URL: ` prefix; missing
path metadata retains the source missing-parameter rejection.

All values and errors remain behind the existing HTTP/metadata boundary. No
native or ABI changes are needed. This closes the typed Path gap; Tower layers and protocol
adapters remain separate work.

## Generic engine router state (development)

`Router<S>` and `MethodRouter<S>` carry the state their typed handlers still need.
`with_state<S2>(state)` binds the existing handlers and returns a fallible router
whose newly added handlers may require `S2`; serving accepts only `Router<()>`.
The same model applies to nested and merged routes, typed fallbacks, method
fallbacks and `any_handler`. Handler-local `on_state` and previously bound method
groups retain their local values. Clone a pending router to bind independent
state values, without mutating any earlier clone.

State remains entirely in Rust host callbacks. Each pending route plan owns a
native validation resource plus a state-binding function. Validation resources
use inert placeholder callbacks for unbound typed handlers and never reach
`serve`. Construction checks paths, duplicate methods and merge/nesting conflicts
immediately. Binding materializes fresh callbacks and native routes with the
chosen state; fully bound plans reuse their native resource. Serving a unit-state
plan materializes any remaining unit-state handlers first. No native/ABI changes
or additional dependencies are required.

This introduces host-side storage for pending construction plans and additional
native construction during binding. It does not add per-request routing work or
claim a measured performance improvement. Failures propagate through io::Result;
partially constructed resources and captured state follow RAII ownership. Tower
layers/services and protocol adapters are still
separate work. The source backend's APIs and default execution are unchanged.

## Engine header arrays and redirects (development)

The remaining helpers exposed by the source response module now have engine
implementations: standalone header arrays, `(headers, response)`,
`(status, headers, response)` and Redirect. Json and Response are also publicly
available through `engine_web::response`, matching the source module paths.
Header arrays convert in order and replace earlier values. Conversion failure
discards the original response and any partial headers, drops the body without
polling it, and returns the source-compatible plain-text 500 response. An explicit
status in the three-element tuple applies only after successful header conversion;
a separate outer status wrapper retains its existing override behavior.

Redirect constructors retain their URI and 303/307/308 status. Location header
validation occurs during response conversion; invalid values produce a 500
rather than panic during construction. These helpers preserve source status,
headers and body behavior. Existing host-local response extensions and versions
are preserved by successful array conversion, but arbitrary extensions still do
not cross the native ABI and response-version overrides remain outside that wire
contract. No native/ABI or dependency changes are needed for these response helpers.
Router-wide layers and protocol response adapters remain
separate work.

## Engine method and fallback services (development)

`MethodRouter::on_service`, `MethodRouter::any_service` and
`Router::fallback_service` accept Clone + Send + Sync Tower services over owned
engine Requests. The runtime-independent Service trait is reexported from
`engine_web`; the feature enables only `tower-service`, not Tower utilities.
Services must expose Infallible call/readiness errors; application errors must
be converted into responses by the service or its wrappers. Response bodies can
be any Send HTTP body with Bytes data and errors convertible into BoxError.

Dispatch clones the service, waits with poll_fn for that clone's readiness, then
calls it without cloning again. This preserves reservations made in poll_ready.
Readiness is per dispatched request, not listener-level admission control. Dropping
a disconnected request drops its pending readiness wait or service call; bodies
are wrapped lazily and can stream the incoming body after the service returns.
Method matching, HEAD suppression and 405/Allow behavior remain in the native
router. Services can coexist with pending typed handlers and state binding.

No native/ABI changes are needed. The standalone HTTP consumer now runs its echo
through a service; its graph grows from 28 to 29 packages because of tower-service.
Router-wide Tower layers, Route service exposure and make-service
adapters remain separate work. The new registration methods do not claim those
capabilities or automatic compatibility with every runtime-dependent service.

## Engine nested services (development)

`Router::nest_service` mounts the same infallible, runtime-independent Tower
services as the method/fallback adapters. The native engine owns prefix matching
and URI stripping; the host owns service cloning, readiness, call futures and
lazy response bodies. Original URI and parent captures cross the existing request
metadata boundary. Native route validation runs immediately, including on routers
with pending typed state; state binding replays the mount without sharing state
between independently bound clones. Root mounting is rejected consistently with
the source router; use fallback_service for that case.

The additive `router_nest_service` wire operation requires a rebuilt engine.
C ABI signatures/layout and version remain unchanged, but an older engine returns
`unknown router command` during registration. Artifacts and consumers must be
updated together when adopting this method. No new dependencies are introduced;
the standalone HTTP consumer retains its 29-package normal/build graph and now
executes a mounted echo under a captured parent prefix. Router layers, Route
service exposure, make-service and protocol adapters remain separate work.

## Engine server-sent events (development)

`engine_web::sse` exposes Sse, Event, EventDataWriter, EventError and KeepAlive
without enabling the source backend's sse feature. The event encoder is adapted
from the pinned Axum 0.8.9 implementation with its MIT notice retained; it uses
only existing host dependencies. Multiline/chunked data, builder field order,
Unicode, JSON serialization errors, duplicate-field rejection and retry hints
retain source behavior. The source web::sse implementation is unchanged.

SSE responses set text/event-stream and no-cache, polling the application stream
lazily and forwarding each event as one body frame. Non-Unpin streams are supported
through a pinned box; no producer task is spawned. Producer failures propagate as
body errors, not synthetic SSE events or replacement HTTP statuses. Dropping the
body releases its stream and timer.

Keepalives remain opt-in: by default they emit an empty comment after 15 idle
seconds, with custom comments or complete events supported. Engine timers start
when the response is converted, requiring an engine runtime only when keepalive
is enabled. Ready events reset the deadline and win over due timers; errors and
completion also take priority. Paused engine-clock tests cover these rules.
Applications retain authorization, subscriptions, replay and reconnect IDs.

The existing streaming-body and timer ABI supports this adapter unchanged. The
standalone HTTP fixture now calls the SSE API and still has 29 normal/build
packages beyond the fixture and library, without Axum or Tokio. Router layers,
Route service exposure, make-service, WebSocket/multipart, TLS/Unix transport and
test-harness parity remain separate work. No production adoption is implied.

## Engine multipart uploads (development)

The engine-web feature now exposes borrowed Multipart/Field and owned
OwnedMultipart/OwnedField, plus MultipartError, under engine_web::multipart.
Multer 3.1.0 is compiled only into the engine, preserving its parsing and charset
behavior without adding downstream packages. The existing source-backed
multipart APIs are unchanged. Field metadata is host-owned; parsing state and
field buffers remain native. Parser errors preserve status and diagnostic text;
MultipartError::source is a host-owned diagnostic, not the native Rust error.
Borrowed limit errors retain Axum's “Request payload is too large” text while
owned limit errors retain the owned source adapter's parser diagnostic.

Requests export their lazy body through the existing callback mechanism. The
engine applies the total request limit (2 MiB by default, with host BodyLimit
overrides) before parsing. Fields expose name, filename, content type, headers,
Stream/chunk, bytes and charset-aware text. Only bytes/text collect complete
fields. File storage, filename validation and additional per-file limits remain
application-owned. The borrowed wrapper prevents overlapping fields at compile
time. Owned fields retain native state after their reader is dropped and reject
concurrent next-field calls until the previous field is dropped or consumed.

Parser resources use kind 11; field slots use kind 12. Slots are allocated
synchronously before awaiting next-field operations and held by host RAII guards.
Cancellation drops the slot registration and in-flight operation, so async
completion never creates an unclaimed resource ID. Native map entries are dropped
outside their locks because callback destruction may reenter resource release.
Tests cover cancellation at header, chunk and text stages, including switching
from a pending chunk to text, as well as dropped-field skipping and reader drop.

New resource/operation commands require a rebuilt engine artifact. C ABI
signatures and layout are unchanged; an old library cannot extract multipart and
returns HTTP 500 through the typed extractor's rejection path. The standalone
HTTP fixture demonstrates both that failure and successful binary upload with
the new library. Its normal/build graph remains 29 packages beyond fixture and
library, without Multer, Axum or Tokio. No runtime speedup is claimed. Router
layers, Route service exposure, make-service, WebSocket, TLS/Unix transport and
public test-harness parity remain pending.

## Engine Unix-socket serving (development)

On Unix targets, engine-web exposes unix::bind, unix::serve and a non-cloneable
UnixListener. Binding and acceptance occur inside the engine, so neither Tokio
nor a Unix HTTP transport dependency is added to the consumer. The host retains
the supplied PathBuf; raw path bytes cross the wire, preserving non-UTF-8 paths.
Bind failures preserve OS error codes. Binding never removes an existing file,
symlink or socket. Dropping a listener closes it but leaves the filesystem entry;
applications own permissions, stale-file handling and final cleanup.

The native listener registry uses resource kind 13. New server_unix_bind and
server_unix_serve commands share route dispatch, callback bodies and graceful
shutdown with TCP. TCP continues to attach SocketAddr metadata; Unix serving
leaves RequestMetadata::peer absent. Pre-requested shutdown accepts no queued
requests; active bodies drain on shutdown. Dropping the serve future alone does
not promise to terminate all spawned connections, matching the existing TCP
contract. Callers retain ownership of shutdown deadlines.

Tests compare Unix HTTP status, headers and bytes with source serving, excluding
only Date and header order. Native Unix tests also cover incremental request and
response streaming, trailers, duplicate headers, nested paths and captures,
non-UTF-8 bind paths, existing-file/symlink preservation, listener drop,
pre-requested shutdown, graceful drain and disconnect-driven body cleanup. The
standalone engine HTTP consumer now serves over both TCP and Unix.

C ABI signatures and layout are unchanged, but Unix serving requires a rebuilt
engine artifact; an older library rejects the bind operation. The normal/build
HTTP consumer graph stays at 29 packages beyond the library and fixture. Unix
client requests, importing externally bound descriptors, peer credentials,
WebSocket upgrades, TLS, router layers and public test-harness parity remain
separate work. This checkpoint does not imply production consumer adoption.

## Engine Unix HTTP client (development)

UnixClient::new creates a native HTTP/1 pool and returns io::Result because engine
resource creation is fallible. Clones share resource kind 14 and the same pool.
Hyperlocal 0.9.1 and Hyper's legacy client run only inside the engine. Requests
select the socket independently of the original URI authority and retain Host,
method, path/query, duplicate/binary headers, version and body. Nonempty UTF-8
socket paths and origin-form paths match the source client's input contract.
Request and response trailers remain observable through the body frame interface.
Received HTTP versions are decoded explicitly; this does not change the separate
server-side response-version override limitation.

Bodies remain lazy: the host exports its request body with a size hint, and the
engine installs the response stream into a body resource allocated before the
request starts. This reuses body kind 6 via server_body_slot. Preallocation and
host ownership avoid unclaimed native response registrations on cancellation.
The response remains readable after its client handle is dropped. Tests verify
streaming before upload completion, request cancellation, response producer drop,
late response errors, upload errors and pool reuse across cloned handles.

UnixHttpError distinguishes InvalidRequest from Transport and exposes a source.
Native failures become host-owned diagnostics; native Rust error identities and
arbitrary request/response extensions do not cross the ABI. Protocol upgrades,
application timeout/retry policy and forwarding-header policy are not supplied
by this adapter. Client construction itself fails explicitly on older artifacts.

The new commands require a rebuilt engine while leaving C ABI signatures/layout
unchanged. The standalone HTTP fixture now uses the public client as well as raw
Unix sockets. Its normal/build graph stays at 29 packages beyond fixture and
library, without Hyper, Hyperlocal, Axum or Tokio. Remaining gaps include TLS,
WebSocket, external listener descriptors, peer credentials, router layers,
Route/make-service exposure and public test-harness parity. No production
adoption or runtime speedup is claimed.
