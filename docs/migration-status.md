# Service migration status

## ScT engine canary — 2026-10-06

In progress from clean ScT master `9e118e6` and simple-server master `16203f8`.
Isolated branches: ScT `migration/shared-engine` and library
`implementation/sct-engine`. ScT uses real routing, custom correlation UUIDs,
header-time metrics/tracing, static files and graceful shutdown. SQLx/PostgreSQL,
Reqwest/OIDC and application Tokio remain application dependencies.

The library now supplies native static-directory fallback, source-free tracing,
`correlation-core`, explicit external-executor runtime scopes, and optional
`engine-tokio` context restoration. Context applies to polling AND destruction
of callback-owned values. The first canary exposed a stream-drop bug: a download
destructor could not schedule release of a PostgreSQL read pin outside its host
Tokio context, leaving tree cleanup stuck. An exact content-length/body-drop
regression and the previously failing ScT tree workflow now pass. Tracing
subscribers/spans and trusted correlation scopes propagate across callbacks;
untrusted request headers do not establish task-local context.

Baseline workspace tests pass; library baseline covers Tower, source tracing
and static files. Four new engine host-adapter contracts pass, as do strict ScT
workspace Clippy and its regular server HTTP tests. ScT database verification
and packaging/measurement are still being finalized; adoption is not yet marked
Done. Logs use `/tmp/sct-engine-*.log`.

Native debug artifact `/tmp/sct-shared-engine-native/debug/libsimple_server_engine.so`
has SHA256 `ce981a84b866028aae057483bce473732a93063b5eafee23d20e13fb7c451eec`.
The engine adds tower-http filesystem support and router_static_dir; native
operation polling/drop enters its own runtime. C ABI signatures/layout are
unchanged. Legacy engines must be rebuilt for this adapter set. The minimal
engine consumer still excludes source HTTP/runtime libraries; ScT intentionally
retains its application runtime and clients. No push, publication or deployment.

## Engine general Tower layers — 2026-10-06

The Tower-layer compatibility increment is implemented; the overall engine
remains **Partial**, production adoption **Pending**. Start: clean local master
`93b9d9f`; isolated branch `implementation/engine-tower-layers`, worktree
`target/worktrees/engine-tower-layers`. No consumer repository changes.

All four Router/MethodRouter layer APIs now accept general Tower layers over
opaque `engine_web::Route`, with the source facade's Clone/Send/Sync/infallible
service and streaming byte-body bounds. Native routes are resolved before layer
construction, so layer-created service state survives across requests and
connections. Each request awaits readiness on the exact service clone it calls.
Pending state validates scope with identity layers, avoiding user constructors
on placeholder handlers; actual construction occurs when state is bound.

Native extension snapshots preserve matching/peer metadata during forwarding.
A layer can replace the whole request or clear extensions and still call its
inner Route; removed metadata remains absent. Custom extensions remain host-owned.
The existing streaming/trailer/context lifecycle contracts apply to this bridge.
The standalone HTTP consumer now constructs a custom stateful Tower layer using
only the re-exported contracts and verifies repeated requests to a native 404.

Baseline: 20 middleware/scope contracts pass
(`/tmp/simple-server-tower-layers-baseline.log`). Seven new contracts pass
(`/tmp/simple-server-tower-layers-contracts.log`), covering per-route state,
independent state binding, readiness reservation and cancellation, full request
replacement, real tower-http gzip compression with a different streaming body,
constructor panic/resource cleanup, and source HTTP parity with an explicitly
state-bound source router. The source binding avoids comparing services rebuilt
per connection against persistent services. Initial middleware/scope/HTTP
regressions also passed 28 tests before the final eager-construction adjustment.

Full `scripts/check` passes (`/tmp/simple-server-tower-layers-full.log`):
workspace and native strict Clippy, source feature matrix, strict rustdoc, seven
all-feature/six engine-only Tower contracts, all middleware/scope/HTTP regressions,
seven standalone engine binaries, dependency guards and their regression tests,
C ABI smoke and five installer tests. The final HTTP fixture verifies persistent
custom-layer state without Tower utilities. Diff whitespace checks pass.

New local x86_64 debug artifact:
`/tmp/simple-server-tower-layers-engine/debug/libsimple_server_engine.so`, SHA256
`39e675dfa2bb16a0c3519fae31bba13a7d04824f5dc856a17e3f6f868d75635a`.
Build log: `/tmp/simple-server-tower-layers-build.log`. New wire commands are
router_tower_layer, router_tower_route_layer, method_tower_layer,
method_tower_route_layer, tower_route_clone, tower_extensions_clone and
tower_route_call; new resource kinds are 16 (Route) and 17 (extension snapshot).
C ABI signatures/layout are unchanged and legacy async-layer commands remain
available. Tower is added directly to the native manifest, already present in
its lockfile graph; host dependencies are unchanged.

The normal/build HTTP consumer graph is verified at 30 packages excluding the
fixture and library, with no Axum/Tokio/Hyper/Hyperlocal/Tower/Reqwest/SQLx/Rustls
(`/tmp/simple-server-tower-layers-graph.txt`). Against the previous layer-scopes
artifact, the new standalone fixture fails construction with `unknown method
router command`, before serving (`/tmp/simple-server-tower-layers-old-engine.log`;
exit 1). Tracker scripts and linked Markdown anchors pass validation.

Layers tied to a source body/runtime still require adaptation. Source-free
correlation/tracing/static-file integration, other recorded protocol/API gaps,
artifact release and production adoption remain pending. No new performance
claim, push, publication or deployment. Release/Bookworm distribution, ARM and
non-Unix targets are not retested.

Implementation `35018b5` was integrated by rebasing local master onto the
implementation branch. Original master remained clean at `93b9d9f`; no concurrent
commits or unrelated edits appeared. Ancestry and exact tested-tree equality
passed. Temporary branch/worktree cleanup follows integration of this evidence
update; no external repository was changed.

## Engine route and method layer scopes — 2026-10-06

Implementation remains **Partial**, production adoption **Pending**. Start:
clean local master `07b6179`; isolated branch `implementation/engine-layer-scopes`,
worktree `target/worktrees/engine-layer-scopes`. No consumer repository changes.

Added owned async Router::route_layer, MethodRouter::layer and
MethodRouter::route_layer. Router::route_layer leaves 404 alone but wraps a
matched path's 405; MethodRouter::route_layer leaves its 405 fallback alone too.
MethodRouter::layer includes the method fallback. Later-added routes/methods
are not wrapped. Native placement and the existing continuation bridge preserve
headers, streaming and host extensions; pending state replays the same scope.
Empty route-layer calls reject eagerly, including fallback-only method groups.
Arbitrary third-party Tower layers remain pending; no ScT adoption is claimed.

Baseline router middleware 8 and state 5 tests pass
(`/tmp/simple-server-layer-scopes-baseline.log`). Five scope contracts and those
thirteen regressions pass (`/tmp/simple-server-layer-scopes-contracts.log`).
Coverage includes real source HTTP comparisons for all three APIs, HEAD/custom
methods, 404/405/Allow, later registration, short-circuit authorization behavior,
nested captures, layer order, host extensions, independent pending-state binding
and invalid empty construction. The standalone HTTP fixture now executes all
three scope APIs together with router-wide and service-local middleware.

Full `scripts/check` passes (`/tmp/simple-server-layer-scopes-full.log`):
workspace/native strict Clippy, source feature matrix, rustdoc, five all-feature
and four engine-only scope contracts, all engine regressions, seven standalone
engine binaries, dependency guards and their regression tests, C ABI smoke and
five installer tests. Tracker scripts/anchors, graph count and diff whitespace
checks pass. Graph evidence: `/tmp/simple-server-layer-scopes-graph.txt`.

Against the previous router-middleware artifact, the updated standalone fixture
fails method route-layer construction with `unknown method router command`,
before serving (`/tmp/simple-server-layer-scopes-old-engine.log`).

New local x86_64 debug engine:
`/tmp/simple-server-layer-scopes-engine/debug/libsimple_server_engine.so`,
SHA256 `76e3a09b0113944fceb7bc9793676746f7f778324ec6083688a2ef7dfe26f8a1`.
Build log `/tmp/simple-server-layer-scopes-build.log`. New wire commands:
router_route_layer, method_layer, method_route_layer. Resource kinds and C ABI
layout/signatures are unchanged; host graph remains 30 packages without native
transport/runtime dependencies. No build/runtime speedup is claimed.
Release/Bookworm distribution, ARM and non-Unix targets are not retested.
No push, publication or deployment.

Implementation `cc38630` was integrated by rebasing local master onto the
implementation branch. Original master remained clean at `07b6179`; no concurrent
local commits or edits appeared. Ancestry and exact tested-tree equality passed.
Temporary branch/worktree cleanup follows integration of this evidence update.

## Engine router async middleware — 2026-10-06

Implementation remains **Partial**, production adoption **Pending**. Start:
clean local master `c4f5988`; isolated branch `implementation/engine-router-middleware`,
worktree `target/worktrees/engine-router-middleware`. No consumer repository changes.

Router::layer now accepts owned from_fn/from_fn_with_state/map_response layers.
Native route placement covers existing routes and fallback (including 404/405),
but not later-added routes. Native continuations retain routing state; guarded
host contexts preserve custom request/response extensions without serializing
Rust objects. RequestMetadata is refreshed from native routing at each callback.
Bodies and trailers stay streaming; short circuits, panics and cancellation
release owned context/continuation registrations. Pending state can be bound
independently after layers are added.

Arbitrary Tower layers, route_layer and method-router layers remain pending,
as do source-free correlation/tracing/static-file integration for ScT and other
previously recorded protocol/API gaps. This is no production migration.

Baseline: middleware 7, router state 5, HTTP 8 tests pass
(`/tmp/simple-server-router-middleware-baseline.log`). Initial routing contracts
and those regressions pass (`/tmp/simple-server-router-middleware-tests.log`).
Eight final router contracts pass
(`/tmp/simple-server-router-middleware-contracts.log`): nested routing, extensions,
layer ordering, state cloning, native errors/later route scope, real source HTTP
parity, incremental duplex/trailers, cancellation, concurrent isolation, panic
and short-circuit cleanup. Capture-release checks verify that completed servers
do not retain continuation-owned handlers.

Full `scripts/check` passes (`/tmp/simple-server-router-middleware-full.log`):
workspace/native strict Clippy, source feature matrix and rustdoc, eight
all-feature/seven engine-only router middleware contracts, all engine regressions,
seven standalone consumers, dependency guards, C ABI smoke and five installer
tests. Public module documentation is then refreshed and formatting/rustdoc
rechecked. Tracker scripts/anchors and the 30-package graph are validated.

Against the previous Unix-client engine, the standalone fixture rejects layer
construction with `unknown router command`, before binding/serving
(`/tmp/simple-server-router-middleware-old-engine.log`).

New local x86_64 debug artifact:
`/tmp/simple-server-router-middleware-engine/debug/libsimple_server_engine.so`,
SHA256 `9ba691d380b203642bc4f13790fd203c4be5d3946c90bdbd558518f54d0bd406`.
Build log: `/tmp/simple-server-router-middleware-build.log`. The wire adds
router_layer/middleware_next_clone/middleware_next and continuation resource 15;
C ABI signatures/layout stay unchanged. No Cargo dependencies added; standalone
HTTP graph stays at 30 packages without Axum/Tokio/Hyper/Tower utilities.
Release/Bookworm distribution, ARM and non-Unix targets are not retested.
No speedup claim, push, artifact publication or deployment.

Implementation `a06e394` was integrated by rebasing local master onto the
implementation branch. Original master remained clean at `c4f5988`; no concurrent
local commits or edits appeared. Ancestry and exact tested-tree equality passed.
Temporary branch/worktree cleanup follows integration of this evidence update.

## Engine host service middleware — 2026-10-06

Implementation remains **Partial**, production adoption **Pending**. Start:
clean local master `26c1b68`; isolated branch `implementation/engine-host-middleware`,
worktree `target/worktrees/engine-host-middleware`. No consumer repository changes.

The engine API now exposes from_fn/from_fn_with_state, Next, map_response and
handler_service. Typed middleware state and head extraction precede the request
and continuation; handler state is bound explicitly. Layers wrap existing host
services before on_service/any_service/fallback_service/nest_service registration.
Request/response extensions remain in one host callback. Short circuits skip
inner readiness; continued calls await readiness on the exact clone they call.
Streaming, trailers and late errors stay lazy; cancellation releases owned work.

This is not router-wide middleware. Tests explicitly verify native 404/405
responses are outside service-local layers. Router continuations, ScT's global
correlation/tracing middleware and source-free static-file integration remain
pending. TLS, WebSockets and the previously recorded remaining gaps are unchanged.
No production adoption or new compilation/performance claim is made.

Baseline engine services 6 and HTTP 8 tests pass
(`/tmp/simple-server-host-middleware-baseline.log`). Focused middleware tests 6,
service regressions 6 and HTTP regressions 8 pass
(`/tmp/simple-server-host-middleware-tests.log`); an additional body-producer
drop/cancellation contract is included in the full check run. Contracts cover
real HTTP routing metadata, independent handler/middleware state, request and
response extensions, layer order, extraction rejections, readiness, streaming
trailers/late errors, plus a source-backend middleware comparison.

Full `scripts/check` passes (`/tmp/simple-server-host-middleware-full.log`):
workspace/native strict Clippy, source feature matrix, rustdoc, seven all-feature
and six engine-only middleware contracts, all engine regressions, seven
standalone engine binaries, dependency guards and their five regression tests,
C ABI smoke and five artifact installer tests. The standalone HTTP middleware
route passes using the previously verified native artifact. Tracker scripts and
anchors, graph counts and diff whitespace checks also pass.

Only the tower-layer interface dependency is added: the standalone HTTP graph
is now 30 packages beyond fixture/library, with no native transport stack or
Tower utilities. The HTTP fixture calls a mapped stateful handler through its
public service API. No native code, C ABI or wire changes; verification reuses
`/tmp/simple-server-unix-client-engine/debug/libsimple_server_engine.so`
(SHA256 `98d4cff50e4237d50e6235b3c1fb84b8a775bd86e8c3b929df9d099989db3fc7`).
The older 29-package benchmark remains historical. No push/publication/deployment.

Implementation `6672dec` was integrated by rebasing local master onto the
implementation branch. Original master remained clean at `26c1b68`; no concurrent
commits or edits appeared. Ancestry and exact tested-tree equality passed.
Temporary branch/worktree cleanup follows integration of this evidence update.

## Engine downstream build measurement — 2026-10-06

Implementation remains **Partial**, production adoption **Pending**. Start:
clean local master `63da312`; isolated branch `implementation/engine-build-benchmark`,
worktree `target/worktrees/engine-build-benchmark`. Runtime/library code is unchanged.

Added an equivalent source/engine JSON HTTP fixture and an opt-in reproducible
benchmark. Three alternating trials use fresh Cargo targets, offline dev builds,
eight jobs, incremental compilation and warm filesystem caches. The installed
Unix-client engine is prebuilt; its build/download cost is excluded. Clean-build
medians are 7.467 s source versus 3.345 s engine (55.2% reduction, 2.23x ratio).
No-op medians are 0.091 versus 0.081 s; leaf-edit medians 0.309 versus 0.236 s.
Normal/build package counts excluding fixture/library are 50 versus 29.
All 18 builds pass actual endpoint/shutdown checks. Initial preflight exposed a
missing loader SONAME path in the runner; fixed before the measured trials,
then a complete preflight passed. No runtime performance or real-service speedup
is claimed. Debug engine packaging is larger in total despite a smaller binary.

See [design and reproduction](design.md#downstream-build-measurement-2026-10-06)
and [raw results](measurements/engine-build-2026-10-06.json). Logs and targets:
`/tmp/simple-server-build-measurement-20261006`; console log has the same prefix
with `.log`. Engine hash, source baseline revision and fixture hashes are recorded.

Both fixture configurations pass strict Clippy; fixture formatting, benchmark
syntax/help, recorded fixture hashes, all measured engine dependency graphs,
tracker anchors/scripts and diff whitespace checks pass. Full Rust suites and
engine rebuild were not rerun because no runtime/library code changed.

ScT was inspected read-only on its active, clean master
`9e118e6c73db6f80b15f85fc525a1d254096a05b` (not remote HEAD). Actual production
startup uses web::serve, application-level correlation/observation middleware,
body-limit/header layers and a StaticDir fallback. Middleware and source-free
correlation/tracing/static-file adapters are the next relevant HTTP gaps.
PostgreSQL SQLx, direct Tokio APIs and Reqwest/OIDC remain independent build-cost
contributors. No ScT files were changed, built or migrated. TLS/WebSocket serving
was not found in the inspected startup/routes, so it is not this service's next
blocker. No push, artifact publication or deployment.

Benchmark commit `8b598ea` was integrated by rebasing local master onto the
implementation branch. The original checkout remained clean at `63da312`; no
concurrent commits or edits appeared. Ancestry and exact tested-tree equality
passed; ScT remained clean at the inspected revision. Temporary branch/worktree
cleanup follows integration of this evidence update.

## Engine dependency isolation guard — 2026-10-06

Engine implementation remains **Partial**; production adoption remains **Pending**.
No consumer repository or runtime code is changed. Work starts from clean local
master `49af123`, in isolated branch `implementation/engine-dependency-guard`
and worktree `target/worktrees/engine-dependency-guard`.

Baseline reproduces a hole in the shell guard: `hyperlocal v0.9.1` is accepted
while Hyper and hyper-util are rejected. A shared Python package-name check now
rejects Hyperlocal alongside Axum, Tokio, Reqwest, Hyper, SQLx, Rustls and Multer,
including their hyphenated subcrates. Paths and duplicate graph entries do not
change the result. `scripts/check-engine` runs this guard on all five standalone
engine fixture graphs using locked normal/build dependencies. Cargo errors fail
the check. The scheduler uses the same family policy while retaining its existing
explicit Tokio sync/macros feature allowlist and mio/socket2 exclusions.

Verification: five guard regression tests pass, all five real standalone graphs
pass, and the real scheduler graph passes with 26 dependencies beyond fixture
and library. Shell syntax and diff whitespace checks pass. The baseline scheduler
graph also passed. These are check-script changes only: the Rust suites and native
artifact build are not rerun, and the previous Unix-client artifact remains
applicable. No compilation-speed claim, production adoption, push or deployment.

Implementation `4544f21` was integrated by rebasing local master onto the
implementation branch. Original master remained clean at `49af123`; no concurrent
edits or commits appeared. Ancestry and exact tested-tree equality passed.
Temporary branch/worktree cleanup follows integration of this evidence update.

## Engine Unix HTTP client — 2026-10-06

Engine implementation remains **Partial** and production adoption remains
**Pending**. Work starts from clean local master `fc2ce03`, in isolated branch
`implementation/engine-unix-client`, worktree `target/worktrees/engine-unix-client`.
No consumer repository is changed.

UnixClient::new now creates a native pooled HTTP/1 client, returning io::Result.
Clones share the same pool. Requests stream through body callbacks and preserve
method, encoded path/query, headers, Host, version and trailers. Responses retain
status, headers, version and streaming bodies. Socket selection is independent
of the original URI authority. InvalidRequest versus Transport classification
matches the source contract; socket paths must be nonempty UTF-8 and request
paths origin-form. Error sources are host-owned diagnostics; native Rust error
objects and arbitrary extensions do not cross the ABI. Protocol upgrades and
application deadlines/forwarding policy remain caller-owned or unsupported.

Response body slots are allocated before asynchronous request work and released
by host guards, preventing unclaimed registrations on cancellation. Request body
size hints preserve fixed-length framing. Responses remain readable after client
drop. Hyperlocal 0.9.1 and Hyper transport/pooling are compiled only into the
engine; the consumer graph remains 29 normal/build packages beyond fixture and
library, without Hyper, Hyperlocal, Axum, Tokio, Reqwest, SQLx, Rustls or Multer
(`/tmp/simple-server-unix-client-graph.txt`). No speedup is claimed.

Baseline: 20 tests pass (engine Unix 5, source Unix 7, engine HTTP 8), recorded in
`/tmp/simple-server-unix-client-baseline.log`. Nine new client contracts plus five
Unix-serving and eight TCP HTTP regressions pass
(`/tmp/simple-server-unix-client-contracts.log`). Tests cover invalid inputs and
transport errors, bidirectional incremental streaming, trailers, upload/late
response failures, cancelled uploads and dropped responses. A raw socket test
verifies two cloned clients reuse one accepted connection and send fixed-length
requests. Source-client comparisons use real sockets for HTTP/1.0, HTTP/1.1,
HEAD, encoded paths/queries, duplicate headers and binary bodies, excluding Date.

Full `scripts/check` passes (`/tmp/simple-server-unix-client-full.log`):
workspace/native strict Clippy, source feature matrix and protocol regressions,
rustdoc, engine contracts (nine all-feature and eight engine-only Unix-client
cases), seven standalone engine consumers, dependency guards, C ABI smoke and
five installer tests. The HTTP fixture exercises TCP, raw Unix sockets and the
public Unix client.

The standalone HTTP consumer now exercises public UnixClient requests alongside
raw sockets. Against the previous library it fails client creation with
`unknown engine resource`, as expected
(`/tmp/simple-server-unix-client-old-engine.log`). New commands and client kind 14
require an updated artifact; response slots reuse body kind 6. C ABI signatures
and layout are unchanged. Verification uses the newly built local x86_64 debug
engine at `/tmp/simple-server-unix-client-engine/debug`, built from this
checkpoint; build log `/tmp/simple-server-unix-client-engine-build.log`.
SHA256 `98d4cff50e4237d50e6235b3c1fb84b8a775bd86e8c3b929df9d099989db3fc7`.
Bookworm/release distribution, ARM and non-Unix targets are not retested.

Implementation commit `2bf12ef` was integrated by rebasing local master onto
`implementation/engine-unix-client`. The original checkout remained clean at
`fc2ce03` before integration; no concurrent commits or edits appeared. Ancestry
and exact tree equality with the tested implementation branch passed. Temporary
worktree/branch cleanup follows integration of this evidence-only update.

TLS, WebSocket, external listener descriptors, peer credentials, router layers,
Route/make-service exposure and public test-harness parity remain pending.
No push, publication or deployment.

## Engine Unix-socket serving — 2026-10-06

Engine implementation remains **Partial** and production adoption remains
**Pending**. Work starts from clean local master `26314cc`, in isolated branch
`implementation/engine-unix-serving`, worktree
`target/worktrees/engine-unix-serving`. No consumer repository is changed.

On Unix, engine-web now exposes unix::bind, unix::serve and UnixListener.
The engine binds raw path bytes and owns acceptance; the host keeps the supplied
path. Native bind errors retain OS error codes. Existing sockets, files and
symlinks are never replaced or removed. Listener drop closes the socket but
retains its filesystem entry; permissions and cleanup remain application-owned.
TCP and Unix share routing, streaming callbacks and shutdown handling. Unix
requests have no TCP peer address. Pre-requested shutdown accepts no queued
requests; normal shutdown drains active responses, with deadlines caller-owned.

Baseline: 15 tests pass (source Unix 7, engine HTTP 8), recorded in
`/tmp/simple-server-unix-baseline.log`. Five Unix contracts and eight TCP HTTP
regressions pass (`/tmp/simple-server-unix-contracts.log`). Unix contracts cover
non-UTF-8 paths, existing files/symlinks, OS errors and listener drop; nested
captures/original URI, duplicate headers, incremental bidirectional streaming and
trailers; pre-requested shutdown; graceful response drain and disconnect cleanup.
Source comparison uses real Unix sockets on both sides for GET/HEAD/405/404,
encoded paths and queries, excluding only Date and header order. Incremental
streaming receives response data before the request upload finishes.

The standalone HTTP consumer now also binds and serves a Unix socket and cleans
up its own test path. Running it with the previous engine rejects bind with
`unsupported engine operation`, as expected (`/tmp/simple-server-unix-old-engine.log`).
New operations and listener resource kind 13 require an updated artifact; C ABI
signatures/layout remain unchanged. No dependencies are added: the HTTP consumer
retains 29 normal/build packages beyond fixture and library, without Axum,
Tokio, Hyper, Reqwest, SQLx, Rustls or Multer
(`/tmp/simple-server-unix-graph.txt`). No speedup is claimed.

The full `bash scripts/check` passes (`/tmp/simple-server-unix-full.log`): strict
workspace/native Clippy, source feature matrix and protocol regressions, rustdoc,
five all-feature and four engine-only Unix contracts, all seven standalone engine
binaries, dependency guards, C ABI smoke and five artifact installer tests. The
HTTP standalone consumer successfully serves both TCP and Unix with the rebuilt
engine. Tracker scripts and evidence links pass validation.

Verification uses the newly built local x86_64 debug engine at
`/tmp/simple-server-unix-engine/debug`, built from this checkpoint; build log:
`/tmp/simple-server-unix-engine-build.log`.
SHA256 `0b4786cfc12f02263fc369df71ff96a8377f023002aad7be5992ee4639c1c732`.
Bookworm/release distribution, ARM and non-Unix targets are not retested.
Unix client requests, externally supplied listener descriptors, peer credentials,
TLS, WebSocket, router layers, Route/make-service exposure and public test-harness
parity remain pending. No push, publication or deployment.

Implementation commit **`9be8f9b`** is integrated into local `master` by rebasing
master onto the implementation branch. Original master remained clean at
`26314cc`; no intervening commits or unrelated edits appeared. Ancestry and exact
tree equality were verified, so the integrated implementation is the tested tree.
This evidence update is integrated the same way; the temporary worktree and merged
branch are then removed after verification.

## Engine multipart uploads — 2026-10-06

Engine implementation remains **Partial** and production adoption remains
**Pending**. Work starts from clean local master `d0473d1`, in isolated branch
`implementation/engine-multipart`, worktree `target/worktrees/engine-multipart`.
No consumer repository is changed.

The engine-web API now exposes borrowed/owned multipart readers and fields,
metadata, streaming chunks and collected bytes/charset-aware text. Multer 3.1.0
is compiled only in the native engine. Total request limits default to 2 MiB and
honor BodyLimit overrides. Borrowed and owned adapters preserve their different
source limit-rejection texts. Error status and diagnostics cross the wire;
error sources are host-owned diagnostics rather than native Rust error objects.
Borrowed fields enforce exclusivity at compile time. Owned fields enforce it at
runtime, may outlive their reader, and support dropping/skipping a field.

Field slots are allocated synchronously before async parsing so cancellation
cannot orphan a newly allocated ID. Host guards release registrations; native
map removals drop outside locks to permit callback reentry. Existing response
body export is factored into a reusable callback helper. Body/error/trailer and
response-disconnect tests cover this shared change.

Baseline: 21 tests pass (source multipart 5, engine extractors 8, engine HTTP 8),
recorded in `/tmp/simple-server-multipart-baseline.log`. Eight new multipart
contracts and eight HTTP regressions pass
(`/tmp/simple-server-multipart-contracts.log`). Contracts cover source status,
headers and bytes for normal/malformed/oversized uploads; borrowed and owned
fields; lazy chunks and body release; cancellation during header/chunk/text
reads; charset/BOM/replacement decoding; field lifetime after reader drop;
and default/overridden total limits. A compile-fail example prevents concurrent
borrowed fields. No whole-request collection is introduced.

The full `bash scripts/check` passes (`/tmp/simple-server-multipart-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc including the borrowed-field compile-fail contract, eight all-feature
and seven engine-only multipart contracts, all seven standalone engine binaries,
dependency guards, C ABI smoke and five artifact installer tests. The standalone
HTTP consumer successfully parses binary uploads with the rebuilt engine.
Multer is also added to the downstream dependency guard; the strengthened guard
is separately verified for all five guarded fixture graphs. Tracker scripts and
evidence links pass validation.

New native multipart commands and resource kinds 11/12 require updated artifacts;
C ABI signatures/layout remain unchanged. The updated standalone fixture fails
against the previous engine with HTTP 500 at multipart extraction, as expected
(`/tmp/simple-server-multipart-old-engine.log`). Verification uses the newly
built local x86_64 debug engine at `/tmp/simple-server-multipart-engine/debug`,
from this checkpoint; build log `/tmp/simple-server-multipart-engine-build.log`.
SHA256 `c76ae40d8b82d9e360fcf31c80576a7785b5bbd3d592073df615711cc23a693a`.
Bookworm/release distribution and ARM are not retested. The HTTP consumer graph
remains 29 normal/build packages beyond fixture and library, without Multer,
Axum, Tokio, Hyper, Reqwest, SQLx or Rustls
(`/tmp/simple-server-multipart-graph.txt`). No speedup is claimed.

Router layers, Route service exposure, make-service, WebSocket, TLS/Unix transport
and public test-harness parity remain pending. No push, publication or deployment.

Implementation commit **`89b51aa`** is integrated into local `master` by rebasing
master onto the implementation branch. Original master remained clean at
`d0473d1`; no intervening commits or unrelated edits appeared. Ancestry and exact
tree equality were verified. The integrated implementation is the tested tree;
the final strengthened dependency guard was separately checked against all five
fixture graphs. This evidence update is integrated the same way; the temporary
worktree and merged branch are then removed after verification.

## Engine server-sent events — 2026-10-06

Engine implementation remains **Partial** and production adoption remains
**Pending**. Work starts from clean local `master` `90d7309`, in isolated branch
`implementation/engine-sse`, worktree `target/worktrees/engine-sse`. No consumer
repository is changed. Router-wide layers need routing continuation and host
extension preservation; this checkpoint instead closes the independent SSE gap.

`engine_web::sse` supplies Sse, Event, EventDataWriter, EventError and KeepAlive
under engine-web alone. Encoding preserves source field order, multiline and
chunked data, Unicode, retry hints, JSON errors and builder validation. The
encoder is adapted from pinned Axum 0.8.9 with its MIT notice retained. Lazy body
polling supports non-Unpin streams, propagates producer errors and releases the
stream on drop. Keepalives use engine timers, remain opt-in and default to empty
comments after 15 idle seconds. Ready events reset the timer; ready events,
errors and completion take priority over heartbeats. Authorization, subscriptions,
replay and reconnect policy remain application-owned.

Baseline: 25 tests pass (source SSE 11, engine HTTP 8, engine services 6), recorded
in `/tmp/simple-server-sse-baseline.log`. Eleven focused engine SSE contracts
pass (`/tmp/simple-server-sse-contracts.log`), including byte/header comparison
with the source encoder, paused engine-clock timing tests, producer-error and
drop checks, and a real HTTP test for authorization, reconnect IDs, heartbeat
delivery and stream release after disconnect. The standalone HTTP consumer
executes SSE alongside its existing routes (`/tmp/simple-server-sse-fixture.log`).
Its added direct futures-util dependency is already in the graph: 29 normal/build
packages beyond fixture and library, with no Axum, Tokio, Hyper, Reqwest, SQLx or
Rustls (`/tmp/simple-server-sse-graph.txt`). No speedup is claimed.

The full `bash scripts/check` passes (`/tmp/simple-server-sse-full.log`): strict
workspace/native Clippy, source feature matrix and protocol regressions, rustdoc,
eleven all-feature and ten engine-only SSE contracts, all seven standalone engine
binaries, dependency guards, C ABI smoke and five artifact installer tests.
Tracker scripts and evidence links pass validation.

No native source or ABI changes are needed. Tests reuse the x86_64 debug engine
built at checkpoint `f4c87cb`, at `/tmp/simple-server-nested-services-engine/debug`,
SHA256 `8a66f5cb8ecf760ef0e5d1ca8dfc515e8dfe3f32ed85afd93ac053dfc2adcc4c`.
Bookworm/release distribution and ARM are not retested. Router layers, Route
service exposure, make-service, WebSocket/multipart, TLS/Unix transport and test
harness parity remain pending. No push, publication or deployment.

Implementation commit **`da5f059`** is integrated into local `master` by rebasing
master onto the implementation branch. Original master remained clean at
`90d7309`; no intervening commits or unrelated edits appeared. Ancestry and exact
tree equality were verified, so the integrated implementation is the tested tree.
This evidence commit is integrated the same way; the temporary worktree and
merged branch are then removed after verification.

## Engine nested services — 2026-10-06

Engine implementation remains **Partial** and production adoption remains
**Pending**. Work starts from clean local `master` `436c87c`, in isolated branch
`implementation/engine-nested-services`, worktree
`target/worktrees/engine-nested-services`. No consumer repository is changed.

`Router::nest_service` mounts host-owned Tower services through the native
router. Prefix stripping preserves queries and encoded path segments; original
URI and parent captures remain available in RequestMetadata. Services preserve
same-clone readiness, cancellation and lazy streaming bodies. Mounts compose with
pending typed state, independently bound clones and merged routers. Invalid,
root, wildcard and conflicting mounts fail eagerly without corrupting clones.
The standalone HTTP consumer now exercises a mounted streaming echo underneath
a captured parent route and checks both stripped and original URIs.

This adds the native `router_nest_service` wire operation. C ABI signatures and
layout remain unchanged, but consumers adopting this method need an updated
engine artifact. Running the updated fixture against the previous engine fails
at construction with `unknown router command`, as expected
(`/tmp/simple-server-nested-services-old-engine.log`). No new dependencies:
the HTTP consumer retains 29 normal/build packages excluding the library and
fixture, without Axum, Tokio, Hyper, Reqwest, SQLx or Rustls
(`/tmp/simple-server-nested-services-graph.txt`). No speedup is claimed.

Baseline: 18 engine service/router-state/HTTP tests pass before edits
(`/tmp/simple-server-nested-services-baseline.log`). The focused suite then
passes 19 tests, including mount/state validation and source parity over HTTP
(`/tmp/simple-server-nested-services-contracts.log`). Comparisons cover mount
boundaries, trailing slashes, queries, encoded paths, GET/HEAD/POST/custom methods
and captured parent prefixes, excluding only Date. Existing readiness,
disconnect and incremental bidirectional streaming tests now also exercise
mounted services. Root mounting is unsupported by the source router too and
is explicitly tested as an error; root dispatch uses fallback_service.

Verification uses the newly built local x86_64 debug engine from this checkpoint
at `/tmp/simple-server-nested-services-engine/debug`; build log:
`/tmp/simple-server-nested-services-engine-build.log`. SHA256:
`8a66f5cb8ecf760ef0e5d1ca8dfc515e8dfe3f32ed85afd93ac053dfc2adcc4c`.
The full `bash scripts/check` passes (`/tmp/simple-server-nested-services-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc, six all-feature service contracts and five engine-only contracts,
all seven standalone engine binaries, dependency guards, C ABI smoke and five
artifact installer tests. The updated standalone HTTP consumer executes
successfully against the rebuilt engine. Tracker scripts and evidence links
pass validation.

Bookworm/release distribution and ARM are not retested. Router-wide layers,
Route service exposure, make-service and protocol adapters remain pending.
No push, publication or deployment.

Implementation commit **`f4c87cb`** is integrated into local `master` by rebasing
master onto the implementation branch. Original master remained clean at
`436c87c`; no intervening commits or unrelated edits appeared. Ancestry and exact
tree equality were verified, so the integrated code is the tested tree. This
evidence update is integrated the same way; the temporary worktree and merged
branch are then removed after verification.

## Engine method and fallback services — 2026-10-06

Engine implementation remains **Partial** and all production adoption remains
**Pending**. Work starts from clean local `master` `679640f`, in isolated branch
`implementation/engine-services`, worktree `target/worktrees/engine-services`.
No consumer repository is changed.

MethodRouter on_service/any_service and Router fallback_service now dispatch
runtime-independent Tower services over owned engine requests. Each request
clones its service, awaits that clone's readiness and calls the same instance.
Service errors must be Infallible; body errors retain the existing BoxError
boundary. Response bodies are wrapped lazily. Services coexist with pending typed
state, raw handlers and method fallbacks. Native matching, HEAD/405/Allow and
fallback behavior are preserved. No service readiness is polled at registration.

The standalone public HTTP fixture now executes its streaming echo through a
service implemented against the reexported engine_web::Service trait. Enabling
engine-web adds tower-service only; the normal/build graph grows from 28 to 29
packages beyond the library and fixture, without Axum, Tokio, Hyper, Reqwest,
SQLx or Rustls. No compile/runtime speedup is claimed. No native source or ABI
changes are needed. Tests reuse the local x86_64 debug engine at
`/tmp/simple-server-responses-engine/debug` from the previous checkpoint;
Bookworm/release distribution and ARM are not retested.

Baseline: 27 tests pass (`/tmp/simple-server-services-baseline.log`): engine HTTP
(8), router state (5), source web core (14). Five new focused contracts pass
(`/tmp/simple-server-services-contracts.log`): readiness and same-clone call,
cancellation both before readiness and during call, bidirectional incremental
streaming, pending-state/method-fallback composition and source parity over real
HTTP on both sides. The streaming test receives the first chunk before sending
the next. Comparisons exclude only Date; both sides use socket transport so
transport-generated Content-Length is compared consistently.

The full `bash scripts/check` passes (`/tmp/simple-server-services-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc, all five service contracts and four engine-only service contracts,
all seven standalone engine binaries, dependency graph guards, C ABI smoke and
artifact installer tests. The actual HTTP consumer graph is recorded in
`/tmp/simple-server-services-graph.txt`. Tracker script syntax and evidence links
pass validation.

Router-wide Tower layers, Route service exposure, nested services, make-service
adapters and protocol adapters remain pending. No push, publication or deployment
is performed.

Reviewed implementation `e577be9` is integrated into local `master` by rebasing
the clean original branch onto the isolated implementation branch. Ancestry and
exact tree equality pass, with no concurrent edits or conflict resolutions.
The temporary worktree and merged branch are removed after this verification
record is integrated.

## Engine header arrays and redirects — 2026-10-06

Engine implementation remains **Partial** and all production adoption remains
**Pending**. Work starts from clean local `master` `be16158`, in isolated branch
`implementation/engine-responses` and worktree `target/worktrees/engine-responses`.
No consumer repository is changed.

Standalone header arrays, array/body and status/array/body responses, Redirect
constructors/getters/conversion, and response-module Json/Response exports now
match the source response helper surface. Arrays replace duplicate values;
HeaderMap::append remains the mechanism for repeated fields. Failed conversion
returns a fresh 500 response without partial headers or the original body.
Status precedence, conversion order and lazy body disposal are preserved.
Redirects retain 303/307/308 statuses and validate Location during conversion.

The previous `/tmp/simple-server-lifecycle-artifact-amd64` artifact was absent,
so the initial baseline stopped at the build script before running tests. The
unchanged native engine was rebuilt with the locked dependencies from `be16158`
into `/tmp/simple-server-responses-engine/debug` (build log
`/tmp/simple-server-responses-engine-build.log`, SHA256
`c3f7c9c9556ebdfa4718bac3501423f7da735fb8624fc30ca2927b385687a110`).
Verification here uses this host-built x86_64 debug library, not the prior
Bookworm artifact. Release distribution, Bookworm compatibility and ARM are not
retested; there are no native source or ABI changes.

The rerun pre-edit baseline passed 27 tests: engine extractors (8), forms (5),
source web core (14), recorded in `/tmp/simple-server-responses-baseline.log`.
Five focused contracts pass (`/tmp/simple-server-responses-contracts.log`): array
replacement, empty responses, preservation of host response metadata, invalid
header disposal without body polling, status precedence, redirects and exact
source comparisons covering relative/absolute/Unicode/invalid locations and
header conversion failures. The standalone public HTTP fixture now executes a
redirect with array headers and an invalid-header 500 response through the `.so`.

The full `bash scripts/check` passes (`/tmp/simple-server-responses-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc, all five response contracts and three engine-only contracts, all seven
standalone engine binaries, graph guards, C ABI smoke and artifact installer tests.
The HTTP consumer graph remains 28 packages beyond library and fixture, without
Axum, Tokio, Hyper, Reqwest, SQLx or Rustls (`/tmp/simple-server-responses-graph.txt`).
Tracker script syntax and evidence links pass validation.

No dependency additions or compile/runtime speedup claims. Layers/services,
TLS/Unix serving and protocol adapters remain pending. No push, publication or
deployment is performed.

Reviewed implementation `beafef4` is integrated into local `master` by rebasing
the clean original branch onto the isolated implementation branch. Ancestry and
exact tree equality pass, with no concurrent edits or conflict resolutions.
The temporary worktree and merged branch are removed after this verification
record is integrated.

## Generic engine router state — 2026-10-05

Engine implementation remains **Partial** and all production adoption remains
**Pending**. This checkpoint starts from clean local `master` `3e7875f`, in isolated
branch `implementation/engine-router-state` and worktree
`target/worktrees/engine-router-state`. No consumer repository is changed.

`Router<S>` and `MethodRouter<S>` now represent missing state. Fallible
`with_state<S2>` binds existing handlers while allowing later handlers to require
a different type; only unit-state routers can serve. Typed normal/any handlers,
fallbacks, method fallbacks, merge and nesting share this model. Bound method-local
and handler-local state remains independent. Pending clones can bind different
values without mutation. The standalone public HTTP fixture actually uses router
state for its typed JSON route.

The host retains native validation resources and state-binding plans. Unbound
handlers have inert validation callbacks; those resources never reach serving.
Paths/methods/composition are validated eagerly. Binding constructs fresh native
callbacks and routers; already-bound resources are reused. This adds construction
work and host plan storage, not a new per-request routing layer. Rust state never
crosses the ABI. The native library and dependency set are unchanged; no measured
compile/runtime improvement is claimed.

Baseline: 42 existing tests pass (`/tmp/simple-server-router-state-baseline.log`):
engine HTTP (8), extractors (8), forms (5), paths (7), source web core (14).
The 28 existing engine contracts pass after the implementation
(`/tmp/simple-server-router-state-regressions.log`). Five new state contracts pass
(`/tmp/simple-server-router-state-contracts.log`), covering clone isolation,
substate, method-local state, binding in stages, any/method fallbacks with
HEAD/405/Allow behavior, raw/local/shared mixtures, eager invalid-route errors and
state release after the last bound clone is dropped. Actual `.so` HTTP comparisons
against the source router match status, headers and body (excluding transport Date)
for shared/local/nested/merged/raw/fallback routes. Three compile-fail examples
prevent serving unbound state, mixing state types and supplying the wrong type.

The full `bash scripts/check` passes (`/tmp/simple-server-router-state-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc (including the three state compile-fail checks and successful binding
example), all five state contracts and four engine-only state contracts, all seven
standalone engine binaries, dependency guards, C ABI smoke and artifact installer
tests. The HTTP fixture's normal/build graph remains 28 packages beyond library
and fixture, without Axum, Tokio, Hyper, Reqwest, SQLx or Rustls
(`/tmp/simple-server-router-state-graph.txt`). Tracker script syntax and evidence
links pass validation.

The existing Bookworm x86_64 artifact at
`/tmp/simple-server-lifecycle-artifact-amd64` is reused. ARM is not retested.
Tower layers/services, remaining response helpers and protocol adapters remain
pending. No push, publication or deployment is performed.

Reviewed implementation `4b1d356` is integrated into local `master` by rebasing
the clean original branch onto the isolated implementation branch. Ancestry and
exact tree equality pass, with no concurrent edits or conflict resolutions.
The temporary worktree and merged branch are removed after this verification
record is integrated.

## Typed engine path extraction — 2026-10-05

Engine implementation remains **Partial** and all production adoption remains
**Pending**. Work starts from clean local `master` `f30edc7`, in isolated branch
`implementation/engine-path` and worktree `target/worktrees/engine-path`.
No consumer repository is changed.

Engine `Path<T>` now deserializes the owned, already-decoded capture metadata.
Scalars, newtypes, tuples, structs, maps, sequences, unit enums and custom Serde
string visitors preserve the source backend's behavior. Metadata is borrowed,
not consumed; failed path extraction short-circuits body reads. Invalid UTF-8 and
value errors preserve 400 responses, while wrong capture counts and unsupported
types preserve 500 responses. The Serde-only parser is adapted from Axum 0.8.9,
with its full MIT license notice retained in `src/engine_web/path_de.rs`; no Axum
runtime types or dependency are introduced in the engine consumer.

The pre-edit baseline passed 35 tests: existing engine extractors (8), forms (5),
public HTTP (8) and all-feature source web core (14), recorded in
`/tmp/simple-server-path-baseline.log`. Seven new contracts pass in
`/tmp/simple-server-path-contracts.log`. They cover metadata reuse, missing
metadata, Result rejection capture, no body polling after path rejection, and
actual `.so` HTTP comparisons against the source router. The HTTP comparisons
check status, headers and exact bodies, excluding the transport-generated Date
header; cases include UTF-8 errors, plus signs, encoded slashes/double encoding,
custom errors, incorrect arity, unsupported shapes, wildcard routes, dynamic
nesting and duplicate parameter names.

The complete `bash scripts/check` passes (`/tmp/simple-server-path-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc, all seven path contracts and three engine-only path contracts, all seven
standalone engine binaries, graph guards, C ABI smoke and artifact installer tests.
Tracker script syntax and evidence links pass validation. The standalone HTTP
normal/build graph remains 28 packages beyond fixture and library, without Axum,
Tokio, Hyper, Reqwest, SQLx or Rustls (`/tmp/simple-server-path-graph.txt`).

The standalone HTTP consumer now serves a typed tuple path route and verifies
successful decoding, numeric parse failure and invalid UTF-8. No native/ABI or
dependency changes are required. The existing Bookworm x86_64 artifact at
`/tmp/simple-server-lifecycle-artifact-amd64` is reused; ARM is not retested.
No compile/runtime speedup is claimed. Generic router state, Tower layers,
remaining response helpers and protocol adapters remain pending.
No push, publication or deployment is performed.

Reviewed implementation `5dbc922` is integrated into local `master` by rebasing
the clean original branch onto the isolated implementation branch. Ancestry and
exact tree equality pass, with no concurrent edits or conflict resolutions.
The temporary worktree and merged branch are removed after this verification
record is integrated.

## Engine forms and request extensions — 2026-10-05

Engine implementation remains **Partial** and all production adoption remains
**Pending**. This checkpoint starts from clean local `master` `fae18de`, in
`implementation/engine-forms` and isolated worktree `target/worktrees/engine-forms`.
No consumer repository is changed.

Engine Form extraction and responses, plus required/optional host-local Extension
extraction, are implemented. GET reads the query; HEAD and other methods read the
bounded body. This preserves the actual source behavior and corrects the source
wrapper's misleading HEAD comment. Source status/header/body contracts, MIME
prefix acceptance, decoding, default 2 MiB limits and per-request overrides are
preserved. Extension errors retain the legacy source diagnostic text. No arbitrary
Rust extension crosses the ABI; Tower layers and generic middleware remain pending.
Typed path extraction needs a dedicated scalar/tuple/struct deserializer and is
still pending, alongside generic router state, response helpers and protocol work.

The baseline passed 25 tests: existing engine extraction/parity (8), source web
core (14) and extraction (3), recorded in `/tmp/simple-server-forms-baseline.log`.
Five new focused tests pass (`/tmp/simple-server-forms-contracts.log`), including
120 method/content-type/payload comparisons against source, form response success
and serialization failure, exact missing-extension rejection, body limits, stream
errors and proof that rejected requests do not poll their body. The standalone
public HTTP fixture additionally serves a form route using a host-local extension.

The full `bash scripts/check` passes (`/tmp/simple-server-forms-full.log`):
strict workspace/native Clippy, source feature matrix and protocol regressions,
rustdoc, combined and engine-only contracts, all seven standalone engine binaries,
dependency guards, C ABI smoke and artifact installer tests. The engine-only test
import was subsequently feature-gated; three minimal engine-web tests pass again
without the unused-import warning (`/tmp/simple-server-forms-final.log`). Tracker
script syntax and evidence links pass validation. The HTTP fixture graph remains
28 dependency packages excluding library and fixture, with no Axum, Tokio, Hyper,
Reqwest, SQLx or Rustls (`/tmp/simple-server-forms-graph.txt`).

No native/ABI or dependency changes are needed. The existing Bookworm x86_64
artifact at `/tmp/simple-server-lifecycle-artifact-amd64` is reused; ARM is not
retested. No measured compile/runtime speedup or production adoption is claimed.
No push, publication or deployment is performed.

Reviewed implementation `5a9794a` is integrated into local `master` by rebasing
the clean original branch onto the isolated implementation branch. Ancestry and
exact tree equality pass, with no concurrent edits or conflict resolutions.
The temporary worktree and merged branch are removed after this verification
record is integrated.

## Typed engine handlers and extraction — 2026-10-05

Engine implementation remains **Partial** and all production adoption, including
Favzetto, remains **Pending**. Work starts from clean local `master` `49d9cb9`
in branch `implementation/engine-extractors`, worktree
`target/worktrees/engine-extractors`. No consumer repository is changed.

Engine `on_handler`, `on_state` and `fallback_handler` now reuse the existing
zero-to-sixteen-argument handler machinery. Raw request registration is unchanged.
Engine State/substate, Query, Json/optional JSON, String/Bytes, raw query, matched
path and direct peer extraction use owned HTTP types. Custom Extract and rejection
contracts are shared. Primitive head and Result-capture implementations moved to
the runtime-independent extraction module to avoid duplicate implementations
when both HTTP backends are enabled.

Body extractors preserve the 2 MiB default and accept a per-request BodyLimit
override. Invalid optional JSON remains an error when a content type is present.
Custom head rejections short-circuit before body polling. Common text/binary,
JSON/HTML, status/HeaderMap tuples, Result and rejection responses now convert
without Axum. Typed path/form extraction, extension adapters, router-wide generic
state, remaining response helpers, middleware and protocol adapters are still
pending; this is not a drop-in source web replacement.

The standalone public HTTP fixture now actually runs both streaming echo and
a typed state/query/JSON route. Its normal/build graph has 28 dependency packages
beyond the library and fixture, with no Axum, Tokio, Hyper, Reqwest, SQLx or Rustls.
MIME parsing and path-aware Serde error support add two packages to the previous
26-package graph. This is dependency evidence, not a compile/runtime benchmark.
The existing Bookworm x86_64 engine is reused without native or ABI changes;
ARM is not retested and source/default execution is unchanged.

The pre-edit baseline passed 25 tests: public engine HTTP (8), extraction (3),
and all-feature source web core (14)
(`/tmp/simple-server-extractors-baseline.log`). The focused comparison run passed
seven new engine/parity tests plus source extraction/web regressions
(`/tmp/simple-server-extractors-contracts.log`). It compares JSON/query/UTF-8
rejection status, headers and exact bodies against source, checks body limits and
stream errors, and exercises state/metadata over loopback HTTP. An additional
response conversion parity test and two engine compile-fail handler-ordering
examples are included in the final check matrix.

The complete `bash scripts/check` passes
(`/tmp/simple-server-extractors-full.log`): strict workspace/native Clippy,
source feature matrix and protocol regressions, rustdoc (including the two new
compile-fail examples), all eight engine/parity tests, engine-only execution,
all seven standalone consumers, dependency graph guards, C ABI smoke and artifact
installer tests. An engine-only unused test import was subsequently feature-gated;
all four engine-only extractor tests pass again without that warning
(`/tmp/simple-server-extractors-final.log`). Tracker script syntax and the evidence
link pass validation.

No push, publication or deployment is performed.

Reviewed implementation `2f69de7` is integrated into local `master` by rebasing
the clean original branch onto the isolated implementation branch. Ancestry and
exact tree equality pass; there were no concurrent edits or conflict resolutions.
The temporary worktree and merged branch are removed after this verification
record is integrated.

## Public engine HTTP core — 2026-10-05

Overall engine implementation remains **Partial**, and all production engine
adoption, including Favzetto, remains **Pending**. This checkpoint starts from
clean local `master` `1c6c0fb`, using isolated branch `implementation/engine-web`
and worktree `target/worktrees/engine-web`. No consumer repository is changed.

The new `engine-web` feature exposes a public `engine_web` module for TCP binding,
method routing, merge/nesting/fallbacks, raw request handlers, owned bodies and
explicit graceful shutdown. Applications no longer need internal resource IDs,
callbacks or byte commands for this subset. Handlers accept one `Request` and
return `Response`; route construction is fallible and state is captured in
closures. This is explicitly not yet a drop-in replacement for source `web`.

Request metadata carries method/URI/version, binary and duplicate headers, peer,
matched/original paths, decoded parameters and path-decoding errors. Independent
incoming body ownership permits returning a request body after the handler ends.
Data and trailer frames remain lazy; response callbacks retain host resources
until the engine acquires them. Collection has explicit limits. Response status,
headers and body cross the ABI; arbitrary extensions and explicit response-version
overrides do not. Explicit shutdown drains responses; dropping the serve future
alone does not promise connection termination.

The independent `tests/fixtures/engine_web` consumer uses only public APIs to
serve a streaming echo and request it through the engine client. Its normal/build
graph has 26 dependency packages excluding the library and fixture, with no
Axum, Tokio, Hyper, Reqwest, SQLx or Rustls. The graph guard now covers this fixture.
The separately supplied native engine still contains those implementations;
no compilation or runtime speedup has been benchmarked. Default/source web
consumers retain their existing dependency graph and runtime behavior.

The original internal HTTP fixture passed before edits
(`/tmp/simple-server-public-web-baseline.log`). Eight new public API contracts
pass (`/tmp/simple-server-public-web-contracts.log`): metadata and streaming echo,
method/HEAD/fallback/merge/clone semantics, limits and body errors, wire trailers,
draining an active response, disconnect cancellation/drop, listener error/release
and already-requested shutdown, invalid route recovery and handler panic isolation.
The implementation reuses the tested Bookworm x86_64 artifact at
`/tmp/simple-server-lifecycle-artifact-amd64` (SHA256
`88d175e260cfdb6768dfcebeba2d7f31a4c713146f8d4151aef94adc97b3162e`),
without native or ABI changes. ARM was not retested for this checkpoint.

The complete `bash scripts/check` passes
(`/tmp/simple-server-public-web-full.log`): strict workspace/native Clippy,
source feature matrix and protocol regressions, rustdoc, engine-only and combined
features, all seven standalone binaries, dependency graph guards, C ABI smoke and
artifact installer tests. This includes actual execution of the new public HTTP
consumer against the prebuilt engine. Tracker script syntax and the evidence
link pass validation.

Typed extractors, generic router state, response conversions, Tower middleware,
TLS/Unix serving, WebSocket/multipart/SSE adapters and the public test harness
remain parity work. No push, publication or deployment is performed.

Reviewed implementation `d36b95e` is integrated into local `master` by rebasing
onto `implementation/engine-web`. Ancestry and tree verification preserve the
tested implementation. The original checkout remained clean; no concurrent
commits or edits required restoration. The temporary worktree and branch are
removed after integrating this verification record.

## Engine scheduler and policies — 2026-10-05

Overall engine implementation remains **Partial**; all production engine adoption,
including Favzetto, remains **Pending**. Work starts from clean local `master`
`15bf9e7` in isolated branch `implementation/engine-scheduler`, worktree
`target/worktrees/engine-scheduler`. No consumer repository is changed.

The new `engine-scheduling` feature exposes `engine_scheduling`: the full
scheduler, cron registry, capacity/priority helpers and polling drivers, with
engine task ownership and clocks. Existing algorithms and contracts are shared
with source `task_scheduling`. Coverage includes fixed-rate/delay/cron timing,
manual/event ingress, bounded queues, resource limits, completion reporting,
retry reservations, queue/runtime budgets, circuits, pause/restore and shutdown.
Policies are shared through `task_policies`; engine scheduler task types come
from `engine_tasks`. The pure `task-policy-core` feature makes policy definitions
available without source task execution. Feature unification makes policy APIs
available on both modules but does not switch source callers to the engine.

Tokio remains a small wrapper dependency for synchronization and macros only.
The standalone `tests/fixtures/engine_scheduler` consumer has 26 normal/build
dependency packages beyond simple-server. A dedicated graph guard rejects
Tokio executor/time/network/signal features and heavy HTTP/database stacks.
It permits only Tokio `sync`, `macros` and the implicit `tokio-macros` feature.
Source modules explicitly retain their previous Tokio features via
`source-runtime`. Enabling those modules still compiles their source runtime.
Calendar parsing and policy algorithms remain downstream. No compilation or
execution speedup has been benchmarked, and the default source backend remains.

The new fixture actually runs scheduled retries, paused-clock timers and
explicit shutdown through the engine; it is not merely a feature declaration.
It passed with the existing Bookworm x86_64 artifact at
`/tmp/simple-server-lifecycle-artifact-amd64` (SHA256
`88d175e260cfdb6768dfcebeba2d7f31a4c713146f8d4151aef94adc97b3162e`).
No native implementation or ABI changes are needed. ARM was not retested.

The original relevant source baseline passed 52 tests, including pure policy
rules (`/tmp/simple-server-scheduler-baseline.log`). After migration, 42 shared
integration contracts pass on each backend, plus two internal clock/ingress
tests per backend and the common shutdown unit test. The existing additional
blocking-executor queue-delay test passes on source only: engine Builder lacks
`max_blocking_threads`, so that exact setup is not claimed verified on engine.
Engine blocking shutdown and runtime budgets are covered by the shared policy
contracts. The combined run passes 90 tests
(`/tmp/simple-server-scheduler-contracts.log`). Standalone execution and its
dependency guard pass (`/tmp/simple-server-scheduler-fixture.log`).

The complete `bash scripts/check` passes
(`/tmp/simple-server-scheduler-full.log`): strict workspace/native Clippy, source
feature matrix, rustdoc, engine-only and combined-feature contracts, all six
standalone binaries, graph guards, C ABI smoke and artifact installer checks.
Source scheduling without policies also passes its 12 unit/integration tests
(`/tmp/simple-server-scheduler-source-only.log`); the default library check passes
(`/tmp/simple-server-scheduler-default.log`). Tracker JavaScript syntax and the
new evidence link are validated.

Public web execution and production adoption remain unfinished. No push,
publication or deployment is performed.

Reviewed implementation commit `dc34f01` is integrated into local `master` by
rebasing onto `implementation/engine-scheduler`. Ancestry and tree comparison
confirm the tested implementation is unchanged. The original checkout remained
clean, with no concurrent commits or edits requiring restoration. The temporary
worktree and branch are removed after integrating this verification record.

## Engine task supervisor — 2026-10-05

Library engine implementation remains **Partial**; all production engine adoption,
including Favzetto, remains **Pending**. This checkpoint starts from clean local
`master` at `dbb4c88`, using isolated branch `implementation/engine-supervisor`
and worktree `target/worktrees/engine-supervisor`. No consumer repository changes.

The new `engine-tasks` feature provides `engine_tasks::{TaskSet, WorkTracker}`.
It shares the source ownership algorithm while selecting the engine runtime,
task collection and clock. Admission limits include unconsumed completions;
first cancellation reasons remain distinct from task outcomes. Blocking jobs
cannot be explicitly aborted. Timed-out or cancelled drains retain ownership
and collected results. Dropping a set requests cooperative shutdown and
detaches unfinished execution; only draining proves completion.

WorkTracker now uses the existing runtime-independent sticky notification,
signalled only when closed and empty. Admission and close remain serialized;
waiters are awakened outside the state lock. Existing `tasks` callers keep their
source runtime and standard-library Instant API even when both features are
enabled. Engine callers use `time::Instant` for deadlines and actual execution
timestamps, including paused time. Full scheduler/policy execution and public
web execution migration remain pending.

The independent `tests/fixtures/engine_tasks` consumer enables only engine-tasks
with default features disabled. Its normal/build graph has seven dependency
packages excluding simple-server (eight including it), with no Tokio or heavy
HTTP/database stack. This does not reduce the default source consumer graph and
does not establish a measured compilation or runtime speedup. No new ABI or
native engine change is needed; verification uses the existing Bookworm x86_64
artifact at `/tmp/simple-server-lifecycle-artifact-amd64` (SHA256
`88d175e260cfdb6768dfcebeba2d7f31a4c713146f8d4151aef94adc97b3162e`).

The ten original task contracts passed before edits
(`/tmp/simple-server-supervisor-baseline.log`). Thirteen shared contracts pass
on each backend with both features enabled
(`/tmp/simple-server-supervisor-contracts.log`). Added coverage checks admission
without a runtime, paused-clock execution timestamps, and multiple tracker
waiters awakened by external completion. Existing coverage checks cancellation,
blocking ownership, panic reporting, bounded admission, detach-on-drop, and
resumable/cancelled draining. The complete `bash scripts/check` passes
(`/tmp/simple-server-supervisor-full.log`): strict workspace and native Clippy,
source feature matrix and scheduler/policy regressions, rustdoc, engine-only and
combined-feature contracts, five standalone consumer binaries, dependency graph
guards, C ABI smoke and artifact installer tests. The new standalone consumer
actually runs tracking, cooperative shutdown and paused-clock draining through
the prebuilt engine. Tracker script syntax and the evidence link pass validation.
ARM execution is not retested at this checkpoint. Nothing is pushed, published
or deployed.

Reviewed implementation `a827ea9` is integrated on local `master` by rebasing
onto `implementation/engine-supervisor`. Ancestry and tree verification confirm
the tested implementation is unchanged. The original checkout remained clean;
no concurrent commits or edits needed restoration. The temporary worktree and
branch are removed after integrating this verification record.

## Engine lifecycle and deadlines — 2026-10-05

Library implementation remains **Partial**; all production engine adoption,
including Favzetto, remains **Pending**. This checkpoint starts from clean local
`master` at `e0d5d86`, in isolated branch `implementation/engine-lifecycle` and
worktree `target/worktrees/engine-lifecycle`. No consumer repository is changed.

The new `engine-lifecycle` feature exposes `engine_lifecycle::{Lifecycle,
Shutdown, Signals}`. Coordination preserves borrowed service futures, explicit
signal installation, cancellation, error reporting and one deadline shared by
service draining and cleanup. Source and engine APIs share the coordination
algorithm and a runtime-independent sticky shutdown notification. Selecting the
engine module is explicit; enabling both features leaves existing source
callers on their source runtime. Full `TaskSet`, `Scheduler`, and public web
execution migration remain unfinished.

Owned engine `Instant`, `Sleep`, `sleep_until` and `timeout_at` now support
absolute deadlines and paused clocks. Timer deadlines are fixed at construction,
including when first polled later. The ABI appends `clock_now` and `clock_valid`,
with signed seconds and normalized nanoseconds relative to a process-local
native origin; no Rust Instant representation crosses the boundary. Native
signal registrations use resource kind 10. Dropping them releases receivers but
does not restore process-wide default signal handlers. The larger ABI table
requires a rebuilt development engine; old artifacts are not compatible with
the new bindings. Interval/reset and standard Instant conversions remain absent.

The standalone `tests/fixtures/engine_lifecycle` consumer runs borrowed work,
paused-clock draining/cleanup and explicit signal registration. Its normal/build
graph has 12 dependencies excluding simple-server (13 including it), with no
Tokio, Axum, Reqwest, Hyper, SQLx or Rustls. This is a feature-specific dependency
count, not a measured compile-time or execution speedup. Default consumers still
compile the source backend.

Verification: the original ten lifecycle contracts passed before edits
(`/tmp/simple-server-engine-lifecycle-baseline.log`). The same ten contracts pass
on each backend, including deadline exhaustion and borrowed futures; four new
clock/timer contracts pass. Source and engine child-process SIGINT/SIGTERM tests
pass outside the sandbox; the first sandboxed engine attempt timed out. Strict
workspace Clippy passed. The full source feature matrix, tests and rustdoc passed
in `/tmp/simple-server-engine-lifecycle-full.log`; that run then caught a borrowed
capture error in the new standalone fixture. After correcting only that fixture,
the complete engine script passed (`/tmp/simple-server-engine-lifecycle-final-engine.log`),
including native Clippy, sys/runtime tests, all four standalone binaries,
heavy-dependency guards, C ABI smoke and artifact installer checks.

A fresh Bookworm x86_64 artifact was built locally at
`/tmp/simple-server-lifecycle-artifact-amd64`; its SHA256 is
`88d175e260cfdb6768dfcebeba2d7f31a4c713146f8d4151aef94adc97b3162e`,
with SONAME `libsimple_server_engine.so.1`. ARM was not rebuilt or retested for
this checkpoint. The complete engine script also passed against this release
artifact on the host (`/tmp/simple-server-engine-lifecycle-release-check.log`),
including clocks, lifecycle, signals, standalone consumers and C ABI checks.
No release, push, or deployment is authorized or performed.

Reviewed implementation commit `328513c` is integrated on local `master` by
rebasing it onto `implementation/engine-lifecycle`. Ancestry and tree comparison
confirm that the tested implementation is unchanged; the original checkout
remained clean and no concurrent commits or edits required restoration. Tracker
JavaScript syntax and the new evidence link pass validation. The temporary
worktree and branch are removed after integrating this verification record.

## Engine scheduling drivers — 2026-10-05

Engine implementation remains **Partial** and all production adoption remains
**Pending**, including Favzetto. Work starts from clean library `master`
`814f64d` in isolated branch `implementation/engine-drivers`, worktree
`target/worktrees/engine-drivers`. No consumer repository is changed.

`task-drivers` now exposes engine-backed `task_drivers::run_bounded_batch` and
`run_poll_worker` independently of the full scheduler. Both reuse the existing
algorithms: batches keep bounded admission, report typed panic payloads, and
abort children when dropped; polling drains the accepted cycle, uses the
caller-selected cadence and yields on zero delay. The application supplies the
engine runtime, persistence and reporting. No hidden runtime is created.

Backend selection is explicit by API module: `task_drivers` uses the engine,
while `task_scheduling` keeps source execution even when both features are
enabled. This avoids changing an existing caller's runtime requirements through
Cargo feature unification. The modules share the driver/error implementation
files and behavioral test contracts. Full `Scheduler`, `TaskSet`, lifecycle clocks/signals and
public web adapters are still on the source backend. This checkpoint adds no
engine ABI or native implementation change.

For applications using only these helpers, normal/build dependency packages
excluding simple-server drop from 30 (`--no-default-features --features
task-scheduling`) to seven (`--no-default-features --features task-drivers`).
The new standalone `engine_drivers` fixture has eight dependencies including
simple-server, contains no direct or transitive Tokio, and actually runs bounded
work and polling through the prebuilt engine. Graphs are recorded in
`/tmp/simple-server-engine-drivers-source-graph.txt` and
`/tmp/simple-server-engine-drivers-engine-graph.txt`. This compares the available
feature selections for that helper-only use case; the engine is prebuilt
separately, full scheduler parity is not claimed, and compile/runtime timing has
not been benchmarked. The default source consumer graph is unchanged.

The six original driver tests passed before edits
(`/tmp/simple-server-engine-drivers-baseline.log`). Five shared behavioral tests
pass with the engine-only feature
(`/tmp/simple-server-engine-drivers-contracts.log`); the sixth checks selection
helpers belonging to the full scheduler. Source scheduler, policy, selection,
driver and task-set tests also pass
(`/tmp/simple-server-engine-drivers-source-tests.log`). The independent fixture
runs successfully (`/tmp/simple-server-engine-drivers-fixture.log`). The same
contract tests exercise both API modules on their respective runtimes when both
features are enabled.
The engine check script now runs the drivers and fixture and rejects heavy
runtime dependencies in both standalone fixture graphs. Final `bash scripts/check`
passed (`/tmp/simple-server-engine-drivers-final-check.log`), including strict
Clippy, all source feature checks, engine-only and combined-feature contracts,
docs, ABI/artifact checks and all three standalone binaries. The final
coexistence run separately records both backend contracts
(`/tmp/simple-server-engine-drivers-coexistence.log`). The new fixture also
passes strict Clippy (`/tmp/simple-server-engine-drivers-fixture-clippy.log`).
Tracker JavaScript syntax and the new evidence link were validated. Reviewed
commit `2583c81` was integrated by rebasing local `master` onto
`implementation/engine-drivers`. Ancestry and tree comparison preserve the
verified implementation; no concurrent local edits needed restoration. The
temporary worktree and branch are removed after integrating this verification
record. No push, publication or deployment was performed.

## Engine task collections — 2026-10-05

Library implementation remains **Partial**; production engine adoption,
including Favzetto, remains **Pending**. Work starts from clean library `master`
`c317ba8`. Initial isolated branch `implementation/engine-tasks` lost its
`/tmp/simple-server-engine-tasks` directory during validation; the main checkout
was unchanged. Recorded patches were restored on `implementation/engine-taskset`
in the ignored, isolated worktree
`target/worktrees/engine-taskset`, and validation was rerun there. No consumer
checkout was changed.

The engine-backed runtime now exposes owned `JoinSet`, `Id` and `Handle` APIs.
Task IDs remain available on join handles, abort handles and panic/cancellation
errors. Collections support asynchronous or nonblocking joins (with or without
IDs), bounded ready-queue polling, abort/drain, explicit detach, and spawning on
selected runtimes from foreign threads. Drop requests cancellation rather than
pretending work already stopped; a running blocking function still completes
normally. Application results and typed panic payloads remain in the host.
Handles do not keep a runtime alive and reject spawning after its owner drops.

The implementation adds no dependency and does not change the C ABI or native
engine. It runs against the existing local x86_64 Bookworm HTTP engine artifact
with SHA-256
`4337d1b6a3d3e1c6cdd9c7b4eb832c4783eace8c2ec5ad8b7ddec5c0c1a08126`.
The standalone consumer now exercises task collections and runtime lookup; its
22-package normal/build dependency graph still excludes Tokio, Axum, Hyper,
Reqwest, SQLx and Rustls. This is fixture adoption, not production adoption or a
claim that the default source-library build is lean. Existing `tasks::TaskSet`,
scheduling drivers, lifecycle clocks/signals and public web adapters still need
the execution migration. Full parity, Favzetto, ARM and release/performance gates
remain pending.

Baseline engine checks passed before edits
(`/tmp/simple-server-engine-tasks-baseline.log`). Recovered-source verification:
11 task-collection/handle tests and eight original runtime tests pass
(`/tmp/simple-server-engine-taskset-contracts.log`), covering cancellation-safe
waits, 2,000 concurrent task completions, abort/detach ownership, typed panics,
nonblocking joins, multiple runtimes, shutdown, foreign-thread spawning and a
panicking application waker. `bash scripts/check-engine` passed
(`/tmp/simple-server-engine-taskset-final.log`), including HTTP transports,
callbacks, client/process/SQLite, C ABI, artifact installer and both standalone
binaries. Workspace/all-target/all-feature strict Clippy passed
(`/tmp/simple-server-engine-taskset-clippy.log`). The full source feature-test
matrix was not rerun for this engine-bindings-only change; it passed at the
preceding checkpoint. Strict sys-crate rustdoc passed
(`/tmp/simple-server-engine-taskset-doc.log`); tracker script syntax and the new
evidence link were validated. Reviewed commit `a4791ae` was integrated by
rebasing local `master` onto `implementation/engine-taskset`; ancestry and tree
comparison confirm the tested tree is preserved. No concurrent local edits
needed restoration. The stale missing-worktree registration and old baseline
branch were removed. The replacement worktree/branch are removed after
integrating this verification record. No push, publication or deployment was
performed.

## Owned Tokio-facing contracts — 2026-10-05

Shared-engine implementation remains **Partial**; all production adoption,
including Favzetto, remains **Pending**. Work starts from clean library `master`
`16305ff` in `implementation/owned-network`, isolated worktree
`/tmp/simple-server-owned-network`. No consumer checkout is changed.

The source HTTP/TLS/Unix adapters now use owned TCP and Unix listeners.
`http::bind` retains string, host/port, IP/port, socket-address and reference
inputs through `net::ToSocketAddrs`; slices preserve ordered bind fallback.
Arrays, vectors and custom async resolvers are also supported. Standard-library
listener import/export retains socket configuration and enables nonblocking
mode. Unix socket paths remain application-owned. The harness uses owned
binding, and transport tests pass owned listeners into the public serving APIs.
Raw Axum comparison tests intentionally retain their backend listener setup.

`task_scheduling::JoinError` and `BatchTaskId` replace exposed Tokio batch-task
types, preserving panic payloads, cancellation classification and task identity.
`rate_limit::AcquireError` owns the closed-admission error. These changes affect
explicit Tokio type annotations and generic trait bounds; they belong to the
unpublished breaking migration. Runtime/main/task/time engine APIs were already
owned. General synchronization APIs and moving remaining source execution into
the engine remain unfinished; public type ownership is not engine adoption.
The default source dependency graph is unchanged, and this checkpoint claims no
compilation or runtime speedup.

The full unmodified baseline `bash scripts/check` passed
(`/tmp/simple-server-owned-network-baseline.log`). New contract checks passed
(`/tmp/simple-server-owned-network-contracts.log`): five listener cases cover
address forms, IPv6 metadata, fallback/last-error behavior, empty/malformed
addresses, resolver errors, port release, standard socket options and Unix path
ownership. Six scheduling-driver and four delayed-admission tests pass,
including typed panic recovery and owned closed-admission errors. Final
`bash scripts/check` passed (`/tmp/simple-server-owned-network-final.log`),
including strict Clippy, the feature matrix, doctests/docs, source HTTP/TLS/Unix
and WebSocket tests, and engine ABI/artifact/standalone-consumer checks. Engine
implementation and ABI are unchanged in this checkpoint. Tracker JavaScript
syntax and the new evidence link were validated. Reviewed commit `c057c34` was
integrated by rebasing local `master` onto `implementation/owned-network`; the
integrated tree matched the tested tree and there were no concurrent local edits
to preserve. The temporary worktree and branch are removed after integrating
this verification record. No push, publication or deployment was performed.

## Shared-engine HTTP transport checkpoint — 2026-10-05

Library status remains **Partial** and all production consumers remain
**Pending** for the shared engine. Work started from clean library `master`
`de2bab8` in isolated branch `implementation/engine-http` and worktree
`/tmp/simple-server-engine-http`. Favzetto remains unchanged; this checkpoint
has no consumer migration commit or production adoption claim.

The engine now owns TCP listeners, HTTP/1 and HTTP/2 transport, basic route and
method assembly, nesting/merge/fallback, and streaming request/response bodies.
Host callbacks receive request metadata and borrow body registrations; an
explicit clone retains a body beyond dispatch. Producer-owned callback reply
buffers retain response streams until the engine acquires them. This avoids
premature release without leaking registrations.

Seven transport tests cover an incremental binary echo, repeated headers and
trailers, body-registration cleanup, nested/original paths and percent decoding,
HEAD/405/Allow/404 behavior, malformed replies and handler panic cleanup,
disconnect cancellation, pre-requested shutdown, in-flight stream draining and
HTTP/2 with both client and server in the engine. The full `bash scripts/check`
suite passed; its log is `/tmp/simple-server-http-full-check.log`. The unchanged
baseline engine checks passed first (`/tmp/simple-server-http-baseline.log`).

The independent fixture's new `http` binary serves and calls a parameterized
route using internal bindings. Its normal/build dependency graph still has 22
dependency packages and excludes Axum, Hyper, Tokio, Reqwest, SQLx and Rustls.
A Rust 1.88 build of this fixture ran against the Rust 1.96 engine inside a
network-disabled Bookworm container, using container loopback only.

The current x86_64 Bookworm artifact is local and unpublished, SHA-256
`4337d1b6a3d3e1c6cdd9c7b4eb832c4783eace8c2ec5ad8b7ddec5c0c1a08126`.
The new HTTP code has not yet been built or tested on ARM. Foundation artifact
hashes recorded below describe the earlier subset and are not current artifacts.

Remaining work includes the public Router/Body/listener adapters, generic state
and Tower composition, protocols, TLS/Unix transport, runtime-dependent library
modules, Favzetto adoption, full ARM verification and performance/release gates.
The public web backend is still the source implementation. No public signature,
consumer checkout, remote branch, release or deployment was changed here.
Reviewed implementation commit `28dea84` was integrated by rebasing library
`master` onto `implementation/engine-http`. The integrated tree matched the
tested tree, with no concurrent edits to preserve. Workspace strict Clippy and
the seven final transport tests also passed against the release-built Bookworm
library after adding explicit stale-body-registration checks. The original
checkout is clean. The temporary worktree/branch are removed after integrating
this verification record; the public adapter and consumer migration remain
unfinished.

## Shared engine implementation — 2026-10-05 (in progress)

Library implementation is **Partial**. Production adoption is **Pending** for
all active services, including the selected Favzetto canary. Existing routing
and other Done cells describe source-library adoption, not dynamic linking.
Quentin Torrentino remains excluded from consumer migration.

The foundation was implemented on `implementation/shared-engine`, based on
simple-server `master` at `bdfa076`, in `/tmp/simple-server-engine-work`.
Reviewed foundation commit `d21ddb1` was integrated by rebasing library `master`
onto that branch; the integrated tree matched the tested tree. Favzetto `master`
at `6bcbc8f` remains unchanged. There is no consumer migration commit or
production adoption evidence yet.

Implemented so far: a C-compatible versioned function table, producer-owned
buffers and callbacks, engine runtime and timers, owned outbound HTTP,
process output, typed SQLite queries and transactions, and entry-point/row
macros. The standalone consumer builds 22 dependency packages without Tokio,
Reqwest, SQLx, Hyper or Rustls in its normal/build graph. This is evidence for
that subset, not for the complete planned server build reduction.

Verification includes root borrowing, task panic payloads, cancellation,
blocking work, paused-clock timeouts, callback cancellation and panic isolation,
lazy binary HTTP bodies and repeated headers, charset decoding, process exit
status, exact SQLite integers/blobs, typed rows, commit and rollback on drop.
Artifact-installer tests cover checksum rejection, cache corruption, offline
reuse/misses, and mismatched target architecture. Initial socket tests were
blocked by the sandbox and rerun outside it. Build and smoke-test evidence is
being gathered before integration; no production migration is marked Done.
The complete existing `bash scripts/check` suite plus `scripts/check-engine`
passed against the x86_64 Bookworm-built engine. A separate Rust 1.88.0 consumer
also built and ran against the Rust 1.96.0 engine. The x86_64 artifact's maximum
required GLIBC symbol version is 2.34 and its SONAME is
`libsimple_server_engine.so.1`. These verify the current foundation, not missing
server/protocol functionality. Local verification logs are
`/tmp/simple-server-engine-full-check.log` and
`/tmp/simple-server-check-engine-unrestricted.log`.

Both Bookworm release builds succeeded. The ARM C ABI smoke test passed under
QEMU; x86_64 C and Rust consumers also ran in a network-disabled
`debian:bookworm-slim` container. Both engines require GLIBC symbols no newer
than 2.34. The local, unpublished artifact hashes are:

- x86_64: `26b271ed2484f1a4db7a9908d429a7de7dcfd9d8028aeb939b72285caf84c1c9`
- aarch64: `d6567df87ece504f1e57be5fc437a02be5da70ea1c9187902a0d4f24da30fa55`

The ARM check covers C ABI/runtime construction, not the complete Rust feature
suite. A follow-up host-waker panic-isolation test and workspace strict Clippy
passed after the full-suite run. The artifact download test uses a local mock
transport; no public release asset exists yet.

Remaining: the server/router and streaming protocol boundary, owned listeners,
all runtime-dependent module migrations, full feature parity, Favzetto's
production migration, full ARM feature checks, final artifact pins and
performance acceptance. The default backend is still the source backend.
No upload, push or deployment has occurred. The tested foundation is integrated
locally; the complete engine plan remains unfinished. The temporary foundation
worktree and branch are removed after integration of this verification record.

## Pezzottify crates.io dependency — 2026-09-29

Pezzottify and Pezzottify-downloader now consume the public
`lelloman-simple-server =0.1.0` package using the existing `simple-server` Cargo
alias. Production features and `simple_server` imports are preserved; Pezzottify's
test-harness dependency uses the same package. Both lockfiles identify crates.io
and checksum `1f3187c81c94701cd041df7ab17ce968b73c967db77c551170e3c24960b6c77a`.
The downloaded archive records source revision
`c95506164669c37a12bc06339a3b977a4b8d6a1b`; its `src/` tree matched the local
shared library byte-for-byte at migration start. This changes dependency delivery,
not capability adoption or runtime behavior.

- Pezzottify `dev`: `acef712f` → `8edff13c`. Removed sibling checkout script/pin,
  CI checkout steps, Docker COPY instructions and Compose source contexts. Docker
  builds enforce Cargo.lock. All-target check, 1,469 Rust tests (36 existing
  ignores), 45 Docker E2E tests (2 Android deselected), formatting, DB boundaries
  and strict production Clippy passed. Docker E2E builds from the consumer root
  without a library source context.
- Downloader `master`: `32d7ab9` → `9bd75f9`. Removed sibling checkout script/pin
  and Docker COPY instructions; build documentation no longer needs a library
  checkout. All-target check, 171 tests (1 existing ignore), and a complete
  standalone Docker release build passed. Existing formatting and strict Clippy
  failures reproduce on the unchanged development branch; no Rust source was
  edited and the Clippy diagnostic sets match.
- Homelab `master`: `bad5318` removes Pezzottify's obsolete build-context
  argument; the downloader invocation also has no library source context.
  Shell syntax and diff checks pass. Other services retain their existing build
  contexts, and unrelated homelab changes were preserved.
- Implemented in isolated worktrees; development branches were rebased onto
  their migration branches. Integrated trees match the tested trees, both
  consumer working trees are clean, and their temporary worktrees/branches were
  removed. No push or deployment was performed. Existing Fucina releases remain
  available; these consumers need neither private-registry credentials nor a
  `simple-server` checkout.

Open the [HTML migration matrix](migration-status.html) for the visual table,
with all services as rows and implemented and planned steps as columns.

This table tracks adoption of implemented `simple-server` capabilities. Each row
is a product; the component column identifies its Rust servers. Add a new step
column when the corresponding shared capability is implemented, then update
each product independently as it adopts that capability.

**Status:** Done = migrated and verified; Pending = not migrated;
Partial = only some listed components migrated; N/A = deliberately not needed.
Optional modules do not have to be adopted by every product.

**Combined auth (08/09):** active services complete locally, including Torrentino; its async bearer/session adoption is recorded below. Quentin Torrentino remains excluded.

**Rate limiting (10):** all applicable active consumers are complete locally,
including Torrentino’s durable application budgets. Quentin Torrentino remains excluded. The five final consumers now use shared
policies for their remaining quotas and pacing; application storage and transaction
ownership are preserved. Test limitations are recorded in the completion evidence.

**Routing / HTTP core (11):** all **17 active services Done**; excluded Quentin Torrentino remains Pending in the historical row. Pezzottify, Androidoscopy, Crumbles, Fausto, lello-auth, lellostore, Favzetto, Observo, Paranza, Peerlo, Meteonesto, Pezzottflix, Pezzottify-downloader, SCT, SimpleAI, simple-agents and Torrentino now
use shared APIs for all production route groups, ordinary handlers, built-in
extractors, response adapters and router assembly. Done excludes explicitly
tracked protocol compatibility boundaries. Pending records unverified adoption,
not a claim that the shared API already supports every service's needs.
See the [contract](web-core.md) and [completion evidence](#pezzottify-routing-completion--2026-09-25).

**Latest integration (2026-09-26):** LelloStore, Pezzottflix,
Pezzottify-downloader, SimpleAI and simple-agents complete the active owned
WebSocket rollout. See the [five-service evidence](#remaining-owned-websocket-rollout--2026-09-26)
for commits, tests, baseline limitations, integration and cleanup. Quentin
Torrentino remains skipped; remaining-exposure cells list pending work only.

**Shared WebSocket API (2026-09-26):** owned upgrades, sockets, messages, close
frames and errors are available with `web` + `ws`, including split streams,
subprotocol negotiation and transport configuration. All applicable services in the active rollout now use owned production
WebSocket APIs. Quentin Torrentino remains pending at user request. See the
[contract](web-core.md#owned-websockets) and
[verification record](#owned-websocket-api--2026-09-26).

**Next execution order:** Step 07a's optional SQLite connection policy and
[07b migration planning](step-07b-database-migrations.md) are implemented in
the shared library and adopted by all applicable in-scope services. Quentin
Torrentino remains excluded at user request. The optional [07e schema core](step-07e-sqlite-schema.md) is now implemented;
Pezzottify, SimpleAI, Paranza, Peerlo, Observo and Favzetto have scoped creation adoption;
Fausto and Crumbles now have complete scoped runtime creation adoption using
published 0.1.2 for vec0 and integration STRICT/CHECK. Androidoscopy and lello-auth
have evidenced N/A assessments for the optional schema module; Meteonesto and
Downloader now have evidenced N/A assessments as well. Pezzottflix CLI creation
is adopted.
The extended creation core is published as 0.1.1 and Peerlo metadata is integrated
with registry verification (2026-10-03). Simple Agents and SCT are integrated (2026-10-04). LelloStore has an evidenced N/A schema assessment; Torrentino’s replacement
service now adopts connection policy, version-only preflight and schema creation.
All applicable active SQLite consumers are complete; Quentin Torrentino remains excluded. Database drivers,
schema execution, queries and backups remain application-owned. See the
[Step 07 survey](step-07-database-survey.md),
[07a contract](step-07a-database-connections.md), [completion evidence](#step-07a-completion-pass--2026-09-29)
and [roadmap](design.md).

**SQLite backup and blocking execution (2026-10-04):** 07c and 07d are
published in crates.io 0.1.7. See the [backup contract](step-07c-sqlite-backup.md)
and [executor contract](step-07d-database-blocking.md). These new columns remain
Pending consumer assessment/canary; no feature flag alone counts as adoption.
07a/07b/07e adoption is independent. The next canaries should compare Pezzottify
checkpoint preparation and Crumbles staged copies, followed by Pezzottify's
blocking executor. Quentin Torrentino remains excluded.

**Step 07a status:** 11 Done, 0 Partial, 1 Pending excluded, 6 N/A.
SCT's PostgreSQL server is distinct from its SQLite archive catalog.

**Step 07b status:** shared preflight available; 11 Done, 0 Partial,
1 Pending (excluded Quentin Torrentino), 6 N/A in the consumer matrix.
Version-only stores report no verified historical digests. See the
[contract](step-07b-database-migrations.md) and
[completion evidence](#step-07b-completion-pass--2026-09-29).

The final observations column records the [Axum exposure audit](#axum-exposure-audit--2026-09-24).
It is independent of module adoption status.

Step 03a rollout verified: 2026-09-20. Earlier adoption evidence retains its original dates.

| Project | Server components | 1. Axum centralization | 2. Lifecycle / main() | 03a. Logging | 03b. Correlation | 03c. HTTP tracing | 04a. Body limits | 04b. Response headers | 04c. CORS | 05. Health/readiness | 06a. Task ownership | 06b. Scheduling | 06c. Execution policies | 08/09. Auth | 10. Rate limiting | 11. Routing / HTTP core | 07a. SQLite connection policy | 07b. Migration preflight | 07e. SQLite schema | 07c. SQLite backup | 07d. Blocking execution | Shared engine | Remaining backend exposure |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local)** | **Done (local canary)** | **Done (local; scoped canary)** | N/A (assessed) | N/A (no served probe) | **Done (local; scoped canary)** | **Done (local; primitives)** | **Done (local; primitives)** | **Done (local; sessions + route permissions)** | **Done (HTTP, MCP, durable quotas + outbound pacing)** | **Done (all production route groups)** | Done | Done (five versioned SQLite stores) | Done (versioned helper; scoped canary) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| favzetto | `backend` | **Done** | **Done (local; scoped)** | **Done (local pilot)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | **Done (local canary)** | **Done (local; request work)** | **Done (local; bounded batches)** | **Done (local; retry scope)** | **Done (local canary)** | **Done (local; global + endpoint budgets)** | **Done (local)** | Done | Done (backend SQLite) | Done (migration-ledger creation; scoped) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| androidoscopy | `server`; Android SDK pairing | **Done** | **Done (local; scoped)** | **Done (local; legacy logger)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (no served probe) | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; controller + LAN access)** | **Done (local; device JNI pairing gate)** | **Done (all production route groups)** | N/A (no SQLite database) | N/A (no Rust migration runner) | N/A (Rust has no SQLite; viewer uses Android platform API) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| crumbles | `crumbles`, `crumbles-integration` | **Done** | **Done (scoped)** | **Done (local canary)** | **Done (local pilot; main HTTP server)** | **Done (local canary; main HTTP server)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped canary)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; durable primitives)** | **Done (local; retry primitives)** | **Done (local; main + integration)** | **Done (local; HTTP + MCP + durable dispatcher)** | **Done (both servers)** | Done (core + integration SQLite) | Done (core + integration ledgers) | Done (core + integration ledger creation) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| fausto | `server`; associated plugin API and plugins | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | **Done (local)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; dynamic cron)** | N/A (assessed) | **Done (local; HTTP + WebSocket + admin/write)** | **Done (local; five API tiers)** | **Done (local)** | Done (storage SQLite) | Done (core SQLite store) | Done (ledger + runtime vec0 creation) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (Rust; Caddy owns CORS) | **Done (local; scoped)** | **Done (local; webhook scope)** | **Done (local; capacity)** | **Done (local; retry scope)** | **Done (local; sessions + resource/admin access)** | **Done (endpoint budgets + persisted device polling)** | **Done (local)** | Done (SQLite backend) | Done (SQLite + PostgreSQL version markers) | N/A (versioned SQL owns creation; no independent SQLite bootstrap) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| lellostore | `backend` | **Done** | **Done (local)** | **Done (local)** | N/A (assessed) | **Done (local)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; OIDC + admin)** | N/A (no inbound admission policy) | **Done (local)** | N/A (no explicit policy) | Done (backend SQLx) | N/A (versioned SQLx owns creation; no independent bootstrap) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; pipeline/gateway)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | **Done (local; weighted claims)** | **Done (local; budgets/retry)** | **Done (local; all three services)** | **Done (local; gateway budgets)** | **Done (local)** | Done (pipeline SQLite) | Done (weather-pipeline SQLite) | N/A (versioned SQL owns creation; no independent bootstrap) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| observo | `observo-server`; standalone extractor logging | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | N/A (no served probe) | **Done (local; scoped)** | **Done (local; primitives)** | N/A (assessed) | **Done (local; IP/key/JWT access)** | N/A (no request quota) | **Done (local)** | Done (server SQLite) | N/A (idempotent schema, no migration ledger) | Done (bootstrap creation; scoped) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| paranza | `apps/paranza-server` | **Done** | **Done (local; scoped)** | N/A (no logger) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; runner + PCM access)** | N/A (no implemented request quota) | **Done (local)** | N/A (driver defaults only) | N/A (idempotent DDL/column repairs, no migration ledger) | Done (store creation; scoped) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| peerlo | `peerlo-api` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; retry primitives)** | **Done (local; bearer + Torznab keys)** | **Done (local; API + crawler + durable DHT)** | **Done (local)** | N/A (driver defaults only) | N/A (create-if-missing stores, no migration ledger) | **Done (tracker + metadata creation)** | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| pezzottflix | `pezzottflix-server` | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | **Done (local; main HTTP)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; socket scope)** | **Done (local; durable queue cadence)** | N/A (assessed) | **Done (local; sessions + permissions + WebSocket)** | Done (login/daily/TMDB + configured HTTP limiter) | **Done (local; HTTP core)** | Done (server + CLI SQLite) | Done (server SQLx) | Done (CLI creation; server owns versioned SQL) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| pezzottify-downloader | Puppeteer API, downloader HTTP server and Python cron | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; Puppeteer)** | **Done (local; both HTTP routers)** | N/A (assessed) | N/A (assessed) | **Done (local; both routers)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; priority admission)** | N/A (assessed) | N/A (no application caller gate) | **Done (local; Python/SQLite quota bridge)** | **Done (local; parent + child HTTP core)** | N/A (Python SQLite only) | N/A (no Rust migration runner) | N/A (Python owns SQLite; Rust quota bridge has no schema API) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| torrentino | `crates/service` (included) | **Done** | **Done (shared lifecycle)** | **Done (shared logging)** | **Done (error-envelope UUIDs only)** | N/A (no HTTP tracing layer) | **Done (64 KiB API; 4 KiB sessions)** | **Done (authenticated cache policy)** | N/A (same-origin; no CORS grant) | N/A (no served probe) | **Done (tracked worker cycles)** | **Done (fixed-rate intake; completion-relative polling)** | **Done (runtime deadlines and retry decisions)** | **Done (async bearer/session verification)** | **Done (durable application counters and snapshot quota)** | **Done (owned HTTP + WebSockets)** | **Done (verified per-connection WAL/FK/busy policy)** | **Done (version-only user_version guard)** | **Done (11 tables; 3 explicit indexes)** | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| quentin-torrentino | `crates/server` (**excluded by request**) | Pending (legacy direct Axum) | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; stage capacity)** | N/A (assessed) | **Done (local; API access)** | **Done (local; MusicBrainz pacing)** | Pending | Pending (excluded by request) | Pending (excluded by request) | Pending (excluded by request) | Pending (excluded by request) | Pending (excluded by request) | Pending | Legacy direct Axum: backend routing/handlers/extractors/responses, auth/metrics middleware and serving; torrent multipart, chat SSE, dashboard WebSockets, static-file routing and HTTP test fixtures. Owned HTTP migration remains pending. |
| sct | `sct-server` | **Done** | **Done (scoped)** | N/A (no logger) | **Done (local; storage-backed HTTP)** | **Done (local; storage-backed HTTP)** | **Done (local; storage-backed HTTP)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; worker scope)** | **Done (local; durable worker cadence)** | **Done (local; retry scope)** | **Done (local; tokens/sessions + transactional access)** | N/A (durable state caps, no request quota) | **Done (local; HTTP core)** | Done (archive SQLite) | Done (PostgreSQL catalog; archive format N/A) | Done (offline archive creation; PostgreSQL outside scope) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| simple-agents | `simple-agents-service`; runner-shell logging; associated coding test servers | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local pilot; service routes)** | **Done (local; scoped canary)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; transactional capacity)** | **Done (local; retry scope)** | **Done (local; caller + transactional access)** | N/A (task/storage admission, no request quota) | **Done (local; HTTP core)** | Done (service + Runner SQLite) | Done (service ledger + Runner version marker) | Done (service ledger + Runner transport creation) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |
| simple-ai | `backend`, `inference-runner` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local; backend)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped canary)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; batch readiness)** | N/A (assessed) | **Done (local; backend user/admin)** | **Done (local; backend budgets)** | **Done (local; HTTP core)** | N/A (driver defaults only) | N/A (column-driven upgrades, no migration ledger) | Done (audit creation; scoped) | Pending (assessment/canary) | Pending (assessment/canary) | Pending | None. |

## Axum exposure audit — 2026-09-24

Recorded in both trackers on 2026-09-25 from the preceding read-only audit of all
17 active development branches, using two Terra agents and one Luna agent plus
coordinator review of simple-server. This was a source inspection of production
code, public interfaces, tests, examples and relevant manifests; no builds,
runtime tests or resolved dependency-graph verification were performed.

At this audit checkpoint all 17 products exposed Axum. Each needed consumer-facing routing, handler,
state/extraction and response/error interfaces, including test and example use.
The observations column identifies additional service-specific requirements;
completed infrastructure-module statuses remain unchanged. Standard `http`,
Tower and Serde APIs can remain where they do not expose Axum types or bounds.
Aliases and re-exports alone do not satisfy the abstraction goal.

Shared-library gaps include public HTTP serving bounds (`src/http.rs`),
correlation request/response and middleware types (`src/correlation.rs`), tracing
callbacks/observer responses (`src/http_tracing.rs`), and BodyLimit's associated
service type (`src/body_limit.rs`). New interfaces must cover middleware and
custom extraction, streaming bodies, multipart, SSE, WebSockets, peer metadata,
TLS/Unix-socket serving and framework-independent test fixtures. Existing auth,
rate-limit, CORS, health and response-header policies largely already use
independent HTTP/Tower interfaces.

Design Fausto's plugin route-contribution contract and Lello Auth's public
integration API before broad rollout: both currently return Axum routers.
Paranza is the smallest core HTTP pilot; Peerlo can follow for middleware
composition. Validate difficult streaming/rejection, multipart, real-time and
transport contracts before migrating the remaining consumers. Final verification
must cover public signatures, associated types, tests and examples before removing
the transitional re-export. See the [completion criteria](design.md#end-goal-completely-abstract-axum-away).

## Step 1: Axum centralization

Completion means the product's own direct Axum dependencies are replaced by
`simple-server`, source and tests use its transitional Axum re-export, and the
resolved graph uses the centrally pinned Axum version. Required feature flags,
companion-crate compatibility, build contexts, and applicable service tests must
be checked. Record existing check failures separately rather than hiding them.

Pezzottify completed this step in commit `3e3c548f` on `left`, using
`simple-server` revision `46a36391ccca522f3ec9aa24206f2b231162dd96` and Axum
0.8.9. Validation passed 1,374 Rust tests, 45 Docker E2E tests, Clippy, and the
release build. Its `docs/simple-server-migration.md` records the detailed results
and existing formatting/audit failures. The migration is committed locally;
this table tracks implementation, not deployment or remote publication.

Favzetto completed this step in commit `382cc8f` on `master`, using the same
`simple-server` revision and Axum 0.8.9 via a pinned public Git dependency.
Validation passed 130 unit tests, 91 API tests, the standalone Docker build, and
container health/readiness, frontend, and authentication smoke checks. Two API
tests fail identically on the untouched baseline; existing formatting and
strict Clippy failures also remain. Its `docs/06-simple-server-migration.md`
records the evidence. The migration is committed locally, not deployed.

Androidoscopy completed this step in commit `cbaf98a` on `master`, using the same
pinned public `simple-server` revision and upgrading Axum 0.7.9 to 0.8.9.
Validation passed 62 server tests (including UDP, WebSocket, and verified TLS
registration), five E2E unit tests, and seven full-stack scenarios. Existing
formatting and strict Clippy failures were confirmed against the unchanged
baseline. Its `docs/simple-server-migration.md` records the results. The migration
is committed locally, not deployed.

Crumbles completed this step for both `crumbles` and `crumbles-integration` in
commit `619aacd` on `simple-server-step01`, using the same pinned public Git
dependency and retaining Axum 0.8.9. Validation passed 1,365 Rust tests (two
intentional ignores), eight real-server Chromium E2E scenarios, formatting,
strict Clippy, unchanged generated API contracts, and the Docker build and
restart/persistence smoke test. See its `docs/SIMPLE_SERVER_MIGRATION.md`.
This was verified in the isolated `crumbles-step01` worktree from `de8b700`;
local `master` was subsequently rebased onto the migration branch on 2026-09-19,
preserving dispatcher development and adapting its newly added Axum imports.

Fausto completed this step in commit `258c4ff` on `master`, migrating the
server, plugin API, blog plugin, and runtime test-echo plugin to the same pinned
public Git dependency and Axum 0.8.9. Validation passed 633 Rust tests, formatting,
optional server feature compilation, and the locked Docker build. Clippy
completed with warnings. All 380 Docker E2E outcomes match untouched baseline
`086814d`: 349 passed, three skipped, seven expected failures, fourteen fixture
errors, and seven failures (six strict XPASS markers and one fixture failure).
The unchanged frontend has existing build errors; embedded UI Rust compilation
used a temporary asset fixture. See `docs/simple-server-migration.md` for details.
The migration is committed locally, not deployed.

Lello-auth completed this step in commit `5118c29` on `master`, migrating both
server crates and all three standalone examples to the same public Git pin and
Axum 0.8.9. The 0.7 migration updates route parameters, custom extractors,
cookies, test support, and the Askama response adapter. Rust 1.88 validation
passed 945 workspace tests (14 ignored), one doctest, strict Clippy, formatting,
all example builds, the Docker build, and all 92 deployed E2E tests including
Chromium and PostgreSQL. The initial SQLite contention test failure cleared on
a complete rerun. The audit still flags pre-existing `rustls 0.23.43`
(RUSTSEC-2026-0285) in workspace and helper lockfiles. See
`docs/SIMPLE_SERVER_MIGRATION.md` for results and release-gate scope limits.
The migration is committed locally, not deployed.

LelloStore completed this step in commit `eed1cff` on `master`, moving its
backend, mock OIDC binary, and tests to the same public Git pin and Axum 0.8.9.
The migration updates route parameters, authentication extractors, and WebSocket
text messages. Validation passed 122 Rust tests (two ignored doctests), strict
Clippy, formatting, the frontend and Docker builds, and container health,
frontend, and fail-closed authentication checks. A new real-socket E2E test
verifies authenticated WebSocket delivery after an admin upload. See
`docs/SIMPLE_SERVER_MIGRATION.md`. The migration is committed locally, not deployed.

Meteonesto completed Step 01 in commit `7cc0faa` on `master`, retaining Axum
0.8.9 through the same pinned public Git dependency in all three components.
Validation passed 251 Rust tests, two Python tests, formatting, strict Clippy,
builds, dependency policy checks and the new gateway-to-API real-socket E2E
test (added and verified before migration in `140e923`). The existing pipeline
Docker E2E suite fails identically before and after migration: expected 22
artifacts, produced 24; later backup/restore checks are not reached. See
`docs/simple-server-migration.md` in Meteonesto. No deployment was performed.

observo completed Step 01 in commit `573050e`. Passed 87 Rust tests, formatting, the Docker build and container smoke checks. New real-binary authentication, CRUD and restart-persistence E2E coverage passed before and after migration; its baseline exposed and fixed a blocking-client startup panic (830e9b8). Five stale test fixtures were repaired. Strict Clippy retains the same 36 baseline findings. See docs/simple-server-migration.md in Observo.

paranza completed Step 01 in commit `d4beb0e`. Passed 284 Rust tests, formatting and Docker release builds. All 28 Docker E2E scenarios ran: 25 passed; two multi-node certificate fixture failures and one freshness assertion failed identically on untouched baseline bec0460. Existing strict-Clippy findings were also confirmed on baseline. See docs/simple-server-migration.md in Paranza.

peerlo completed Step 01 in commit `e5a50dc`. Passed 780 Rust tests (six intentional ignores), formatting, Docker release builds and all 49 Docker swarm E2E tests before and after migration, including load and node recovery. Strict Clippy retains the same 13 baseline findings. See docs/simple-server-migration.md in Peerlo.

pezzottflix completed Step 01 in commit `a130c02`. Passed 529 Rust tests (three intentional ignores), Docker/frontend builds and all 10 backend E2E tests. New real WebSocket tests were committed first in f3c2ad1 and passed before migration. Rust tests needed a larger compiler stack; one cleanup assertion cleared on a full rerun. Existing formatting and strict-Clippy failures remain. Android TV protocol tests were outside scope. See docs/simple-server-migration.md in Pezzottflix.

pezzottify-downloader completed Step 01 in commit `98cab3a`. Passed 146 Rust tests, seven doctests, 90 Python tests, Docker build and isolated container smoke/shutdown checks. Two real-binary credential-free E2E tests were committed first in a01480a and pass before and after migration, alongside existing HTTP-to-Unix-socket integration tests. Live Spotify downloads were outside scope. Existing formatting/Clippy findings remain. See docs/simple-server-migration.md in the downloader repository.

quentin-torrentino completed Step 01 in commit `a5cbfc6`. Upgraded all 145 E2E scenarios to real HTTP before migration (1be03ef). Full workspace results match untouched baseline a88e04c: 739 passed, two failed, 14 ignored; E2E is 144/145. Existing MusicBrainz mock and temporary-directory assumptions cause the failures. Repaired stale Docker workspace inputs; release/dashboard builds and container checks pass. Existing formatting/Clippy failures remain. See docs/simple-server-migration.md in Torrentino.

sct completed Step 01 in commit `d577999`. Originally completed on simple-server-step01 from 5cffda9; local master was subsequently rebased onto it, preserving newer storage and tree work. Full scripts/check passes: 33 Rust/Postgres tests plus one doctest, strict Clippy/formatting, 49 contract tests, independent client packaging, frontend build and three real-server browser E2E scenarios. Four future milestone scenarios remain explicitly skipped. See docs/simple-server-migration.md in sct-step01.

simple-agents completed Step 01 in commit `5b13de6`. Full scripts/check passes before and after migration: 313 Rust tests, strict checks, seven JavaScript tests, 11 Android contract tests, three real-service browser E2E scenarios and repository consistency checks. Standalone managed-handoff qualification also passes against local Crumbles core; both lockfiles updated. See docs/simple-server-migration.md in Simple Agents.

simple-ai completed Step 01 in commit `daf92bd`. Backend and inference runner migrated together. E2E-first commit 1eb5974 adds a real-runner HTTP test and repairs stale gateway protocol fixtures. All 440 Rust tests and four Docker gateway E2E groups pass before and after; Rust 1.91 Docker build passes. Existing formatting and three common-crate Clippy findings remain documented.

## Step 2: lifecycle and entry-point setup

The shared implementation is available through the opt-in `lifecycle` feature.
Library checks pass for default, lifecycle-only, HTTP+lifecycle, all-features,
and no-default-features configurations. Real HTTP, streaming, WebSocket, and
child-process signal tests cover the shutdown contract.

Favzetto adopts shared signals, HTTP draining, and its assistant worker in commit
`ddce8e2`, with a
configurable 30-second default budget. It uses the local sibling library while
this implementation is under review. Its new lifecycle tests pass; two existing
catalog-research failures reproduce before and after migration. Existing detached
request jobs and upgraded WebSocket sessions remain outside coordinated draining;
see Favzetto's `docs/07-lifecycle-migration.md` for the exact scope and build setup.
This status describes that scoped adoption, not complete background-task ownership.

LelloStore migration commit `a71ec9f` adopts the committed library (`c535907`) for
both HTTP listeners, its
metrics updater, and tracked catalog WebSockets, followed by SQLite pool cleanup.
All 128 backend tests, strict Clippy, formatting, 16 script tests, and the Docker
release build pass. Its
local path dependency, CI sibling checkout, Docker context, shutdown budget, and
limits are documented in `docs/STEP_02_LIFECYCLE.md` in LelloStore.

LelloAuth commit `40f1c65` adopts shared HTTP/maintenance shutdown and updates all three HTTP
examples. It preserves connection information, CLI startup boundaries, and
existing outbound alert/webhook queue behavior. Validation: 949 workspace tests
pass (14 existing ignores), 92 deployed E2E tests pass, strict Clippy/formatting,
19 workflow contracts, all three example builds, the doctest, Docker build, and
Compose validation pass. See `docs/STEP_02_LIFECYCLE.md` in LelloAuth for scoped
shutdown guarantees and the reviewed sibling-source requirement.

Fausto commit `5a5e6e8` coordinates HTTP, tracked WebSockets, event distribution,
triggers, cron/manual jobs, and rate-limit cleanup. Plugin shutdown follows their
drain under the same configurable deadline. Auth last-seen updates and tasks
spawned independently by plugins retain their existing ownership. Validation:
642 Rust tests pass (4 existing ignores), formatting and locked Docker build pass,
and Clippy completes with warnings. All 380 existing E2E outcomes match baseline
(349 pass; 7 failures, 14 setup errors, 3 skips, 7 expected failures); the new
WebSocket shutdown test also passes. See `docs/STEP_02_LIFECYCLE.md` in Fausto for
shutdown limits, build setup, and baseline evidence.

Meteonesto commit `fb8519e` coordinates Weather API HTTP/cache workers and final
popularity persistence, gateway HTTP/JWKS refresh/SIGHUP policy reload, and the
pipeline's existing ordered drain through database/watchdog cleanup. The pipeline
snapshots its hot-reloaded grace at shutdown; API/gateway budgets apply on restart.
All three component checks pass: 253 Rust tests, six Python tests, strict Clippy,
formatting, builds, dependency policy, and applicable deployment checks. The
gateway-to-API E2E passes through the production lifecycle path. Pipeline Docker
build passes, but its representative E2E retains the documented 22-versus-24
artifact fixture failure; later restore assertions are not reached. Details,
rollback, and scoped guarantees are in `docs/step-02-lifecycle.md` in Meteonesto.

Pezzottify commit `aea97788` coordinates both HTTP listeners, scheduler jobs, event/storage/WAL
maintenance, playback/media recovery, sync/MCP WebSockets, and tracked search/
ingestion work under one 30-second deadline. Admin reboot uses the same graceful
path. The resumable OS-thread search-index build remains outside that scope.
Validation: 1,396 Rust tests pass (36 existing ignores), including four binary
lifecycle tests and two tracker tests; all 45 Docker E2E tests pass. Locked
all-target checking, strict Clippy, formatting, database-boundary checks, Docker
release/frontend builds, and Compose validation pass. Dependency audit passes
with six existing allowed warnings. See `docs/step-02-lifecycle.md` in Pezzottify
for shutdown scope and checkout details.

Observo commit `365d0d5` coordinates HTTP, scheduler ticks, and the server-task
runner, then drains tracked plugin webhooks under one 30-second deadline.
In-flight claims finish and finalize before the runner stops. Separate Node
workers and client-abandoned blocking route work retain their existing ownership.
Validation: 88 Rust tests, real-binary CRUD/persistence and SIGINT/SIGTERM E2E,
Docker release/container smoke checks, formatting, and Compose validation pass.
Strict Clippy retains the baseline findings (36 production, 37 including tests).
See `docs/step-02-lifecycle.md` in Observo.

Paranza commit `ac3126c` coordinates HTTP, runner sessions, and PCM maintenance
under one 30-second deadline. It interrupts active runner sockets and joins their
sessions, including stalled TLS handshakes. Separate runner applications retain
their own lifecycle. Validation: 283 Rust tests and both real-process signal
cases pass; the obsolete signal-flag test was removed. Release builds, formatting,
and Compose validation pass. All 28 Docker E2E scenarios ran: 25 pass, with the
same two multi-node certificate fixtures and one freshness assertion failing as
on the Step 01 baseline. Existing strict-Clippy findings remain. See
`docs/step-02-lifecycle.md` in Paranza.

Peerlo commit `c35c76d` drains HTTP and then runs ordered subsystem cleanup under
one 30-second deadline, including configurations with HTTP disabled. API task
failures are observable and shutdown joins the HTTP task. Internal subsystem
ownership and API-triggered bootstrap tasks retain their existing scope.
Validation: 782 Rust tests (six existing ignores), real-node SIGINT/SIGTERM
checks, shell swarm checks, and all 49 Python Docker E2E tests pass. Release
builds, formatting, and Compose validation pass; the 13 existing strict-Clippy
findings remain. See `docs/step-02-lifecycle.md` in Peerlo.

All three use the reviewed sibling source revision `c535907`, recorded in their
revision files and Docker build contexts. These commits are local; no deployment
or remote publication was performed.

Simple AI commit `e55f18f` adopts shared signals and a 30-second deadline in both
backend and inference runner. Backend HTTP drains before stopping affinity
invalidation and batch dispatch; tracked dispatched requests are joined. Runner
HTTP drains while its gateway connection/heartbeat work stops. Existing upgraded
backend WebSockets, detached jobs, and engine-process ownership retain their
scope. Validation: 441 Rust tests pass (one existing doctest ignored), including
SIGINT/SIGTERM during real-runner inference and batch-drain coverage. All four
Docker gateway E2E groups, the backend release build, and both release-container
signal checks pass. Compose, shell syntax, entry-point formatting, and diff checks
pass; existing workspace formatting differences and three common-crate Clippy
findings remain. CI and Docker builds use reviewed sibling source `c535907`;
remote publication is still required for fresh checkouts. See Simple AI's
`docs/step-02-lifecycle.md`. Both commits and validation are local; no deployment
or push was performed.

Simple Agents commit `0517053` coordinates HTTP, maintenance, execution dispatch,
and optional host wake/keepalive, then closes SQLite under the existing configured
budget (default 30 seconds). Active worker ticks finish cooperatively. External
Runner jobs, upgraded transport sockets, and detached engine monitors retain their
existing ownership/recovery semantics. Full `scripts/check` passes: 315 Rust
tests, strict Clippy/formatting, JavaScript/Android checks, all three browser E2E
scenarios, and repository consistency checks. Managed-handoff qualification,
x86_64 musl packaging, and static-container readiness/restart/signal checks pass.
The initial concurrent-build test deadline failure cleared on the full final run.
Unrelated Android development files remain uncommitted and untouched. See
`docs/step-02-lifecycle.md` in Simple Agents.

SCT commits `0322d36` and `bd21716` coordinate HTTP and writer-lease renewal under
one 30-second deadline, followed by writer release and catalog pool closure.
Renewal continues until HTTP drains; lease loss returns a nonzero exit. The
frontend scaffold also adopts shared HTTP shutdown. Full `scripts/check` passes,
including PostgreSQL integration, strict checks, client packaging, 49 contract
tests, frontend build, and three browser E2E scenarios (four existing future
scenarios pending). New process tests cover both signals, an 8 MiB response drain,
immediate database restart, and lease-loss failure. Local `master` now includes the migration and subsequent storage/tree work,
with transfer heartbeats retained alongside lease renewal and recovery stopped
before writer release. The checks above describe the original migration. See SCT's `docs/step-02-lifecycle.md`.

Androidoscopy commit `bb4033d` coordinates v2 controller HTTP, discovery/session
workers, and tracked device/action/socket tasks; legacy HTTP, WS/TLS, and UDP
also share signal-driven draining. Both modes use a 30-second deadline. CLI/MCP
stdio and remote Android processes retain their ownership. Validation passes
70 server tests, 12 full-stack tests, and six real-process signal cases spanning
v2, legacy WebSocket, and legacy TLS, including a stalled outbound TLS handshake.
Existing Clippy/formatting findings remain. See `docs/step-02-lifecycle.md`.

Pezzottflix commit `d2902d8` coordinates both HTTP listeners, queue/scheduled jobs,
tracked WebSockets and forwarding tasks, then SQLite cleanup. All phases now
share 30 seconds, including localhost operation. Validation passes 531 Rust tests
(three ignored), ten Docker backend E2E tests, both process signal/WebSocket-close
cases, and release backend/frontend builds. Existing Clippy/formatting debt
remains. Its `docs/step-02-lifecycle.md` records a corrected fixture-isolation
incident: the first process test used the local database override; its fixture
user/sessions were removed and verified absent. Final tests use isolated state.

Pezzottify-downloader commit `7355728` coordinates parent HTTP, connection
monitoring, status WebSockets/restart tasks, and child cleanup; the child Unix
HTTP server also drains connections. Streaming HTTP finishes before child
termination. Linux parent-death protection prevents an orphaned worker after a
hard parent exit, while audio/librespot internals retain their process ownership.
Validation passes 148 Rust tests, seven doctests (one ignored), 90 Python tests,
release Docker build and isolated SIGINT/SIGTERM checks. New process coverage
includes open upgraded sockets and hard parent death. Existing Clippy/formatting
findings remain. Credential-dependent Spotify operations were not exercised.
See `docs/step-02-lifecycle.md` in the downloader repository.

Crumbles commit `47062ec` coordinates HTTP, scheduler/outbox workers, accepted
WebSockets, and SQLite cleanup; the integration binary coordinates control HTTP
and daemon settlement/host-lock release under its existing configured budget.
Final outbox rows remain durable for replay, without guaranteed live publication,
and socket cancellation does not promise a Close handshake. Validation passes
1,367 Rust tests (two ignored), strict Clippy/formatting, unchanged generated
contracts, eight real-server browser scenarios, both release Docker builds, and
signal/drain/restart checks for both binaries and release containers. Local `master` now includes the migration and subsequent dispatcher work.
Native dispatcher cleanup finishes before SQLite closure. The checks above
describe the original migration. See Crumbles' `docs/STEP_02_LIFECYCLE.md`.

Quentin Torrentino commits `4cddf9d` and `7728402` coordinate HTTP, the V2
orchestrator, accepted WebSockets, pipeline jobs, and final audit flushing under
one 30-second deadline. Producer draining also runs after early audit-writer
failure; explicit audit receiver closure avoids waiting on idle sender clones.
External torrent services, FFmpeg and library internals retain their ownership.
Workspace results: 741 passed, two existing failures, 14 ignored; HTTP E2E remains
144/145. The recorded MusicBrainz mock mismatch and temporary-directory subtitle
assumption remain. New pipeline/audit regressions and real-process signals with
held WebSockets and durable final audit events pass. The original Rust 1.91
Docker toolchain is preserved; release/dashboard builds and isolated container
signal/audit checks pass. Existing Clippy/formatting findings remain. See
`docs/step-02-lifecycle.md` in Torrentino.

All 17 inventoried products now have locally verified, scoped Step 02 adoption.
Crumbles and SCT local `master` branches have been rebased onto their migration
branches, preserving their subsequent development. The reviewed shared source remains `c535907`; no
library implementation changed during these consumer migrations. No push or
deployment was performed.

## Step 03a: logging setup

Library implementation `57625c4` provides opt-in text/JSON logging, explicit
filters, output and ANSI policies, and fallible process-wide initialization.
It builds without HTTP or Tokio and installs no log-facade bridge. All-feature
and logging-only tests, strict Clippy, formatting, and the standalone example pass.

Favzetto pilot `eda9fc0` preserves stdout, target display, `NO_COLOR`, empty and
invalid-filter behavior, and its explicit log-facade bridge. The old/new process
comparison and lifecycle smoke tests pass. The backend retains its two baseline
API failures (131 unit and 93 API tests pass) and existing strict-Clippy findings;
Clippy completes with those findings capped at warnings. Browser and container
builds were not repeated. See Favzetto's `docs/08-logging-migration.md` and the
[logging contract](step-03a-logging.md). Both commits are local; no push or
deployment was performed. This pilot covered only 03a; 03b adoption is recorded separately below.

Crumbles canary `5bdd2b5` adopts the same library source in both the text
server/CLI and JSON integration daemon. Its environment policies and log bridges
are preserved; fresh-process comparisons include span-field filters, malformed
input, and JSON context. The untouched baseline passed 1,425 tests; the final
workspace passes 1,429 tests with two intentional ignores. Strict Clippy,
formatting, both binary builds, and SIGINT/SIGTERM drain/restart checks pass.
No library API change was needed. Browser/release-container checks were not
repeated; nothing was pushed or deployed. See Crumbles' `docs/STEP_03A_LOGGING.md`.

## Step 03a rollout applicability and workflow

All rollout work follows the [consumer migration workflow](consumer-migration-workflow.md):
verify capability use, work on a temporary branch/worktree from the active
service branch, test and commit, rebase that branch onto the migration, verify
integration, and remove only the integrated temporary branch/worktree. Both this
record and the HTML matrix must be updated. The initial rollout uses library
source `71755b5`, which adds pretty output while retaining the tested text/JSON
API. Peerlo uses `9f83845`, adding reloadable filters, compact output and optional
span events while preserving existing initializer defaults.

- **Paranza — N/A:** `apps/paranza-server/src/main.rs` emits explicit
  `println!`/`eprintln!` diagnostics; the production workspace has no logging
  subscriber/logger or direct tracing/log dependencies. No unused initializer is added.
- **SCT — N/A:** `crates/sct-server/src/main.rs` uses explicit stdout/stderr
  diagnostics without a tracing subscriber or logger. Its active inspection work
  remains untouched. Existing error request IDs are not logging setup adoption.
- **Peerlo — Done locally:** production `crates/peerlo/src/logging.rs` now uses
  the shared reloadable initializer. Its runtime API, compact output and CLOSE
  span events are preserved by the shared-library extension described below.

LelloAuth `ab38b81` adopts 03a in the server and all three runnable HTTP examples,
preserving pretty/JSON/text output, environment filters, stdout, color policy,
and explicit log bridging. Final validation passes 974 workspace tests (22
existing ignores), all example checks, formatting, fresh-process comparisons,
and real-binary startup/lifecycle checks. Clippy retains the same five findings
as the untouched branch; Docker/browser/provider checks were not repeated.
CI pins use `71755b5`. Local `master` was rebased onto the tested migration;
its temporary worktree/branch were removed and unrelated research files verified
unchanged by hash. See LelloAuth's `docs/STEP_03A_LOGGING.md`.

Androidoscopy `1caa8d5` migrates its existing legacy-server logger; v2 and MCP/CLI
entry points had no subscriber and retain their behavior. Validation passes 72
tests, 30 fresh-process policy combinations and six real-process signal/drain/
restart cases. Changed-file formatting passes; existing Clippy/dead-code and
unrelated formatting findings remain. Local `master` includes the tested tree,
uses source `71755b5`, and its migration worktree/branch are removed. See its
`docs/step-03a-logging.md`; Android/browser/container checks were not repeated.

LelloStore `d810104` migrates its backend's existing logger, retaining permissive
environment parsing, stdout, color, spans and log bridging. Its mock OIDC helper
has no logger to migrate. Final validation passes 133 tests (two existing ignored
doctests), strict Clippy and formatting; logging and lifecycle process tests pass
again after integration. Local `master` also preserves a newer Android fix,
replayed as `4eff608`; 47 unrelated WIP files were restored and verified by hash.
The migration worktree/branch and temporary stash are removed; a recovery ref
retains the pre-rebase tip. CI uses source `71755b5`. See its
`docs/STEP_03A_LOGGING.md`; Android/browser/container checks were not repeated.

Fausto `372d51e` migrates its production server logger with its original strict
filter/default, stdout, color, spans and log bridge. Server unit/integration
tests, 30 policy comparisons and three real-process lifecycle tests pass; one
existing doctest is ignored. Changed-file formatting passes; strict Clippy stops
on unchanged core findings. CI uses `71755b5`; local `master` contains the tested
tree and temporary worktree/branch are removed. See `docs/STEP_03A_LOGGING.md`.

Pezzottify-downloader `7960637` migrates both downloader and login startup.
Final validation passes 157 tests (one existing ignored doctest), 24 process
comparisons, and real-binary HTTP/WebSocket/shutdown/auth-failure checks.
Existing formatting and Clippy findings remain; external Spotify authentication
was not exercised. Source `71755b5` is pinned. Local `master` matches the tested
tree; migration worktree/branch removed. See `docs/step-03a-logging.md`.

Pezzottflix `1846e98` migrates its existing pretty/JSON logger while retaining
configured fallback levels, stdout, color, spans and log bridging. Final tests
pass 533 cases (three existing ignores), including 48 old/new policy comparisons.
The binary builds and both isolated SIGINT/SIGTERM HTTP/metrics/WebSocket drain
checks pass. New-file formatting passes; original formatting/Clippy findings
remain. Source `71755b5` is pinned. Local `master` matches the tested tree;
migration worktree/branch removed. See `docs/step-03a-logging.md`.
Browser/release-container checks were not repeated for these three migrations.

Pezzottify `c2d78c4e` migrates the production server and search-index builder,
retaining their different environment parsers/defaults, output and log bridges.
The full Rust suite passes 1,398 tests (36 existing ignores), including 60 policy
comparisons and four real-server lifecycle cases; indexer startup also reaches
its expected missing-catalog diagnostic. Changed-file formatting passes; one
unchanged strict-Clippy finding remains. Source `71755b5` is pinned. Local `dev`
matches the tested tree; migration worktree/branch removed. See its
`docs/step-03a-logging.md`.

Simple Agents `ce3b75e` migrates service text logging and runner-shell fixed-INFO
JSON logging. The two package suites pass 132 tests, 48 policy comparisons,
service signal/restart and real runner-shell startup checks. Full formatting
and strict package/all-target Clippy pass. Source `71755b5` is pinned. The
migration was integrated into `main` through a clean linked worktree; all 13
concurrent Android WIP files were verified unchanged and temporary worktree/
branch removed. Subsequent Android commit `b6e2daf` retains the migration as an
ancestor. See `docs/step-03a-logging.md`; Android/browser/container checks were
not repeated.

Quentin Torrentino `0a02cfa` migrates its production server logger while retaining
the original filter/default, stdout, colors, spans and log bridge. The final
serial workspace run passes 743 tests (14 ignored doctests), excluding two
MusicBrainz-name tests including the confirmed baseline search failure. Audit
startup timeouts under parallel load clear with all five cases passing serially.
The 24-case logging comparison, binary build and both signal/durable-audit
process checks pass. Existing formatting/Clippy findings remain. Source
`71755b5` is pinned; local `master` matches the tested tree and the migration
worktree/branch are removed. See `docs/step-03a-logging.md`; browser/container
checks were not repeated.

Observo `a481ae7` migrates the server's fixed-INFO stdout logger and the standalone
extractor's CLI-controlled WARN/INFO/DEBUG stderr logger; neither begins reading
`RUST_LOG`. The extractor enables only the shared logging feature, without HTTP
or lifecycle; link-scorer has no logger to migrate. Final validation passes 91
server tests, three extractor logging tests, 84 policy comparisons per adapter,
both binary builds, real-server HTTP/signal/listener-release and extractor
startup checks. The same three extractor library failures reproduce on the
untouched baseline (16 passes); existing formatting/Clippy and extractor
no-default-features compilation issues remain documented. Source `71755b5` is
pinned; local `master` matches the tested tree, with temporary worktree/branch
removed. See `docs/step-03a-logging.md`; browser/container checks were not repeated.

SimpleAI `36c650d` migrates backend and runner startup while retaining their
configured/INFO filter fallbacks, stdout text, colors, spans and log bridges.
Workspace tests pass 445 cases (one existing ignored doctest), including both
adapters' fresh-process comparisons; builds and changed-file formatting pass.
Strict Clippy retains three unchanged common-crate findings. Isolated backend
startup logs before expected loopback OIDC refusal; the runner starts with
engines disabled and shuts down on SIGTERM. An existing ignored config fixture
was needed for a compile-time parsing test and was not committed; no model or
remote inference operation was performed. Source `71755b5` is pinned. Local
`master` matches the tested tree and its temporary worktree/branch are removed.
See `docs/step-03a-logging.md`; browser/container checks were not repeated.

Meteonesto `32f16ba` migrates all three production initializers, preserving
API/gateway JSON with permissive environment filters and the pipeline's strict
configured text/JSON policy. Full suites pass 259 tests (one existing gateway
ignore); all six logging tests pass again after a mechanical lint adjustment.
All three builds, strict Clippy and formatting pass. Four existing API/pipeline
process tests pass with isolated data, including signals, admin shutdown,
database integrity and deadline/drain behavior. Gateway production startup
emits valid JSON before expected loopback HTTPS discovery failure; this does
not claim successful production health/shutdown without a trusted issuer.
Source `71755b5` is recorded in all component build instructions. Local `master`
matches the tested tree; migration worktree/branch removed. See
`docs/step-03a-logging.md`; container/provider checks were not repeated.

Final build-instruction review also corrected active README source references
in Fausto `0222a75`, LelloAuth `bb82dd2`, and LelloStore `1c8ed91`, each through
a separate documentation worktree. Their temporary branches/worktrees are
removed; existing user documents and README edits were preserved. Pezzottify's
quick-start now uses the revision-checking helper in `824f1dfb`, integrated into
`dev` through a separate documentation worktree that was then removed. Newer
concurrent Android/documentation edits remain in place.

Peerlo `6479504` adopts shared reloadable logging using reviewed library source
`9f83845`. Environment/configuration policy and API parser errors remain local;
empty filters disable logging, while whitespace-only or malformed filters are
rejected without changing the active filter. Its existing "pretty" name still
means text with CLOSE span events; production main still selects that format.
Compact/JSON output, ANSI, span context and log bridging match the original
logger in 84 fresh-process combinations, each exercising repeated live updates.
The untouched `c35c76d` baseline passes 782 workspace tests; final passes 784,
both with six existing ignores. Formatting, binary build and real loopback HTTP
filter-update/SIGTERM/SIGINT checks pass. Strict Clippy retains baseline findings;
all-target warning-capped comparisons introduce no logging findings. Docker,
full swarm and external-network checks were not repeated. Master was rebased
onto the migration; identical tree and ancestry verified, temporary worktree
and branch removed. See Peerlo's `docs/step-03a-logging.md`.

The shared library's full `scripts/check` passes, including strict Clippy,
feature combinations and reload/format/span process tests. LelloAuth `d5699c8`
also updates its test-only formatter fallback to tolerate the new Compact enum
variant: its three logging compatibility tests pass. That follow-up was
integrated into master through its own worktree, then cleaned up; unrelated
research documents were preserved and its production source pin is unchanged.

The assessed rollout now has **15 locally adopted products**, **two N/A products**
(Paranza and SCT), and **no Pending logging migrations**. The totals
include the two earlier canaries. All new migration commits are integrated and
their temporary worktrees/branches removed. Pezzottify uses `dev`, Simple Agents
uses `main`, and the other targets use `master`; remote HEAD is not the
branch-selection rule. Unrelated work and subsequent commits were preserved.
Nothing was pushed or deployed. Known baseline failures and verification
limits are recorded above and in each consumer's migration notes.

## Step 03b: request correlation pilot

Shared source `52e1922` implements optional request correlation with fresh IDs
by default, explicit bounded caller-ID acceptance, configurable header names,
request/response extensions and task-local access. It neither installs logging
nor rewrites bodies. Scope ends at response creation; spawned tasks and deferred
body/WebSocket work must capture the ID explicitly. The full library
`scripts/check` passes: strict Clippy, docs, feature combinations and seven new
correlation contract tests. See the [03b contract](step-03b-correlation.md).

Crumbles `91a1cb8` pilots adoption in the main HTTP server. Its existing
`x-correlation-id`, first-header validation, error envelopes, tracing span,
application extension, service context and audit use are preserved. Generated
IDs use the shared random generator, without the former counter fallback;
entropy initialization can panic as documented in the contract. The integration
daemon has no inbound correlation middleware to migrate; its existing outbound
client and durable business correlation keys remain application-owned.

Untouched master `5bdd2b5` passes 1,429 workspace tests with two ignores; final
passes 1,431 with the same ignores. New tests compare 24 original/shared
middleware cases and concurrent extension/context consistency. The same 15
real HTTP rejection cases pass before and after migration, verifying response
header/error-body agreement for valid, absent, empty, unsafe and oversized IDs,
followed by clean SIGTERM. Strict workspace/all-target Clippy, formatting and
diff checks pass. Existing frontend build assets were copied into the isolated
worktree for RustEmbed; browser, release-container and broader lifecycle checks
were not repeated. See Crumbles' `docs/STEP_03B_CORRELATION.md`.

Master was rebased onto the pilot branch, with identical tested tree and ancestry
verified. The temporary worktree and branch were removed; pre-existing worktrees
were preserved. No pushes or deployments. The pilot initially left the other
16 products Pending; the subsequent assessment is recorded below. 03a status
is unchanged and 03c is locally adopted by nine products, with eight N/A after assessment.

## Step 03b rollout applicability

Source `2740d5c` extends the default API with explicit application-selected
`HeaderRequestId`, scoped access, optional request-header insertion and response
overwrite/preserve policies. This permits existing UUID formats, validation and
rejection policies, opaque header values and repeated headers to remain intact.
It does not relax the default validated API. The full library `scripts/check`
passes, including strict Clippy/docs, independent features and ten correlation
tests. Applications retain their selection policy, error envelopes and tracing.
Crumbles' seven correlation tests also pass against the extended library,
including its original/shared middleware comparisons and error-header agreement;
its consumer source pin remains the original pilot revision.

The following eleven products do not currently need request-scoped correlation.
N/A is based on production entrypoints, middleware, header and ID use, not merely
dependencies. Existing source pins and executable behavior are unchanged.

| Product | Applicability evidence | Assessment / preservation |
| --- | --- | --- |
| Favzetto | Optional `x-request-id` is read for legacy runtime bridge events; no ID selection, scope or response propagation. An existing `api_flow.rs` test checks the supplied marker. | Inspected master `eda9fc0`; no files changed. |
| LelloAuth | Error-specific UUID incident references appear selectively in `x-error-id`, bodies and UI redirects. They are not a request-wide context; reauthentication IDs are durable business keys. | Inspected master `d5699c8`; three unrelated research documents preserved. |
| LelloStore | Backend router installs authentication, metrics, TraceLayer and CORS, without HTTP ID selection or propagation. | Inspected master `22881b3`; existing Android edits preserved. |
| Androidoscopy | Legacy/v2 HTTP routers have no correlation layer. Device call/cancel IDs bind protocol operations, not HTTP requests. | Assessment `f4461a8` integrated into master from `1caa8d5`. |
| Quentin Torrentino | Production router/auth/metrics record method/path/status/latency; error envelopes contain messages without request IDs. Business/WebSocket identities are separate. | Assessment `e606414` integrated into master from `0a02cfa`. |
| Pezzottify | `ApiError::new` creates per-error UUID references for log/body/header; no request-wide scope or handler extension. MCP/download IDs are domain identities. | Inspected dev `824f1dfb`; no files changed. |
| Simple Agents | Auth/session error responses generate `request-<32hex>` incident references in bodies only. Workflow/session IDs are domain identities. | Inspected main `a3e11bc`; existing tracked/untracked work and worktrees preserved. |
| Observo | Server router/auth and standalone extractor do not establish request ID headers/extensions or a correlation scope. | Assessment `fd16a9d` integrated into master from `a481ae7`. |
| Paranza | Management router has no request IDs. Runner command correlation spans protocol sessions and remains application-owned. | Assessment `2bd528f` integrated into master from `ac3126c`. |
| Peerlo | REST/Torznab router, metrics, TraceLayer, auth and rate limits do not select or propagate HTTP IDs. | Assessment `a142552` integrated into master from `6479504`. |
| Simple AI | HTTP logger records method/path/status/duration. UUID inference records are durable database/queue/cancellation keys, not middleware request IDs. | Assessment `7a745da` integrated into master from `36c650d`; newer Android icons and semantic-tool script preserved. |

The six documentation-only assessment commits used isolated worktrees, rebased
the original development branches onto their dedicated branches, verified trees
and ancestry, then removed those temporary worktrees/branches. Source inspection
and diff checks cover these assessments; application tests were not rerun for
unchanged executable code. Their `docs/step-03b-correlation.md` files hold details.
The other five N/A assessments made no repository changes and are recorded here.

Meteonesto `9a83f3e` adopts shared source `2740d5c` in its pipeline request envelope
and gateway dispatch/readiness failures. Pipeline retains 128-byte IDs, its
restricted alphabet and correlated 400 responses for malformed caller IDs.
Gateway retains generated `wg-` IDs and its existing uncorrelated successful
health/metrics responses. Weather API has no request correlation to migrate.
Validation: pipeline 74 tests, gateway 31 (one existing ignored E2E), unchanged
API 70: **175 passed**. Pipeline/gateway strict all-target Clippy and formatting
pass; other pipeline integration suites were not repeated. Master was rebased
from `32f16ba`, tested tree/ancestry verified, temporary worktree/branch removed.
See Meteonesto's `docs/step-03b-correlation.md` for baseline and scope details.

Pezzottify-downloader `d61b17d` replaces Puppeteer's request-ID layers with shared
scope/propagation at `2740d5c`. It preserves raw header bytes, UUID v4 generation,
repeated headers, tower extensions and downstream response overrides. The child
downloader and login tool have no corresponding request-ID layer to migrate.
Baseline: 157 Rust tests/doctests passed, one ignored; final: **159 passed**, one
ignored. Thirty paired old/new cases and generation checks pass. Real-process
HTTP 200/404/405 ID propagation, WebSockets and both shutdown signals pass.
Clippy matches the existing 11 library/12 unit-target warnings; changed files
are formatted and diff checks pass, while whole-repository formatting debt
remains. External authentication, browser/container checks were not repeated.
Master was rebased from `7960637`, exact tree/ancestry verified, temporary
worktree/branch removed. See its `docs/step-03b-correlation.md`.

Pezzottflix `39f6b51` adopts shared source `2740d5c` while retaining its local
RequestId extension, permissive first-string-header acceptance, UUID v4 fallback,
unchanged incoming headers and response-ID overwrite. Baseline: 533 tests pass,
three ignored; final: **535 pass**, three ignored. Production-router cases cover
legacy values, scope, repeated headers, overrides and 404/405. Real HTTP
correlation plus metrics, upgraded WebSocket and worker drain checks pass for
SIGINT and SIGTERM. All-target capped Clippy matches the 40 baseline warning
entries; changed-module formatting/diff checks pass, global formatting debt
remains. Browser/container checks were not repeated. Master was rebased from
`1846e98`, exact tree/ancestry verified, temporary worktree/branch removed.
See Pezzottflix's `docs/step-03b-correlation.md`.

Fausto `300e884` adopts shared source `2740d5c` in the production router while
preserving tower request/response extensions, arbitrary first-header values,
UUID v4 generation, repeated headers and downstream overrides. Audit/events
continue reading the existing request header. Baseline: 197 server tests pass;
final: **199 pass**, including 40 paired tower/shared compatibility cases and
generated-ID scope checks. Formatting passes. All-target Clippy completes with
existing warnings and no new correlation-module findings. Active README and
four CI checkout references use the reviewed source. Master was rebased from
`0222a75`, exact tree/ancestry verified, temporary worktree/branch removed;
checkout clean. See Fausto's `docs/step-03b-correlation.md` for verification scope.

SCT migration `5180c42` adopts shared source `2740d5c` in storage-backed HTTP,
preserving generated UUID v4 IDs in 32-hex form and ignoring caller IDs. Errors,
response normalization and observability share the scoped value. Storage-free
health/static responses retain their previous absence of an ID; API fallback
errors retain generated IDs. Final integrated-tree workspace all-feature tests
pass (31 tests, 65 qualification ignores, including the concurrent M4 addition).
Two explicitly run disposable-DB HTTP foundation/observability tests also pass.
Formatting and strict all-target, all-feature Clippy pass on the migration tree.
The no-storage HTTP test also verifies the unchanged header boundary. Browser,
S3, transfer-scale and complete container qualification were not repeated.

While integrating SCT from `117734c`, concurrent developer commit `186096e`
landed. Rebase preserved it as `2d1a921` on top of migration `5180c42`; range-diff
confirms the developer patch is unchanged. Original master is clean at
`2d1a921`; the dedicated worktree/branch are removed and pre-existing worktrees
are retained. Recovery ref `backup/pre-step03b-correlation-20260920` retains the
pre-rebase developer commit. See SCT's `docs/step-03b-correlation.md`.

The 03b assessment is complete locally: **six adopted products**, **eleven N/A**,
**none Pending**. All migration/assessment changes are committed and integrated;
only their temporary worktrees and branches were removed. Unrelated work was
preserved, including subsequently committed Pezzottify Android changes and
Simple Agents runtime-assets work. Nothing was pushed or deployed. Each product's
verification scope and existing limitations are recorded above; 03c is locally adopted by nine products, with eight N/A after assessment.

## Roadmap

The current [design roadmap](design.md#next-milestones-updated-2026-10-04)
records completion of all applicable active HTTP, lifecycle, observability,
auth, task, rate-limit and 07a/07b/07e database migrations. The detailed sections
below are dated implementation evidence, not an outstanding-work checklist.

07c backup coordination and 07d bounded synchronous execution are published
in crates.io 0.1.7; consumers await assessment/canary. Their new matrix
columns remain Pending assessment/canary. Backup canaries must preserve both
checkpoint and staged-copy strategies; executor canaries must preserve actual
priorities, lanes, cancellation and drain behavior. Quentin Torrentino remains
excluded. Custom/embedded protocol servers outside the matrix are not scheduled
for migration merely because they contain another backend library.

## Step 03c: HTTP tracing

Shared source `5ccbbabdc1c91b2869cb9090648b3cae4dac9f34` provides the opt-in
`http-tracing` feature. The [contract](step-03c-http-tracing.md) defines safe route
spans, status and header latency, body completion/error/cancellation, and upgrade
handoff. It works with an application-owned subscriber and optionally captures
validated correlation IDs. It never records raw URIs, headers or bodies by default.
HEAD and protocol bodyless responses finish at headers rather than producing
false cancellation warnings. Server-error headers and body errors retain ERROR
visibility; response-body outcomes are independent of status.

The final shared-library `scripts/check` passes: formatting, strict all-target /
all-feature Clippy, feature combinations, lifecycle/socket/signal tests, and
warnings-as-errors documentation. Twelve tracing tests pass with correlation;
ten pass independently without it. Tests cover actual router templates, fallback
and extension-method labels, header/body timing, data/trailers, size hints,
errors, cancellation phases, HEAD/bodyless statuses, upgrade handoff, callback
and body span context, concurrent IDs and opaque-ID exclusion.

Crumbles **master `4a44b33`** adopts it in the main HTTP server. Applicability was
its production `TraceLayer::new_for_http()` plus application correlation span;
the integration daemon and runner have no HTTP tracing to migrate. The shared
callback sits inside the correlation scope and includes canonical response
normalization, retaining the existing `correlation_id` parent field and matching
it with the shared `request_id`. It replaces the Tower tracing layer and trace
feature. CSRF rejections are now inside tracing; outer CORS short-circuits remain
outside. Status, headers, bodies, ID validation/generation, audits, auth, metrics
and shutdown contracts are retained. Telemetry intentionally adopts the shared
safe-field and timing schema; the application logging initializer is unchanged.

Verification evidence:

- Baseline main-server tests: **529 passed, two existing ignores**. Initial
  sandbox socket restrictions were resolved by running with local socket access.
- Workspace suite: **1,433 passed, two existing ignores**. After the final HEAD
  refinement, the affected Crumbles package was rerun: **588 binary tests plus
  66 CLI integration tests passed**, retaining the two existing ignores.
- Final strict workspace all-target Clippy, formatting and diff checks pass.
- Two new production canary tests cover status/error/ID agreement, safe fields,
  HEAD, no duplicate Tower logs, and an authenticated real WebSocket handshake:
  one correlated 101 event before close and no HTTP lifetime event at close.
  Existing legacy correlation comparisons continue to pass.
- Real-process checks pass: 15 rejection cases, safe health-query and HEAD
  requests, shared header/body log fields and correlation, plus both binaries'
  SIGINT/SIGTERM behavior, active HTTP drain and immediate restart.
- The existing ignored frontend `dist` was copied unchanged for Rust embedding.
  Frontend rebuild, browser, Docker and deployment qualification were not repeated.

The dedicated worktree/branch started from clean active master `91a1cb8`.
`simple-server.rev` pins the reviewed source above. The migration was committed,
master rebased onto it, and ancestry plus exact tree equality verified. Original
checkout is clean; the temporary worktree and branch are removed. Pre-existing
worktrees/branches are retained. See Crumbles' `docs/step-03c-http-tracing.md`.
At pilot completion, both trackers showed **one Done canary and sixteen Pending assessments**; the rollout assessment below supersedes those counts.
No pushes or deployments were performed.

## Step 03c rollout assessment

Reviewed shared source `adc1640bde4ac8f934ed454c8d6c5e264a6a2790` adds
`trace_with_observer`, preserving application-owned event sinks, response fields
and severity without installing a subscriber or duplicating HTTP events. The
safe span and body lifecycle remain shared. `trace` retains its default behavior.
Full `scripts/check` passes, including 14 HTTP tracing contract tests (12 without
correlation), strict Clippy, formatting and documentation. Observer tests verify
read-only response metadata, replacement of default events, and terminal
callbacks even without a subscriber. The already-integrated Crumbles production
HTTP and real WebSocket canary tests also pass against this extension; its
historical source pin is unchanged.

Eight additional products had actual request tracing/logging and are now migrated:
Fausto, LelloStore, SCT, Pezzottify, Peerlo, Simple AI, Pezzottflix and the
downloader. Adoption is recorded only after testing, committing and integration. The following eight
are N/A after source assessment; no dependencies or application behavior were
changed merely to enable tracing.

| Product | Development branch / assessed commit | Applicability evidence |
| --- | --- | --- |
| Favzetto | master `eda9fc0` | Backend final router has rate-limit and body-limit layers; domain events but no request-wide tracing/logger. |
| LelloAuth | master `d5699c8` | Server security headers, cookies and metrics middleware; HTTP histograms are not request spans/events. Three unrelated research documents preserved. |
| Androidoscopy | master `f4461a8` | Legacy and control routers have auth/device events, no HTTP lifecycle logger or spans. |
| Quentin Torrentino | master `e606414` | API middleware records Prometheus request counts/durations only; domain events remain local. |
| Simple Agents | main `e054d4b`, rechecked at `646f3f1` | Browser boundary, auth/session and domain logs; no request lifecycle observer. Existing harness/runner/service/web work was preserved and subsequently committed externally; production router unchanged. Other worktrees preserved. |
| Meteonesto | master `9a83f3e` | Weather API metrics, pipeline envelope/audit and gateway metrics/admission/problem diagnostics; selective errors are not a request lifecycle logger. |
| Observo | master `fd16a9d` | Production router layers body limit, auth and CORS; metrics endpoint and domain logs only. |
| Paranza | master `2bd528f` | Management API router has no request tracing/logger; runner domain events are separate. |

These N/A assessments used source inspection, not runtime test runs. No service
worktree or branch was needed for them because no consumer files changed.
Pezzottify `dev` is integrated at **`6998803214f3df5f85a22d54fdb7a19b1f17d158`**,
from `b82e17d4`, using shared source `adc1640`. Production request logging retains
None/Path/Headers/Body selection, INFO response-header visibility for all statuses,
opt-in redacted diagnostics, metrics/bandwidth accounting and separate incident
IDs. The shared observer replaces request lifecycle events with safe route spans
and header/body timing; terminal events use the shared default policy.

Baseline: 1,103 library tests pass, two existing ignores. Final: **1,122 passed,
two ignored** (1,103 library, one actual-wire all-mode tracing, four production
lifecycle, three route-contract and eleven streaming/range tests). Lifecycle
checks cover SIGINT/SIGTERM/reboot/bind failure and WebSocket drain. Changed-file
formatting and diff checks pass. Strict Clippy finds only the unchanged
`enrichment_store/works.rs:247` `items_after_test_module` finding; all library/test
targets pass with that single lint exempted and all other warnings denied. Other
integration suites and Android/browser/container builds were not repeated.
See Pezzottify's `docs/step-03c-http-tracing.md` for exact commands and boundaries.
The original development branch was rebased onto the migration, exact tree and
ancestry verified, temporary worktree/branch removed; checkout clean. No pushes.

Fausto migration is integrated at **`4ad3ff4a7fb36c501f325de278e3faf5d49cd4ef`**
from `300e884`, using `adc1640`. Its production default Tower tracing layer is
replaced in place, inside correlation and outside CORS/auth/rate limits/plugins
and static fallback. HTTP and opaque-ID contracts remain unchanged. Shared safe
spans and header/body events intentionally replace raw URI / DEBUG Tower output;
Fausto still owns filtering, audits and metrics. Baseline **199 passed / one
ignored doctest**, final **200 / one ignored**, including real process signals,
HTTP draining, logging compatibility and the new production-router 200/405
safe-field/correlation check. Full formatting and all-target Clippy complete;
Clippy retains existing repository warnings (not a warning-free claim). README
and four active CI pins are updated. Frontend/container/external OIDC and separate
exhaustive WebSocket load checks were not run. Original master is clean. A final
review found that Fausto's scoped fallback excluded the shared target. Follow-up
**`7b494b7e1780b83f267ec0a28ff5e42e57eb7d73`** fixes only that fallback by adding
`simple_server::http_tracing=info`; explicit `RUST_LOG` remains authoritative.
The real-process regression first reproduced missing events, then passed default,
service-only and shared-error-only filter cases with real 200/500 responses.
Seven focused checks pass (router tracing, four process/lifecycle and two logging
compatibility tests); full formatting, diff and focused Clippy checks pass with
existing unrelated warnings. The original full suite remains 200/one ignored;
a full 201-test suite rerun is not claimed. Master was rebased onto the dedicated
fix branch, exact tree/ancestry verified and its temporary worktree/branch removed.
Final Fausto master is `7b494b7`, clean.

LelloStore master is integrated at **`d2276292ceb349d03c61bcb54d72554bd2b6528a`**
from `22881b3`, using `adc1640`. Backend default Tower tracing is replaced outside
metrics and inside CORS, covering auth/fail-closed API, health/admin and static
fallback; outer CORS short-circuits and the separate metrics listener retain their
scope. No IDs/subscriber changes are added. Baseline **133 passed / two ignored
doctests**, final **134 / two ignored**. The new production-router test covers
200/405/503, safe fields, no new IDs and no duplicate Tower events; the existing
suite covers API/file ranges/startup/signals/draining/logging configuration.
Changed-file formatting and all-target Clippy pass, with no Clippy warnings.
README and active CI pins are updated. Android/frontend/external OIDC/Docker/APK
checks were not run. Fourteen unrelated Android/backend WIP files and the entire
working diff were verified unchanged after integration; those user edits were
not part of the tested migration tree.

SCT master is integrated at **`3343fbed2fae113f5c5628ae9dad516e244011c4`** from
`2d1a921`, using `adc1640`. Storage-backed `application()` uses the shared observer
for header timing and retains its unconditional metadata-only stderr JSON event,
including request ID and approved error code. Existing metrics keep their labels,
buckets and header-time active gauge. There is no new subscriber, duplicate
event, or body-completion output; its observer deliberately leaves completion
silent. No-storage `router()` and domain/startup/worker/CLI output remain unchanged.
Baseline **31 passed / 65 PostgreSQL/S3 ignores**, final **33 / 65 ignored**.
New exact JSON-schema and real-loopback pending-stream tests verify response code,
ID policy, 405 and metrics at headers without a subscriber. Strict all-feature /
all-target Clippy, full formatting and diff checks pass. Database-backed router,
S3/browser/container recovery qualification was not repeated; active user
qualification fixtures were not touched. Sixteen WIP file hashes and the entire
working diff were verified unchanged after integration.

All three original masters were rebased onto their dedicated migration commits;
ancestry and exact tested trees verified, temporary worktrees/branches removed.
Dirty LelloStore/SCT checkouts were integrated through clean linked worktrees
without stashing. Pre-existing worktrees remain. Each service's
`docs/step-03c-http-tracing.md` records commands and boundaries.

Peerlo master is integrated at **`907c9ad246c79fa71ffd905566075301d9bf4708`**
from `a142552`, using `adc1640`. Production `create_router` replaces both Tower
tracing and duplicate request events from metrics middleware. It preserves INFO
headers below 400, WARN for 4xx and ERROR for 5xx; shared body outcomes are separate.
Metrics retain endpoint normalization/counters/histograms at header creation.
Outer CORS/auth/rate-limit short-circuit placement is unchanged; no IDs are added.
Baseline **165 passed**, final **167 passed**. New production-router tests cover
200/400/503/404/HEAD, safe labels/severity, metrics at headers and one cancellation
on dropped bodies. Existing loopback HTTP/auth/rate-limit/shutdown tests pass.
All-target Clippy exits successfully with existing warnings in unchanged code;
an introduced item-order warning was fixed and Clippy rerun. Changed-file rustfmt
and diff checks pass. Full workspace and remote tracker/DHT deployment checks
were not run; repository-wide formatting is not claimed. Original master is clean.

Simple AI master is integrated at **`2b9c6a6165a764c650836b3e89638a58239f1fe8`**
from `11462d6`, using `adc1640`. The existing backend request middleware delegates
to shared tracing with an observer preserving INFO header events for all statuses.
It covers the same gateway/auth/rate-limit/runner-WebSocket-handshake scope;
inference/audit events remain local, and the inference-runner binary has no
HTTP tracing layer to migrate. Baseline **310 passed / one ignored doctest**,
final **312 / one ignored**. New adapter tests cover severity, safe matched routes,
unchanged headers/bodies, lazy SSE and exactly-once completion/cancellation;
existing smoke/auth/backend tests pass. All-target Clippy succeeds with 12 backend
and three common-library warnings in unchanged files, no new adapter/test findings.
Changed-file formatting/diff checks pass. Full inference-runner, Android, GPU,
browser and deployed-OIDC checks were not run; full-repository formatting is not
claimed. All twenty unrelated working files, including README and semantic-scoring
scripts/docs/fixtures, were verified byte-identical after integration.

Both base masters were rebased onto their dedicated migration branches, exact
trees and ancestry verified, temporary branches/worktrees removed. Simple AI used
the clean linked-worktree integration path; existing worktrees are retained.
Their `simple-server.rev` files and service migration docs record reviewed source.

Pezzottflix master is integrated at **`f0557a29117de03e3abdb10ef6474fa4a514531e`**
from `39f6b51`, using `adc1640`. The main production router delegates its existing
request logger to shared observation; the separate metrics listener is unchanged.
Header severity remains INFO below 500 and WARN for 5xx. Its parent request span
retains conventional bounded IDs and redacts other opaque IDs in telemetry only;
HTTP ID semantics remain unchanged. Raw paths/queries become safe route labels,
with header timing and body completion/error/cancellation/upgrade outcomes.
Baseline **535 passed / three ignored**, final **536 / three ignored**. New
telemetry contracts cover severity, ID shapes, privacy, exactly-once events and
unchanged bodies/headers/extensions. Real-process SIGINT/SIGTERM checks pass,
including HTTP, metrics, authenticated WebSocket drain and visible, safe tracing.
Capped all-target Clippy completes with existing 33 library / 37 including unit
warnings and three integration findings; strict warning-free lint is not claimed.
Changed Rust modules/tests and whitespace checks pass; unrelated formatting debt
remains. Browser/frontend builds, external providers and release containers were
not run.

Pezzottify-downloader master is integrated at
**`2bf0158a627437fd366163c7458bb8c0de33403b`** from `d61b17d`, using `adc1640`.
Both downloader-child and Puppeteer HTTP routers replace custom Tower tracing
with shared observation, preserving their separate severity policies and optional
numeric content length. Puppeteer retains approved bounded parent-span IDs;
opaque IDs are redacted only in telemetry. HTTP correlation and body behavior
remain unchanged. Default filtering enables the shared target, while explicit
`RUST_LOG` remains authoritative. Baseline **159 passed / one ignored doctest**,
final **161 / one ignored**. New checks cover both components, status/ID classes,
content length, privacy and pass-through behavior. Fresh-process filter checks
and actual Puppeteer HTTP/WebSocket/SIGINT/SIGTERM telemetry checks pass. Capped
all-target Clippy completes with unchanged 11 library / 12 including unit-test
warnings; strict warning-free lint is not claimed. New tracing and changed
logging/test files pass rustfmt; router files retain existing formatting debt.
Whitespace checks pass. Authenticated Spotify child startup, browser/provider
workflows and release containers were not exercised.

Both original masters were rebased onto their dedicated migration branches;
ancestry and identical tested trees verified, temporary worktrees/branches
removed, original checkouts clean. Their service-local
`docs/step-03c-http-tracing.md` records behavior and verification limits.

Final rollout: **nine Done, eight N/A, zero Pending**. All implementation and
consumer migration changes are committed and integrated into the original active
development branches. Temporary migration worktrees/branches are removed;
unrelated user work and pre-existing worktrees are preserved. Nothing pushed or
deployed.


## Step 04a: extractor body limits

Shared source **`0b945750b6b97a9e18c531d1cf1137d4ed4b69c9`** adds the optional
`body-limit` feature and `BodyLimit::max(bytes)`. Values, route placement and
extractor rejection behavior remain application-owned. It does not impose an
unconditional wire-body ceiling or buffer/limit responses. See the
[contract](step-04a-body-limits.md). Full `scripts/check` passes, including strict
Clippy, formatting, warnings-denied rustdoc, minimal feature builds and four
body-limit contract tests (three without multipart), plus the API doctest.
Differential tests cover zero/below/at/above limits, JSON failures, absent and
declared Content-Length, multipart, route overrides, raw reads and lazy responses.
Loopback lifecycle tests required sandbox escalation, then passed.

Simple Agents main is integrated at **`b8c4276`**, from `62b44a3`, using shared
source `0b94575`. All nine production extractor-limit declarations in the service
now use the shared API with unchanged values and placement. Baseline service
suite: **126 pass**; final: **127 pass**, including the new production-router
Bytes/JSON test, passing before and after migration. It checks below/at/above
5 MiB session and 4 KiB broker limits, present/absent lengths, unchanged 413 text
and no-store headers. Existing real-process, WebSocket, SSE and auth tests pass.
Strict all-target service Clippy, full workspace formatting and diff checks pass.
Whole-workspace tests, browser/container builds and external-provider checks were
not run. These results apply to the committed migration tree, not concurrent WIP.

Original main was rebased onto the dedicated migration branch in the clean
worktree; identical tested tree and ancestry were verified. On restoration, Git
merged the overlapping fleet route file without conflicts. All 15 recorded WIP
files were verified byte-identical except the intended body-limit substitution
in that file; the new power-status route and all user edits are retained. Existing
worktrees are preserved; the temporary migration worktree/branch are removed.
Service-local `docs/step-04a-body-limits.md` records scope and commands. During
final verification, concurrent work was committed externally as `5e46aa0`;
main still contains the migration by ancestry. Its additional changes were not
part of the migration test run.

Pezzottify dev is integrated at **`b5c96887`**, from `69988032`, using shared source
`0b94575`. All three production declarations now use BodyLimit: legacy bug reports
2 MiB, reports 20 MiB and ingestion multipart 5 GiB. Values, placement, custom
admission and rejection behavior are unchanged. Baseline: **1,103 library tests
pass, two existing ignores**, plus **17 affected HTTP tests** (including the new
canary) passing before migration. Final: **1,120 passed, two ignored** across the
library and body-limit/ingestion/reports/streaming suites. The canary exercises
below/at/above JSON ceilings and a 3 MiB multipart body reaching filename validation;
a full 5 GiB runtime upload was not attempted. An initial fixture permission
mismatch was corrected before migration; production authorization was unchanged.
Eleven existing media-stream/range cases pass. Library/test Clippy passes with
only the previously documented items-after-test-module lint exempted; existing
num-bigint-dig future-incompatibility remains. Changed-file formatting and diff
checks pass. Other integration suites, frontend/Android, containers, provider
workflows and production-scale uploads were not rerun.

Original dev was rebased onto the dedicated migration branch; ancestry and
identical tested tree were verified, temporary worktree/branch removed and
original checkout clean. See its `docs/step-04a-body-limits.md` for exact scope.

At pilot completion, 04a totals were **two Done, fifteen Pending assessment/migration**;
the rollout record below supersedes these counts. The five-service
inventory establishes additional candidates, not completed migrations or N/A.
04b/04c remain Planned. Shared implementation, pilot and canary are committed and
integrated into their original development branches. Nothing pushed or deployed.

## Step 04a remaining-service rollout

The following six products have no explicit extractor body-limit policy to migrate.
Framework defaults remain in effect; protocol limits, raw-body reads and bandwidth
settings are not replaced with extractor middleware. Assessments used production
source inspection, not runtime test runs. No consumer files changed.

| Product | Branch / inspected commit | Evidence |
| --- | --- | --- |
| androidoscopy | master `f4461a81` | Legacy WebSocket routers and control::router auth layer have no explicit extractor limit. |
| fausto | master `7b494b7e` | server/src/api/mod.rs composes auth/rate limits, CORS, tracing and plugins without explicit extractor limits. |
| paranza | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Private routing, state/path/query/raw-body extraction, response conversion, HTTP serving and test helpers use shared APIs; Axum remains internal to simple-server. |
| peerlo | **Done (local)** | None. |
| pezzottify-downloader | master `2bf0158a` | Both production HTTP routers install CORS/tracing (Puppeteer also correlation); no explicit extractor limit. Proxy body handling is separate. |
| quentin-torrentino | master `e6064149` | crates/server/src/api/routes.rs composes auth, metrics and static fallback without an extractor limit; torrent bandwidth limits are unrelated. |

Nine additional products have explicit limits. Their verification and integration
evidence follows. All use reviewed shared source `0b94575`; no shared-library
changes were needed for this rollout.

### crumbles

Migration **`f74f6a8b5e9fd865b99f79370e29a7f3d59f4f0f`**, based on master `4a44b330`.
Main HTTP upload ceiling (configured upload size plus multipart overhead), setup ceiling and Simple Agents browser proxy ceiling. All three declarations preserve exact values and placement.

Baseline: 588 passed, two ignored in the crumbles binary suite. Final: 589 passed, two ignored. New production setup-route boundary regression passed before and after migration: below/at/above limits, declared/absent lengths, correlated 413 code and response header. Initial fixture lacked required same-origin headers; corrected before migration. Existing ignored web/dist assets were copied for compile-time embedding; no fresh frontend build.

Strict all-target package Clippy passes. Changed-file formatting and whitespace checks pass.
Other workspace packages, full release/frontend/container builds and external providers were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 0 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### favzetto

Migration **`22b6a2d2fb356079982ea63d8df17e6a53395f6c`**, based on master `eda9fc0a`.
The backend router retains its existing 128 MiB extractor limit outside rate limiting. No global policy changes or new limit is introduced.

Both baseline and migrated runs: 131 unit tests and 93 API tests pass; the same two API tests fail: catalog_research_runtime_bridge_approves_runtime_draft and catalog_research_runtime_bridge_rejects_runtime_draft. Both reject transitions from terminal catalog flows with the same messages/statuses. Cargo stops at that failing integration target, so later targets are not claimed. Existing ignored web/dist assets were copied for compile-time embedding.

All-target Clippy completes with capped warnings; existing catalog/runtime/test warning debt remains. Strict warning-free lint is not claimed. Changed-file formatting and whitespace checks pass.
The unrelated catalog-flow failures were not fixed. Frontend, Docker, provider workflows and later integration targets were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 0 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### lello-auth

Migration **`758db1b975a338664b3ad99436cf774d8f694005`**, based on master `d5699c81`.
Three 64 KiB authenticator API/hosted/enrollment declarations. Existing response mapping, no-store and CSRF/auth ordering are retained, including conversion of 413 extractor rejection into 400 invalid_request.

Baseline affected HTTP/server packages: 357 passed. Final: 358 passed. New authenticator regression passes before/after replacement with valid requests padded below/at/above 64 KiB; overflow retains the exact application error JSON and no-store response. Existing authenticator, login, cookie, CSRF and logging tests pass.

Strict Clippy finds existing core large_enum_variant and too_many_arguments warnings. Capped all-target Clippy is used to review remaining targets; existing warnings remain. Changed-file formatting and whitespace checks pass.
Core-only suites, external OIDC/deployed E2E, browser and container builds were not rerun. Three unrelated research documents are preserved.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 3 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### lellostore

Migration **`54615a10126284ec8464fc8f6470a5e04d57050b`**, based on master `d2276292`.
Backend configured multipart request ceiling is unchanged, including its existing overhead calculation and route placement. Per-file/APK validation remains local.

Baseline and final: 134 passed, two ignored doctests. Existing API, upload/file, process, lifecycle and logging tests pass.

Strict all-target Clippy passes. Changed-file formatting and whitespace checks pass.
Android/frontend/container and external-provider checks were not repeated. Original Android/backend working edits are excluded from the tested migration tree and preserved.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 14 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### meteonesto

Migration **`685094181f03e9e9fe0d920d03a5f26fc24c66a1`**, based on master `9a83f3e1`.
Only weather-pipeline control-plane DefaultBodyLimit is replaced, keeping envelope.max_body_bytes. Declared-length checks, admission permits, correlation, problem responses and header-time timeout stay application-owned. Weather API and gateway have no equivalent declaration to migrate.

Baseline and final pipeline suite: 161 passed. Existing HTTP control-plane, envelope timeout/capacity, correlation and application contract suites pass.

Strict all-target pipeline Clippy passes. Changed-file formatting and whitespace checks pass.
Weather API/gateway suites, Android/renderers, live weather providers and container builds were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 0 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### observo

Migration **`ef3bff39964d0adb6641dd2d659f04ec2c518f43`**, based on master `fd16a9dd`.
The production server router retains its existing 64 MiB extractor limit. Content-extractor/link-scorer binaries do not acquire body-limit middleware.

Baseline and final server suite: 91 passed. The initial baseline compiler overflowed while building headless_chrome; retry with RUST_MIN_STACK=16777216 passes and the same setting is used after migration.

All-target Clippy completes with capped warnings; existing server/extractor lint debt remains. Strict warning-free lint is not claimed. Changed-file formatting and whitespace checks pass.
Live headless-browser/provider, frontend and Docker E2E workflows were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 0 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### pezzottflix

Migration **`ca8a370e6a5ce071e818d0e2d57321303c63d9e3`**, based on master `f0557a29`.
Main production router retains the 100 MiB upload extractor limit at the same layer position. Metrics router, auth, media streams and WebSockets retain their existing behavior.

Baseline and final server suite: 536 passed, three ignored. Existing production HTTP/media, tracing/correlation and lifecycle contract tests pass.

All-target Clippy completes with capped warnings: existing 33 library / 37 including unit-test warnings plus existing integration findings remain. Strict warning-free lint is not claimed. Changed-file formatting and whitespace checks pass.
Separate real-process lifecycle script, frontend/browser/release-container and external-provider checks were not repeated.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 0 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### sct

Migration **`90c02a4d7bbea3424084a8fda74fab0d68cfc579`**, based on master `3343fbed`.
Storage-backed application router retains its explicit 65,536-byte extractor limit. Existing problem response, correlation, auth, content-type and observability behavior remains local. No-storage router has no explicit limit to migrate.

Baseline and migrated sct-server suite: five passed, eleven existing PostgreSQL/S3 integration tests ignored. After preserving the concurrent qualification commit, the combined final tree passes five tests with fifteen ignored, and strict all-target/all-feature Clippy passes again. No database-backed production HTTP qualification is claimed.

Strict all-target/all-feature server Clippy passes. Changed-file formatting and whitespace checks pass.
Database-backed HTTP, PostgreSQL/S3, browser and container qualification were not repeated. Active original qualification work/staging and pre-existing worktrees/stash are preserved.

Original master rebased onto the migration and now points to **`d3961d4c14bda4d22a6dec5cf0f349511b1c6a16`**. Concurrent developer commit `be48680d` was replayed as `d3961d4`; `git range-diff` verifies its patch is unchanged. Recovery branch `recovery/step04a-before-rebase-be48680d` is retained. Migration ancestry and the combined tested tree were verified; temporary migration worktree and branch removed. Pre-existing worktrees and stash remain untouched.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

### simple-ai

Migration **`f511523922aa33231898f7bf8f66e6a872834e04`**, based on master `2b9c6a61`.
Both production components: backend OCR/extract keep 25 MiB, backend audio 200 MiB; runner OCR 100 MiB and runner audio 200 MiB. All five declarations preserve placement. Domain file validation, inference scheduling, SSE and WebSockets remain local.

Baseline and final: backend 312 passed, one ignored doctest; inference runner 87 passed. Combined final 399 passed, one ignored. Runner compilation initially required the existing ignored scripts/configs/rtx.toml fixture, copied unchanged into the isolated worktree. That local fixture is not committed.

All-target Clippy completes with capped warnings: existing 12 backend, three common and five runner warnings remain. Strict warning-free lint is not claimed. Changed-file formatting and whitespace checks pass.
GPU/model/provider, Android/browser and container/release workflows were not rerun. Original semantic-evaluation working files are preserved.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary branch/worktree removed. 20 unrelated working files
were preserved byte-for-byte; staged/unstaged status also verified unchanged.
See its service-local `docs/step-04a-body-limits.md` for commands and scope.

Final rollout: **eleven Done, six N/A, zero Pending**. All applicable migrations
are committed, integrated into their development branches, and their temporary
migration worktrees and branches removed.
At completion of 04a, 04b/04c remained planned; the 04b record below supersedes
that implementation status. No pushes or deployments.


## Step 04b: response headers

Shared source **`b88b908421db2552ff1e81966c56958925741e27`** implements the optional
`response-headers` module. It builds without Axum, lifecycle or logging; its
normal minimal dependency graph contains only http, bytes and itoa. Full
`bash scripts/check` passes, including formatting, strict Clippy, rustdoc, the
feature matrix and five new contracts (four in the minimal feature build), plus
the new doctest. Tests cover repeated values and sensitivity flags, invalid
input, Vary duplicates/wildcards and lazy data/trailer/error frames without
changing response status, version or extensions. Shared main was rebased onto
the verified feature branch and its temporary worktree/branch removed.

The [04b contract](step-04b-response-headers.md) separates defaults from explicit
replacement. Vary merging intentionally retains existing field lines and opaque
bytes, appending missing names on separate lines. This improves preservation
compared with Pezzottify's old normalizing helper; consumers must read all Vary
values as a combined list. Cache eligibility, header values, route/layer placement,
CSP generation, ETags, response construction and application policy stay local.

### Pezzottify canary

Migration **`a25b0c3aa65c543f6ddc220f4f7105aafdfb365d`** is integrated into **dev**,
based on `0b258266`. The actual API/cache middleware calls shared default,
replacement and Vary operations. Explicit Cache-Control remains authoritative;
private caching still excludes errors, partial/media/SSE responses and mutations.
The outer API safety net still covers early auth/CSRF/rate-limit failures with
unchanged `/v1` selection. Other domain-specific response headers remain local.

Baseline: 1,103 library tests plus 43 actual-HTTP tests pass, two existing library
tests ignored. Two additional middleware regressions also passed before
replacement. Final: **1,148 passed, two ignored** (1,105 library, 26 catalog,
17 body-limit/ingestion/report/streaming HTTP checks). Tests cover HEAD, repeated
Cache-Control/Set-Cookie, Vary credential lists, early errors, API path boundaries,
partial/media/SSE classification, body-limit rejections and actual range streams.
Shared tests additionally cover wildcard/opaque Vary and lazy trailers/errors.

Clippy passes with the existing `items-after-test-module` exemption; the existing
num-bigint-dig future-incompatibility notice remains. Changed-file formatting and
whitespace checks pass. Other integration suites, frontend/Android, containers,
live providers and production-sized uploads were not rerun. Source pin and
lockfile updated. Original dev rebased onto the migration; ancestry and identical
tested tree verified, temporary worktree/branch removed, existing worktrees kept.
See Pezzottify's `docs/step-04b-response-headers.md` for exact commands and scope.

### Simple Agents canary

Migration **`61b36f86123ec1453586540304f183f9db83a907`**, based on main `5e46aa0`, is
integrated into **main at `0def27fb528d99cd8eaf2f70fb6f1ba40e923bb7`**. Eight route
groups use shared replacement via a local no-store adapter; browser/native auth
also retain their no-referrer replacement. Handler-specific response tuples and
release/metrics/UI response construction remain local; adoption is scoped to the
reusable route policies. Only the service opts into this feature.

Baseline and final service suites: **131 tests pass**. Added assertions verify
no-store on successful session creation/replay and SSE before/after migration.
Existing production-router checks cover body-limit boundaries with and without
Content-Length, unchanged 413 bodies and headers on auth/limit errors. Existing
checks also cover browser/native auth, cookies, process signals, WebSockets and
SSE revocation. Strict all-target Clippy and full formatting pass. Tests use
disposable databases and local mock listeners. Whole-workspace, browser/Android,
external providers, containers and deployment checks were not rerun.

During integration, concurrent frontend commit `396efc6` was replayed as
`0def27f`; range-diff verifies its patch is unchanged. Recovery branch
`recovery/step04b-before-rebase-396efc6b` is retained. Rebase used the clean linked
worktree; seven unrelated working files and their staged/unstaged status were
verified unchanged. The combined final tree passed the same 131 service tests,
strict all-target Clippy and full formatting. Migration ancestry verified;
temporary migration worktree/branch removed, pre-existing worktrees retained.
See Simple Agents' `docs/step-04b-response-headers.md` for commands and scope.

At canary completion, 04b totals were **two Done (scoped canaries), fifteen Pending assessment/migration**;
the rollout record below supersedes those counts.
Both source pins record the reviewed shared revision above. HTML and Markdown
matrices agree; no other service has been marked adopted or N/A without
assessment. 04c remains planned. No pushes or deployments.



## Step 04b remaining-service rollout

Eight additional products have applicable cache/security-header policies. Their
production header operations use reviewed shared source
`b88b908421db2552ff1e81966c56958925741e27`; no shared API extension was needed.
Applications retain policy values, placement, eligibility and response behavior.
Seven other products are N/A after source assessment; no runtime tests or
behavior changes are claimed for those assessments.

| Service | Assessed development branch | N/A evidence |
| --- | --- | --- |
| androidoscopy | master `f4461a81` | server/src/control.rs and dashboard.rs: auth/HTTP/WebSocket routes and MIME-only asset responses; no explicit cache/security-header policy. |
| lellostore | master `54615a10` | backend/src/main.rs and api/{file_response,static_files}.rs: headers describe media type, lengths, disposition and ranges; no explicit cache/security-header policy. File/range handling remains outside 04b. |
| observo | master `ef3bff39` | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Routing, forms/HTML/redirects, auth middleware, peer-aware serving, plugin request/body proxying and HTTP tests use shared APIs; Axum remains internal to simple-server. |
| paranza | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Private routing, state/path/query/raw-body extraction, response conversion, HTTP serving and test helpers use shared APIs; Axum remains internal to simple-server. |
| peerlo | **Done (local)** | None. |
| pezzottify-downloader | master `2bf0158a` | downloader/http_server.rs and puppeteer/{proxy,correlation}.rs: media/transport headers and proxy/correlation propagation; no cache/security-header policy. CORS is separate. |
| quentin-torrentino | master `e6064149` | crates/server/src/api/{routes,middleware}.rs: auth, metrics and static-service routing, no explicit response cache/security-header policy. Outbound provider request headers are unrelated. |

### crumbles

Migration **`387126219ed8c3da78423466e609aa6de5919dd7`**, based on master `f74f6a8b`.
The installed browser security policy uses shared insert-if-absent for CSP, nosniff, frame/referrer and permissions headers, retaining configured HSTS replacement. Session-cookie, auth/setup and runner-history no-store helpers use shared replacement. CSP generation, trusted-proxy decisions, cookie append semantics, route placement and endpoint-specific response tuples stay local.

Baseline and migrated suites: **589 passed, 0 failed, 2 ignored**.
Existing resource-specific CSP, configured HSTS, browser auth/CSRF, repeated cookies, setup rejection and cache-header assertions run in the complete main binary suite. Existing ignored frontend build assets were copied unchanged for compile-time embedding.

Strict all-target Clippy passes. Changed-file formatting and
whitespace checks pass. Other workspace packages, fresh frontend/Android builds, standalone native-dispatch qualification, live providers and containers were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. The original checkout
was clean; pre-existing worktrees and branches were left alone.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### fausto

Migration **`e59b3241a608e6e34b03ec64daaebe0b0514f517`**, based on master `7b494b7e`.
The production plugin-file success response uses shared replacement for its existing public, max-age=3600 cache policy. MIME selection, file/path validation, missing-file errors and the response body remain local; request correlation and rate-limit Retry-After are outside this scope.

Baseline and migrated suites: **202 passed, 0 failed, 1 ignored**.
A new production-router regression passes before/after replacement for successful GET, HEAD, body/MIME/cache headers and missing-file responses without a cache policy. The server package suite includes that regression. Its initial test-client HEAD API mismatch was corrected before obtaining the passing baseline.

All-target Clippy completes with capped warnings; existing lint debt remains. Strict warning-free lint is not claimed. Changed-file formatting and
whitespace checks pass. Other workspace packages, optional plugin feature combinations, frontend/Android builds, live federation/plugins and Docker were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. The original checkout
was clean; pre-existing worktrees and branches were left alone.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### favzetto

Migration **`47c8fe67d532ea8823c0167fe548a48b02d2a329`**, based on master `22b6a2d2`.
The embedded frontend and catalog person-picture responses use shared replacement for their three existing cache policies: immutable assets, no-cache frontend fallback, and public picture max-age. Asset selection, auth, MIME, response bodies and application policy remain local.

Baseline and migrated suites: **224 passed, 2 failed, 0 ignored**.
Both runs pass 131 unit tests and 93 API tests, with the same two failures: catalog_research_runtime_bridge_approves_runtime_draft and catalog_research_runtime_bridge_rejects_runtime_draft. Both reject transitions from completed terminal flows with the same messages/statuses. Cargo stops at that integration target; later targets are not claimed. Existing ignored web/dist assets were copied unchanged for embedding.

All-target Clippy completes with capped warnings; existing lint debt remains. Strict warning-free lint is not claimed. Changed-file formatting and
whitespace checks pass. The unrelated catalog-flow failures were not fixed. Later integration targets, fresh frontend/Android builds, containers and live providers were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. The original checkout
was clean; pre-existing worktrees and branches were left alone.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### lello-auth

Migration **`b1827fddee81bd1d1ba6e2ccca088f439d1b17ec`**, based on master `758db1b9`.
The Axum adapter's existing token/authenticator/admin/hosted-UI no-store and Pragma helpers use shared replacement, along with authenticator Referrer-Policy and embedded-asset cache/nosniff headers. Header values, cookies, authentication, error formats and call placement remain local; unrelated endpoint-specific response tuples are unchanged.

Baseline and migrated suites: **358 passed, 0 failed, 0 ignored**.
Both affected HTTP/server packages run, including token/authenticator/admin/hosted-UI cache and no-store assertions and the existing body-limit rejection regression. The core-only package suite was not separately run.

All-target Clippy completes with capped warnings; existing lint debt remains. Strict warning-free lint is not claimed. Changed-file formatting and
whitespace checks pass. Browser builds, external identity providers, release/containers and full deployment qualification were not rerun. Three existing untracked research documents are preserved.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. 3 unrelated working files
were preserved byte-for-byte, with staged/unstaged status unchanged.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### meteonesto

Migration **`6d0eb4589776aeaaddae25871a7780b8fcb3993d`**, based on master `68509418`.
Weather API forecast-cache helpers and integrity-checked map assets use shared cache-header replacement; the gateway retains route/status-selected cache policy and no-store problem responses through shared replacement. ETags, map validation, upstream forwarding/deadlines and eligibility remain local. Map manifest/error tuples remain local response construction. The pipeline has no matching cache/security-header policy to migrate.

Baseline and migrated suites: **101 passed, 0 failed, 1 ignored**.
Both weather-api and weather-gateway package suites run before/after, including cache/ETag, map asset, proxy/error and auth checks. The three component README source pins are synchronized; only API/gateway enable response-headers.

Strict all-target Clippy passes. Changed-file formatting and
whitespace checks pass. The pipeline suite, renderers, Android, live weather/identity providers and containers were not rerun. One existing gateway test remains ignored.

The proxy cache-value selection was extracted into a local helper to retain
strict function-length lint; the changed gateway suite and lint passed again.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. The original checkout
was clean; pre-existing worktrees and branches were left alone.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### pezzottflix

Migration **`8b07b337415120cad79278e860a11fed49fdacf8`**, based on master `ca8a370e`.
All seven installed security-header operations use shared replacement, preserving CSP/permissions values and production-only HSTS. Existing headers for unrelated fields, cookies, response bodies and route/layer placement are unchanged. Media/image/subtitle response constructors remain application-owned.

Baseline and migrated suites: **537 passed, 0 failed, 3 ignored**.
A new middleware regression passes before/after for repeated Set-Cookie, explicit cache headers, body/status preservation and differing development/production HSTS override behavior. The server package suite includes existing security/auth/media checks.

All-target Clippy completes with capped warnings; existing lint debt remains. Strict warning-free lint is not claimed. Changed-file formatting and
whitespace checks pass. The separate real-process lifecycle script, frontend/Android, live providers and release/containers were not rerun.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. The original checkout
was clean; pre-existing worktrees and branches were left alone.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### sct

Migration **`18d086ea64ab5f308acd37fb3319e8827da4b1d0`**, based on master `d3961d4c`.
The storage-backed API response policy and ApiError use shared replacement for no-store, nosniff and no-referrer. Correlation, retry guidance, framework-error normalization, auth, status and body construction remain local. No-storage fallback and metrics response tuples remain local response construction; no new global policy is installed.

Baseline and migrated suites: **6 passed, 0 failed, 15 ignored**.
A new real-HTTP regression exercises the production policy without storage: GET/HEAD, existing cache/referrer override, cookies, body/status, generated correlation and normalized early errors. It passes before and after migration alongside the server suite.

Strict all-target Clippy passes. Changed-file formatting and
whitespace checks pass. Fifteen existing PostgreSQL/S3 qualification tests remain ignored. Database-backed production HTTP, browser and destructive container qualifications were not rerun.

Concurrent archive commits `592adcd` and `abae1ed` were replayed as `2342023` and
`f4de667`; range-diff verifies both patches are unchanged. Recovery branch
`recovery/step04b-before-rebase-abae1edb` is retained. The combined final tree at
**`f4de6679702955b9bc32535ff64e57846d1e50af`** passes six server tests
(fifteen ignored) and strict all-target Clippy. New archive features were not
separately qualified by this migration. Temporary migration branch/worktree removed;
pre-existing worktrees, stash and subsequent user archive edits remain untouched.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

### simple-ai

Migration **`b5fa60aee57cd4e6b9c3a34e3c760d60a8885f60`**, based on master `f5115239`.
Five production streaming branches use shared replacement for their existing no-cache headers: backend chat, Responses and speech, plus runner chat and speech. Stream construction, reservation/cancellation/accounting, content-type, keep-alive and placement stay local; no body polling/buffering is introduced.

Baseline and migrated suites: **399 passed, 0 failed, 1 ignored**.
Backend and inference-runner package suites run before/after. The existing ignored scripts/configs/rtx.toml test fixture was copied unchanged into the worktree; it is not committed. Shared module contracts separately verify lazy data/trailer/error frames.

All-target Clippy completes with capped warnings; existing lint debt remains. Strict warning-free lint is not claimed. Changed-file formatting and
whitespace checks pass. Real GPU/model inference, external providers, Android/browser and container/release workflows were not rerun. Existing semantic-evaluation working files are preserved.

Original master rebased onto the migration; ancestry and identical tested tree
verified, temporary migration worktree/branch removed. 20 unrelated working files
were preserved byte-for-byte, with staged/unstaged status unchanged.
See the service's `docs/step-04b-response-headers.md` for commands and scope.

04b rollout totals: **10 Done, seven N/A, 0 Pending**.
All applicable migrations are committed and integrated into their development
branches; temporary migration worktrees/branches are removed. Source pins and
both trackers are updated. 04c remains planned. No pushes or deployments.

## Step 04c: CORS configuration

Implemented and rolled out locally on 2026-09-21: **8 Done, 9 N/A, 0 Pending**. Three gpt-5.6-sol agents at low reasoning effort
prepared the rollout in disjoint repositories. The coordinator owns the shared
trackers and executed final commands where child approval transport stalled.
The optional [04c contract](step-04c-cors.md) exposes only owned public policy,
layer/service/future types plus HTTP/Tower primitives. Default configuration grants
no cross-origin permissions. It works with default features disabled, without Axum.

Reviewed source: `3aa933295860a9ed08b51ae882c8297f17e7eac2` on simple-server `main`.
The full baseline and final feature matrix, strict Clippy and formatting pass.
A rustdoc link failure was fixed and strict documentation checks rerun successfully.
Five new CORS contract tests cover 300 comparisons against the prior middleware,
restrictive defaults, invalid wildcards/credentials, setter replacement, response
identity, readiness and service errors. Production dependency-tree inspection
confirms standalone CORS has no Axum dependency.

| Consumer | Production adoption | Baseline → final verification | Integrated commit |
| --- | --- | --- | --- |
| Crumbles | Main HTTP router uses configured exact origins/credentials, six methods, existing authorization/content-type/accept/CSRF/correlation headers and exposed correlation ID. Empty origins grant no cross-origin access. Placement preserves security headers on preflights, with correlation IDs on ordinary responses only. Integration daemon has no CORS policy. | 589 → **590 passed**, two existing ignores, complete main binary suite. Five focused CORS tests passed against the old middleware before replacement; final suite includes allowed/denied/missing origins, credentials on/off, unauthorized responses, preflights and browser security state. Strict all-target Clippy and changed-file formatting pass. | `433d7094247c28ec7022f5751949c747e5aadc7c` on `master` |
| Simple AI | Both backend and inference-runner use shared CORS policies. Both retain wildcard origins/methods/request headers without credentials. Only the runner exposes wildcard response headers. Route/layer placement and inference streams are unchanged. | 399 → **401 passed**, one existing ignore, across backend/runner suites. Both new production-policy tests passed against the old implementation before replacement. All-target Clippy completes with existing capped warnings (backend 12, common 3, runner 5); changed-file formatting passes. | `f73daa6a30c037151b2b6f167dc248e309fb0763` on `master` |

Both consumers remove direct tower-http dependencies and pin the reviewed source
in `simple-server.rev`. Their resolved tower-http versions remain 0.6.11 and
0.6.8 respectively; no dependency version upgrades were needed. Detailed commands
and scope are in each consumer's `docs/step-04c-cors.md`.

Each migration used a dedicated sibling worktree and branch from the inspected
active `master`: Crumbles started at `3871262`, Simple AI at `b5fa60a`. Both master
branches were rebased onto their migration branches, ancestry verified, and final
trees matched the tested trees without concurrent commits to replay. Temporary
migration branches/worktrees were removed. Simple AI's 20 unrelated working files
were preserved byte-for-byte with unchanged Git status. Shared-library work was
also committed in an isolated worktree, integrated into `main` and cleaned up.

Limits: Crumbles reused ignored frontend assets; other workspace packages, fresh
frontend/Android builds, browser E2E, standalone native-dispatch qualification,
providers and containers were not rerun. Simple AI reused its ignored runner TOML
fixture; real GPU/model inference, external providers, browser/Android E2E and
containers were not rerun. This is local migration qualification, not a production
deployment. Nothing was pushed or deployed.

### 04c rollout applicability assessment

The following nine products have no Rust CORS middleware to extract. This does
not mean CORS is absent from their deployment: Lello Auth manages it at Caddy.
These are source assessments, not claims that unrelated test suites were rerun.
No feature enablement, source pin update, worktree or consumer commit is needed
for a capability that is not used. Existing dependency features alone do not
establish adoption. Their application-owned browser/CSRF boundaries remain intact.

| Product | Inspected development branch / HEAD | Applicability evidence |
| --- | --- | --- |
| pezzottify | `dev` / `a25b0c3a` | `pezzottify-server/src/server/route_builder.rs` installs authentication, CSRF, rate limits, tracing and cache policy, but no CORS layer or allow-origin response policy. |
| favzetto | **Done (local)** | None. |
| androidoscopy | `master` / `f4461a81` | `server/src/main.rs` HTTP/WS router setup and legacy server have no CORS response policy. |
| lello-auth | `master` / `b1827fdd` | CORS managed by Caddy; not migrated into Rust. `homelab/caddy/Caddyfile` permits selected application origins with credentials and OPTIONS 204. Server, integration crate and examples install no Rust CORS layer. Moving ownership needs coordinated proxy/application changes; live deployment not probed. |
| meteonesto | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. API, gateway and pipeline control-plane routes, proxy/byte/static responses, client identity, auth/correlation middleware, serving and mock upstreams use shared APIs. |
| paranza | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Private routing, state/path/query/raw-body extraction, response conversion, HTTP serving and test helpers use shared APIs; Axum remains internal to simple-server. |
| quentin-torrentino | `master` / `e6064149` | `crates/server` router has no CORS layer despite an enabled tower-http feature. |
| sct | `master` / `28604f17` | Server browser-origin and CSRF checks are application security boundaries, not a CORS response policy. Existing dirty work remains untouched. |
| simple-agents | `main` / `11674342` | `crates/simple-agents-service/src/ui.rs::browser_boundary` and the same-origin development proxy are outside the shared CORS scope; no installed CORS middleware. |

### 04c rollout verification

The rollout uses reviewed source `3aa933295860a9ed08b51ae882c8297f17e7eac2`.
Three `gpt-5.6-sol` workers at low reasoning effort owned disjoint repositories;
the coordinator reviewed changes and owned the two central trackers. Child edit
and escalation calls stalled, so the coordinator executed reviewed commands in
the same dedicated worktrees. One worker subsequently hit model capacity; the
coordinator completed its documentation and integration. No replacement model
was used for delegated work.

| Consumer | Preserved production policy | Verification and scope | Migration commit on `master` |
| --- | --- | --- | --- |
| lellostore | Outermost backend CORS: any origin/request header; exactly GET/POST/PUT/DELETE; no credentials, exposed headers or max-age. | Baseline 135 tests; strengthened production-router assertion passed on both legacy and shared policy, including a fail-closed 503 and preflight short-circuit. Final all-feature suite **136 passed, two ignored doctests**; strict all-target/all-feature Clippy passes. Existing ignored frontend assets were copied after the initial all-feature build reported their absence. | `7abeb7a2597acca4f8e2d140aea460ca0e4da394` |
| fausto | Configured comma-separated origins with existing invalid-entry filtering; six methods and four request headers; no credentials/exposed headers/max-age; trace/correlation placement unchanged. | Existing baseline CORS audit passed. Strengthened real-router tests cover configured/denied origins and exact methods/headers. Final server-package suite **203 passed, one ignored doctest**. All-target Clippy completes with warnings capped; existing lint debt remains. | `cb8f077886849865452256938b6d243fb6e7e74c` |
| observo | Protected-route tree only: wildcard origins/methods/request/exposed headers, no credentials; public routes stay outside CORS, auth/body-limit stay inside. | Baseline route tests: 47 passed. Final server suite **92 passed**; new policy test distinguishes preflight from ordinary exposed headers. All-target Clippy completes with warnings capped. | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Routing, forms/HTML/redirects, auth middleware, peer-aware serving, plugin request/body proxying and HTTP tests use shared APIs; Axum remains internal to simple-server. |
| peerlo | Conditional production server plus both CORS router helpers: wildcard origins/methods/request headers, no exposed headers/credentials; original auth/rate-limit order retained. | **Done (local)** | None. |
| pezzottflix | Wildcard origins/methods/request/exposed headers; no credentials/max-age; compression/security-header placement retained. | Existing production-router CORS test passed before migration and was strengthened for OPTIONS status and ordinary exposed headers. Final server suite **537 passed, three ignored**; capped all-target Clippy passes. Shared tower-http 0.6.11 CORS is added alongside retained 0.5 dependencies for other middleware. | `1e1a94a80b3843b3d54a3cdcaec805de924cea45` |
| pezzottify-downloader | Puppeteer exact configured/empty origin list, GET/POST/OPTIONS and four request headers; downloader any origin/header but GET/OPTIONS only. Neither exposes headers or enables credentials/max-age. Existing trace/correlation placement retained. | Baseline parser tests passed. Three differential tests compare old tower-http 0.5 CORS with shared 0.6 behavior, plus tests of both production policies. Full package suite **166 passed, one ignored** and capped all-target Clippy pass, including a full rerun after worktree recovery. Existing formatting debt is retained and diff checks pass. | `22103243f75c0f6c8ad75f732c6dbbbd5c1ab122` |

Each consumer's `docs/step-04c-cors.md` records policy and verification
limitations. Source pins and active CI/build instructions were updated where
applicable; historical migration references retain their original revisions.
CORS feature enablement is backed by production API calls, not dependency flags
alone. The migrations introduce no global policy defaults and preserve middleware
placement. Broader frontend, Android, browser/container E2E and external-provider
workflows were not repeated unless explicitly listed above.

### 04c integration and cleanup

All six applicable rollout repositories started from the inspected clean active
`master` branches and used dedicated sibling worktrees. Baseline tips were
LelloStore `fed9e9b`, Fausto `e59b324`, Observo `ef3bff3`, Peerlo `907c9ad`,
Pezzottflix `8b07b33` and downloader `2bf0158`. Their master branches were rebased
**onto** the migration branches; ancestry, matching tested trees and clean
original status were verified. All temporary migration branches/worktrees were
removed. The two earlier canaries remain integrated as recorded above.

The downloader's first uncommitted worktree and branch disappeared before
integration for an undetermined reason; neither assigned worker nor coordinator
removed it. Its original master was unchanged. Saved scripts reconstructed the
change in a fresh isolated worktree, and the complete test suite and lint were
rerun successfully before commit/rebase/cleanup. A separate source backup was
also retained during recovery. Final adoption is verified against the recovered,
committed tree, not merely the earlier test logs.

N/A assessments did not change production code, pins or policies. Quentin,
SCT and Simple Agents additionally passed targeted package compilation in clean
assessment worktrees. Quentin and Simple Agents assessment worktrees/branches
were removed without base-branch integration; SCT's assessment worktree also
ceased to exist during concurrent work. Active SCT and Simple Agents development
advanced independently and was not rebased or overwritten. SCT's later untracked
`docs/step-04c-cors.md` and other unrelated working files were left untouched.
Lello-auth's existing untracked research files were verified unchanged by hash.

Both central trackers agree on **8 Done, 9 N/A, 0 Pending**, with eight of fourteen
shared modules implemented. The HTML remains a single scrolling surface. This
records local implementation and verification only: no pushes or deployments.

## Step 05: health and readiness

The [contract and source inventory](step-05-health.md) define the optional `health`
module, implemented in `44a9fa24c122f3ac93d930813b320abd099cbb79` on `main`.
Shared checks run in order, stop at the first original application error, and
retain application-owned response rendering. No implicit deadlines or lifecycle
policy. **Canary milestone: one Done (Favzetto).** The subsequent rollout is complete:
**14 Done, 3 N/A, zero Pending**; see the per-service record below.

Shared verification: baseline 63 tests/doctests passed; final 68 passed, strict
all-target/all-feature Clippy and formatting passed. Four health tests also pass
without default features. Minimal normal dependencies are HTTP and Tower only;
no-feature compilation passes. Documentation links and HTML script syntax checked.

### Favzetto canary

- Applicability: real production `/health` liveness and `/ready` readiness routes.
  Both now mount shared probe endpoints; database, storage and PDF checks remain
  ordered and stop at the first AppError. JSON/version, 500 error behavior, public
  access, GET/HEAD/405, and rate limits are preserved. No new route or timeout.
- Base: clean `master` at `47c8fe67d532ea8823c0167fe548a48b02d2a329`.
  Canary commit: `14d5b0d690aed861b00c2fde943254a7a242ef7b`.
  Reviewed shared revision: `44a9fa24c122f3ac93d930813b320abd099cbb79`.
  The sibling path dependency is retained; Cargo.lock does not pin that source.
- Baseline: 131 unit and 93 API tests passed, with the two previously recorded
  catalog bridge transition failures. Four new health regressions pass against
  old production handlers before migration; four process tests pass separately.
- Final: 131 unit, 97 API, and four lifecycle/logging process tests passed
  (**232 passed total**); the same two catalog bridge failures remain. The new
  tests cover response/method contracts, storage failure/recovery, liveness during
  failure, database-before-storage error precedence, and missing OCR languages.
  Existing rate-limit tests pass. Four health regressions pass again after
  formatting with locked Cargo. All-target Clippy completes with capped warnings.
- Existing debt: strict Clippy fails on the untouched baseline (53 library / 55
  library-test findings). Full formatting output is identical to baseline, with
  issues only in three unchanged files. Changed files and diff whitespace pass.
- Integration: `master` rebased onto the canary; ancestry and identical tested
  tree verified; temporary worktree and branch removed. Original checkout clean.
  Library `main` likewise rebased onto its implementation branch. Final tracker
  integration/cleanup is recorded in the tracking commit.
- Limits: no Docker rebuild, browser suite or live deployment probe. Tests used
  isolated temporary storage/databases and a separate build target; existing
  ignored frontend assets were copied for embedding. Full commands and evidence
  are in Favzetto's `docs/step-05-health.md`. No push or deployment.


## Step 05 rollout — complete locally

**14 adopted, 3 N/A, zero Pending.** Three `gpt-5.6-sol` agents at low reasoning
effort assessed disjoint repositories and prepared migrations; the coordinator
reviewed patches, ran blocked child mutations/checks, integrated branches, and
owned both trackers. Child approval stalls required parent execution; they were
not source failures. No pushes or deployments.

The final shared revision is `ed245d2d46e9d29aeee7be5202f3a8b8113c9caf`.
It adds backward-compatible `Check<E, T = ()>::run()` so detailed aggregate
reports survive healthy and unhealthy outcomes without duplicate polling or
mutable side channels. Unit-valued Probe checks remain unchanged. Verified
**69 all-feature tests/doctests**, **five minimal-feature health tests**, strict
all-target/all-feature Clippy, and formatting. Favzetto's earlier canary record
above retains its original reviewed revision and evidence.

### Consumer commits and verification

All test counts below describe the stated scope, not an unqualified full product
suite. Most runs used offline Cargo, two jobs, debug information disabled, and
isolated temporary build targets; socket tests required local network permission.
Consumer Clippy and full workspace/browser/container/deployment suites were not
generally rerun. Existing formatting/lint debt was preserved and is not presented
as passing. Per-service `docs/step-05-health.md` records implementation details;
Favzetto's canary evidence remains above.

| Service | Development base at start | Integrated migration commit | Verified scope |
| --- | --- | --- | --- |
| crumbles | `master` / `433d709` | `c35fa3f297ba0ca2789d7557234b4aa7b374ed62` | Baseline 533 server tests passed, 2 ignored; final 534 passed, 2 ignored. Added HEAD suppression and POST 405/Allow; existing correlation, headers, metrics and CORS covered. Formatting/diff clean. |
| lello-auth | `master` / `b1827fd` | `5215f6d6e55d6388072255c27f7d4f161a201a6f` | Baseline two focused tests; final 22 server tests (13 unit and 3 each configuration, lifecycle process, logging). Liveness and composite database/signing readiness retain independent result fields. Changed-file formatting passed. |
| lellostore | `master` / `7abeb7a` | `c3a3f7f2cdc811232cfbfa8c022bd54bd32761cf` | Baseline health 1; final health 1, authentication 10, HTTP tracing 1. Failed OIDC still leaves liveness public. Formatting/diff passed. |
| fausto | `master` / `cb8f077` | `4ccfb9cee540992aacb46511acbb0f56c2f758e7` | Baseline production health 1; final health 1 plus HTTP tracing 1. Version JSON, OpenAPI, auth exemption and middleware retained. Formatting/diff passed. |
| meteonesto | `master` / `6d0eb45` | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. API, gateway and pipeline control-plane routes, proxy/byte/static responses, client identity, auth/correlation middleware, serving and mock upstreams use shared APIs. |
| sct | `master` / `b7d8230` | `b34840dbf7dbf1939426565acedd068916bf20b4` | Baseline HTTP test compiled but sandbox denied bind; permitted final HTTP test 1/1 and server check passed. Database timeout, writer readiness, 204/503 and text liveness preserved. Changed-file formatting passed. |
| simple-agents | `main` / `0cf88e2` | `3a106d9efac3852146ff10d74bda924830a7281c` | Focused real-router database readiness/liveness regression 1/1 before and after; formatting passed. Replayed concurrent UI commit; tested Cargo/crates tree unchanged by replay. |
| simple-ai | `master` / `30ed3ea` | `0523c03277930f2e8bc64ff715779d16e600541c` | Baseline runner health 2; final backend 1 and runner 4. Fake engines verify complete mixed/unhealthy reports, every-engine polling, empty/OCR behavior; GET/HEAD/POST verified. Existing ignored rtx.toml copied unchanged after missing-fixture compile failure; final rerun passed. |
| peerlo | `master` / `067c2a7` | **Done (local)** | None. |
| paranza | `master` / `2bd528f` | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Private routing, state/path/query/raw-body extraction, response conversion, HTTP serving and test helpers use shared APIs; Axum remains internal to simple-server. |
| pezzottflix | `master` / `1e1a94a` | `f8159f0e19436d5cfba6a785d255a3f0b74bc3ab` | Health-filter baseline/final each 14 passed. Final full API health suite 12 passed (overlaps filter), with added storage failure, database failure vs independent liveness, HEAD/POST tests. Both aggregate dependency results retained. Formatting/diff passed. |
| pezzottify-downloader | `master` / `2210324` | `620a0af18c5918bb9383f3b57469e1c397ddcc4b` | Baseline health 6; final all-target suite 159 passed, including Unix/public probes, proxy/CORS and real process lifecycle. Both production endpoints migrated; Spotify connection policy unchanged. Existing broad formatting debt preserved. |
| quentin-torrentino | `master` / `e606414` | `a8a921f3e8014e7996095ad5ea26bcbfc8625633` | Two health tests passed in each baseline/final phase: in-process API and real-server startup. Existing API auth and metrics placement retained. Changed-file formatting/diff passed. |

### Scope and applicability

- **Favzetto:** both production probes, as verified in the earlier canary.
- **Crumbles, LelloStore, Fausto, Quentin:** existing static production liveness;
  no dependency readiness or new routes were invented. Crumbles integration
  daemon has no additional HTTP health route to migrate.
- **Lello Auth:** production server `/health`, `/health/live`, `/health/ready`.
  The independently runnable webhook-handler demonstration is not shipped in
  that server binary and retains its own sample endpoint; no sample adoption
  is claimed. Both database and signing checks still run on a failed request.
- **Meteonesto:** weather API, gateway and pipeline control API all migrated.
- **SCT:** liveness, database and writer readiness in the server; clients only
  consume probes. Existing archive work is unrelated and was preserved.
- **Simple Agents:** service `/healthz` and `/readyz`; runner/client/delivery
  binaries do not serve additional health routes.
- **Simple AI:** backend liveness and runner aggregate engine health; every
  engine is still polled and original empty/OCR/any-healthy semantics remain.
- **Peerlo:** dynamic DHT configuration/uptime snapshot; degraded remains 200.
- **Paranza:** detailed snapshot including node/runner/activation/freshness data
  and original store/mutex error handling. No second snapshot or new probe.
- **Pezzottflix:** aggregate database/storage, independent liveness, database
  readiness. Aggregate failures still include both results. An unrelated generic
  monitoring abstraction not installed at these production routes stays local.
- **Downloader:** public Puppeteer TCP and child downloader Unix HTTP probes.

The following three products have no served production health/readiness contract;
adding a feature or new endpoint solely to fill the table would not be adoption.

| N/A service | Audited branch/revision | Evidence and documentation |
| --- | --- | --- |
| Pezzottify | `dev` / `a25b0c3aa65c543f6ddc220f4f7105aafdfb365d` | Route builder/bootstrap serve application routes and metrics, no dedicated probe. Docker startup polling of `/` is not a probe contract; outbound downloader health calls are client operations. No migration branch or code change; existing paravoid worktree untouched. |
| Androidoscopy | `master` / `f4461a8155a4461005b72ed6fd752e137774145c` | Legacy dashboard/app WebSocket and v2 controller expose no health route. Source-only assessment committed as `a6648cd8d5ad873ac91dc367639aac48048c78ee`, rebased/integrated, assessment worktree/branch removed. |
| Observo | `master` / `86d28f401fc4fa5236b30dc26c310e8ce3262684` | Server, content extractor and link scorer expose no probe. Source-only assessment committed as `a2dc0c21de26253862b55997b23b140c9fbbd43c`, rebased/integrated, assessment worktree/branch removed. |

### Integration and remaining limits

Every applicable development branch was rebased **onto** its migration branch.
Ancestry and the tested tree were verified before deleting each temporary
worktree and migration branch; documentation-only assessments followed the same
workflow. Final audit confirms no Step 05 consumer migration worktrees/branches
remain. Pre-existing unrelated worktrees were not removed.

Simple Agents advanced from `0cf88e2` to UI commit `91b63b0` during migration.
Its migration is `3a106d9`; replay produced `977c7ce`, with identical tested
Cargo/crates contents. A recovery ref retains the pre-rebase tip. Later user
commits may advance the branch further; adoption is verified by ancestry.
Uncommitted Simple Agents UI changes, SCT archive work, Lello Auth research,
and Simple AI semantic-evaluation files were preserved. Dirty path lists matched
before/after integration; no unrelated work was committed by this rollout.

This record covers local implementation and the checks explicitly listed, not
live deployments, all language clients, every workspace test, or a promise that
all existing lint/formatting debt is resolved. No extra runtime routes, readiness
conditions, authentication policies, deadlines or deployment changes were added.
The coordinator's final tracker commit is integrated into `simple-server/main`
using the same worktree/rebase/cleanup workflow.

## Step 06: background tasks

06a/06b/06c are implemented. Pezzottify has completed the local canary through
the shared ownership, scheduling/capacity, and policy primitives. All remaining
services have now been assessed and supported scoped migrations integrated locally.
The 2026-09-23 completion pass resolves all remaining 06b/06c adoption gaps with
composable primitives and drivers: 06b is 13 Done / 4 N/A; 06c is 8 Done / 9 N/A.
Durable database authority and product-specific workflows remain application-owned.
Earlier checkpoint records below are historical; the final completion record
supersedes their Pending/Partial assessments.

06a verification: unchanged baseline 69 tests/doctests; 10 ownership contract
tests covering callback reservations, admission races, retained errors/panics,
bounded outcomes, cancelled drains, blocking work and explicit abortion.
Final 06a checks: 79 tests/doctests, strict all-feature/all-target Clippy,
standalone example, HTML script syntax and task-only dependency audit passed.
Local HTTP tests required permission to bind sockets outside the sandbox.
See [the contract](step-06-background-tasks.md). Implementation follows isolated
worktree, commit, development-branch rebase, verification and cleanup workflow.

06a integrated on main at `44fe8d1`; tested tree matched and temporary worktree
and branch were removed. 06b adds seven contract tests for UTC boundaries,
interval/jitter calculations, registration validation, pool isolation, bounded
queues, typed event fanout, failure observation and retained ownership after an
outer deadline. At that implementation checkpoint, no consumers had migrated.

06b final checks: 86 tests/doctests, strict all-feature/all-target Clippy and
the runnable scheduler/lifecycle example passed. HTML script syntax and a
no-default-features scheduling build were checked.

06b integrated on main at `44f7688`; tested tree matched and its temporary
worktree and branch were removed. 06c adds storage-independent execution budgets,
classified retries, generation-fenced circuit breakers, pause scopes and control
snapshots, with scheduler integration and actual execution timestamps.

06c verification includes six pure policy tests, twelve combined policy/scheduler
tests and two additional scheduler clock/ingress tests, plus an observer-shutdown
regression test. These exercise retained
blocking capacity, source errors after timeout, actual versus observed runtime,
retry reservations, pause cancellation, snapshot restoration without job replay,
atomic snapshot validation, UTC/monotonic separation and bounded command ingress.
Consumer adoption remains Pending for all three stages; no service code changed.

Final 06c verification: **107 tests/doctests pass** across all features. Minimal
feature suites also pass: tasks 22, scheduling 32, policies 28, scheduling plus
policies 50; the empty-feature build passes. All four normal dependency graphs
exclude Axum and Tower HTTP. Strict all-target/all-feature Clippy, formatting,
three runnable examples, rustdoc warnings-as-errors and HTML matrix validation
pass. The existing health rustdoc link was qualified to fix its resolution error.
Socket and child-signal tests were verified outside the sandbox; the sandboxed
signal-child attempt timed out. No production/deployment checks or consumer
adoption are claimed.

Reviewed Step 06 library revision: `2ee010ecc59ca2443cb09bbf5eb828dbb68a082b`.
Stage commits: 06a `44fe8d1`, 06b `44f7688`, 06c `2ee010e`.
All three used dedicated worktree branches from the established `main` branch.
The verification-record commit changes only the two central trackers.


### Pezzottify canary preparation: shared execution capacity

The canary requires capacity limits independently of the shared scheduler so it
can retain its database-owned history, manual-run schedule resets, and existing
pause cancellation policy. `ExecutionCapacity` now exposes nonzero global and
named-pool limits with cancellation-safe class-before-global acquisition and
owned execution permits. Six new contract tests cover class isolation, global
limits across clones, cancelled partial acquisition, retained executing permits,
unknown pools, and invalid configuration. All **113** library tests/doctests and
strict all-feature/all-target Clippy pass; the six capacity tests also pass with
only `task-scheduling` enabled and default HTTP features disabled. The addition
is committed at `4a6353f55b23dff173ec1968915c6e10312d5795`; consumer adoption
is recorded separately below.


### Step 06 Pezzottify canary — complete locally

Verified 2026-09-22. Active development branch `dev`, starting at `700d0394`,
is integrated at **`5ea9ed6bb8206943b86d883738ae78fae39046ee`**. Reviewed shared
source **`4a6353f55b23dff173ec1968915c6e10312d5795`** is recorded in the
consumer's `simple-server.rev`, which its CI checkout script reads.

Applicability is established by production call sites, not Cargo features:

- 06a: `server/lifecycle.rs` uses `WorkTracker` for upgrade reservations and
  tracked request/maintenance work; WebSocket and MCP reject late admission.
  `background_jobs/scheduler.rs` owns each execution with a bounded `TaskSet`.
  Blocking completion and history finalization remain inside the owned scope.
- 06b: persisted interval/jitter recurrence uses `Schedule::FixedDelay`, and
  global/resource-class limits use `ExecutionCapacity`. Class-before-global
  acquisition, defaults, per-job overlap exclusion, first-run policies, hooks,
  manual-run schedule resets and durable dates remain application-owned.
- 06c: production queue/runtime deadlines use `ExecutionBudget`, circuit
  transitions use `CircuitBreaker`/snapshots, and pause admission uses
  `PauseState`. Existing JSON/history/HTTP contracts and persistence failure
  behavior remain intact, including loading state after a threshold change.

The application deliberately retains its scheduler orchestration, SQLite
persistence/recovery, metrics/audit, and domain cancellation tokens. Ownership
adoption covers the existing lifecycle-tracked application work and registered
background jobs; the existing Step 02 subsystem boundaries, including the
independently owned OS-thread index builder, remain unchanged. It does not
adopt the shared runnable scheduler wholesale: its manual-run resets, accepted
queued-work pause behavior, cancellation-support checks, and runtime budget
starting before Tokio blocking dispatch differ from that scheduler's defaults.
No new automatic retries or execution of previously stubbed cron variants are
introduced. Runtime expiry still waits for blocking execution, preserves its
capacity, and records the established timeout history. Application-specific
retry/claim semantics remain application-owned.

Completion now wakes the loop promptly. Finished owners are consumed before
replacement; shutdown has admission priority. Interrupted draining retains live
owners, and resuming it does not rerun stale-job recovery or startup hooks.

Verification:

- Baseline: 127 background-job tests passed, two existing ignores; 16 admin-job
  E2E and four production-process lifecycle cases passed before changes.
- Full consumer suite: **1,429 passed, 36 existing ignores**, including the new
  E2E coverage. After the final drain-resume guard and import cleanup, **128
  background-job tests** (two existing ignores) and **44 focused HTTP/process
  E2E cases** passed again. The restart case was then made deterministic by
  selecting local `device_pruning` and passed independently again.
- **23 new scheduler E2E scenarios** drive real HTTP routes, application
  scheduling, blocking jobs, and SQLite. They cover payloads/history, concurrent
  deduplication, queue/runtime limits, class/global capacity, cancellation,
  errors/panics, pause scopes, shutdown ownership, circuits, hooks/recurrence,
  reopened-database restart state, overdue/stale records, actual SQL write
  failures, authorization, and changed circuit thresholds.
- **Five process lifecycle cases** run the actual binary: SIGTERM, SIGINT,
  admin reboot, both listeners, WebSocket/MCP drain, bind failure, and persisted
  pause restoration across process restart with HTTP rejection/resumption.
- Final release container build passed; **all 45 non-Android Docker API/browser
  E2E tests passed** (two Android cases deselected). The final run includes the
  drain-resume fix. Isolated project, image tags, fixtures, network and volumes
  were used; test containers, network and volumes were removed afterwards.
- Strict CI Clippy, formatting, whitespace and database-boundary checks pass.
  `cargo audit` passes under the existing policy with six allowed warnings;
  that policy was not changed. The shared extension passed **113 tests/doctests**,
  strict all-feature/all-target Clippy, and its six HTTP-free capacity tests.

The migration was committed in a dedicated worktree; `dev` was rebased onto that
branch, ancestry and tested-tree equality verified, and the temporary consumer
worktree/branch removed. The unrelated `pezzottify-paravoid` worktree is preserved.
The coordinator's tracker changes use the same worktree/rebase/cleanup workflow
on `simple-server/main`. No pushes or deployments were performed by this work.
See Pezzottify's `docs/step-06-background-tasks.md` for commands and retained
behavior. At this canary checkpoint only Pezzottify was marked complete; the
subsequent consumer rollout and current statuses are recorded below.

### Step 06 remaining-service rollout

**All applicable Step 06 migrations are complete locally (2026-09-23).**
All 17 services are assessed. Development branches were rebased onto their tested
migration branches; ancestry and tested trees were verified, and migration
worktrees/branches removed. Unrelated edits and pre-existing worktrees remain
untouched. No pushes or deployments were performed.

| Module | Done (scoped) | Partial | Pending compatibility | N/A (assessed) |
| --- | ---: | ---: | ---: | ---: |
| 06a ownership | 14 | 0 | 0 | 3 |
| 06b scheduling/capacity | 13 | 0 | 0 | 4 |
| 06c policies | 8 | 0 | 0 | 9 |

The completion pass below records eight additional scheduling migrations and the
two outstanding policy migrations. Shared selection/capacity/batching/cadence
primitives support each product's existing storage authority; they do not move SQL
claims, leases, workflow state or fencing into memory. Baseline failures and checks
not rerun remain explicit. **The following original rollout records are historical
checkpoints; their Pending/Partial statements are superseded by the completion
pass at the end of this document.**


LelloStore `master` is integrated at `0074ced571db77cd51b6b7d753338db8bdd64563`,
from `dbebdfa`, using shared source `4a6353f55b23dff173ec1968915c6e10312d5795`.
06a adopts `WorkTracker` in the production catalog WebSocket hub, preserving
pre-upgrade reservations, 503 after shutdown, close frames and Lifecycle drain.
06b is N/A: the directly awaited metrics refresh loop has no independent job
queue or scheduling controls. 06c is N/A: no background execution retry, budget,
circuit or pause policies. Startup authentication retries are not job policies.
Baseline 136 tests; final 137 passed, two existing ignored doctests. Includes ten
HTTP/authentication E2E and three process lifecycle tests. Multi-client WebSocket
shutdown/disconnect coverage was strengthened; interrupted drain retains upgrade
reservations. Strict all-target Clippy, formatting and diff checks pass. Frontend
embedding was not rebuilt. Master was rebased onto the migration branch; ancestry
and identical tested tree verified, clean temporary worktree and branch removed.
See LelloStore's `docs/step-06-background-tasks.md` for scope and commands.

Fausto `master` is integrated at `44dc1b02575156bc8370603f1d595115e40e3e9a`, from
`4ccfb9c`, using shared source `4a6353f`. 06a adopts shared work reservations for
WebSocket sessions and manual/cron jobs. At that rollout checkpoint, 06b remained Pending for live schedule
enable/disable, unsupported by the shared static-registration scheduler.
Production job registration occurs at startup. The dynamic cron extension below
now offers separate timing controls; the subsequent Fausto adoption is recorded below. This is a compatibility gap, not N/A. 06c is N/A: no execution retry,
budget, circuit or pause policies (cron registration enablement remains app-owned).
Baseline 203 tests; final 204 passed, one existing ignored doctest, including six
lifecycle integrations and four process tests. Added interrupted-stop coverage
proves accepted persisted jobs remain owned and late submissions are rejected.
Configured all-target Clippy completes with warnings; formatting/diff pass.
Optional embedded frontend/swagger builds not rerun. Master was rebased onto the
migration branch, ancestry and identical tested tree verified; temporary worktree
and branch removed. See Fausto's `docs/step-06-background-tasks.md`.

Observo `master` is integrated at `9ce6b07` from `a2dc0c2`, using shared source
`06c7531dfcd4149a0947b4b74a79e96bf245639c`. 06a adopts named webhook reservations
and permanent closed admission. 06b adopts cron occurrence primitives while
retaining the existing parser grammar, SQLite queue, catch-up/claim/recovery and
WAL behavior. 06c is N/A: no background execution retry/circuit/pause policy;
stale-claim recovery and HTTP-client timeout remain domain/client concerns.
Baseline 92 Rust tests; final 94 pass. Real-process HTTP/SQLite tests pass with
added due alias/field, disabled/invalid/future schedules, pending worker runs and
restart checks. Real TCP tests cover held webhook drain and late admission through
clones. Formatting/diff pass. All-target Clippy has a reproduced baseline failure
at `src/indexing/embeddings.rs:387` (`approx_constant`) and 36 warnings. Master was
rebased onto the migration, tested tree verified and temporary branch/worktree
removed. See Observo's `docs/step-06-background-tasks.md`.

Shared cron correction `06c7531` is integrated on main. Observo's compatibility
comparison exposed cron 0.12 skipping lower calendar fields across a future-year
jump: after February 2024, `0 15 3 1,15 * * 2026-2030` incorrectly started in March
2026 instead of January. Updated the internal dependency to cron 0.15; a focused
regression demonstrably failed before and passes after. All 114 tests/doctests,
strict all-target/all-feature Clippy, formatting and HTTP-free scheduling tests
pass. At that checkpoint the rollout worktree remained active for central documentation
updates; it was subsequently integrated and removed as recorded above.

Paranza assessed on clean `master` `1c559ee`: 06a/06b/06c N/A. In
`apps/paranza-server/src/main.rs`, `serve_runners` already owns its structured,
typed blocking TLS-session joins and interrupts actual sockets before joining
cleanup. No detached job/callback registry needs shared ownership. The directly
awaited `run_pcm_maintenance_loop` is a Lifecycle service, not an independent job
scheduler. No job retry, execution-budget, circuit or pause policy exists here;
runner activation/freshness settings are protocol/domain state. No Cargo feature,
service code, worktree or branch was added; this was a read-only applicability
assessment, not a new test run or migration claim.

Simple AI `master` is integrated at `d3103f8` from `0523c03`, shared source
`06c7531`. 06a adopts named WorkTracker reservations before runner planning,
retaining completion/response delivery and shutdown drain. Late dispatch is
rejected without consuming runner capacity. 06b remains Pending for model-aware
batch size/readiness/age and live runner saturation semantics, unsupported by
the generic scheduler. 06c is N/A for background execution policies; batching
readiness ages and outbound inference client behavior remain application-owned.
Baseline/final backend suites: 314 passed, one existing ignored doctest. Strengthened
real mock-runner coverage verifies accepted requests drain, responses arrive,
capacity returns and late dispatch makes no outbound request. All-target backend
Clippy completes with 12 warnings; changed-file formatting/diff pass. Workspace
formatting has unrelated existing differences; inference/GPU/browser suites were
not rerun. Master was rebased in the clean linked worktree, original README diff
and untracked file list verified preserved, then temporary worktree/branch removed.
See Simple AI's `docs/step-06-background-tasks.md`.

Peerlo `master` is integrated at `e3b08f6` from `e51e4b7`, shared source `06c7531`.
06c adopts RetryPolicy delay calculations for metadata cooldowns and persisted
candidate-peer retries. Reason-specific penalties, unlimited attempts, count/
exponent saturation, reset, eviction and SQLite eligibility remain application-owned.
06a is N/A: independently spawned network components already have explicit
lifecycle-owned abort/join order; typed per-cycle fanout remains structured.
06b is N/A: no separate cron/job-trigger scheduler beyond protocol operations.
Baseline affected suites: 162 passed, two existing ignored; final 164 passed,
two ignored, including two process lifecycle checks and SQLite eligibility tests.
New matrices compare counts 0..63/u32::MAX, caps, failure reasons and fixed penalties.
Affected all-target Clippy completes with warnings; changed-file formatting/diff
pass. Full DHT network/Docker/browser suites not rerun. Master rebased onto the
migration, tested tree verified, temporary worktree/branch removed. See Peerlo's
`docs/step-06-background-tasks.md`.

Androidoscopy `master` is integrated at `49bfea4` from `a6648cd`, shared source
`06c7531`. 06a adopts WorkTracker for legacy upgrades and v2 controller connection,
reader, event and action work, retaining TLS reader abort-on-drop. Guards precede
spawn/mutation; closed upgrades return 503 and rejected connections leave no
phantom devices. 06b/06c N/A: heartbeat/discovery/cleanup/reconnect and call/pairing
budgets are protocol loops and transport behavior, not an independent job scheduler
or execution-policy system. Baseline 72 Rust tests; final 74 pass. Added closed
admission/pending-reservation and real-socket shutdown tests. All six real-process
v2/legacy WebSocket/legacy TLS cases pass under SIGINT/SIGTERM with open sockets
and listener reuse. All-target Clippy completes with warnings; changed-file
formatting/diff pass. Android/device/browser suites not rerun. Master rebased,
tested tree verified, temporary worktree/branch removed. See Androidoscopy's
`docs/step-06-background-tasks.md`.

Crumbles `master` is integrated at `a74f4dd` from `c35fa3f`, shared source
`06c7531`. 06a adopts WorkTracker for realtime WebSocket reservations, with guards
before upgrade and 503 after admission closes. 06b remains Pending: production
SQLite reservations provide cross-process priority/project scheduling and paused
sessions retain profile capacity while releasing global capacity. 06c remains
Pending for durable recovery authority and signed-jitter retry semantics. The
standalone CapacityCoordinator has no production callers. Baseline 657 tests;
final 658 passed, two existing ignored. Authenticated socket drain and closed
admission tests, both binary builds, SIGINT/SIGTERM active HTTP drain and restart
checks pass. Clippy completes with one existing unused import warning. Master
rebased, tested tree verified, migration worktree/branch removed. See Crumbles'
`docs/step-06-background-tasks.md`; frontend was not rebuilt.

Simple Agents `main` is integrated at `6b520b0` from `999a877`, shared source
`06c7531`. 06a owns broker upgrades and Codex checks/login work before database
mutation, draining inside the existing Lifecycle deadline before database close.
06c uses RetryPolicy for persisted recovery delay calculation; durable fencing,
cleanup evidence, attempts and exhaustion remain application-owned. 06b remains
Pending for durable fleet reservations; the unused CapacityCoordinator is not
adoption. Baseline 134 service tests; final 136 pass, including a live enrolled
broker during SIGINT/SIGTERM/restart, rejected Codex admission without DB mutation,
and exact 999/1000ms retry eligibility. Strict service Clippy and changed-file
formatting pass. Main rebased, tested tree verified, migration worktree/branch
removed. Runner GPU/container/browser suites not rerun. See Simple Agents'
`docs/step-06-background-tasks.md`.

LelloAuth `master` is integrated at `8122dc8` from `5215f6d`, shared source
`06c7531`. 06a owns the webhook worker and deliveries and drains accepted queue
entries inside the existing Lifecycle deadline. Alert delivery remains application-
owned. 06b adopts bounded ExecutionCapacity; 06c uses RetryPolicy for production
retry configuration, retaining permissive library-input compatibility fallback.
Baseline 24 webhook tests and three process tests; final 26 webhook and all three
process tests pass. Real TCP coverage verifies interrupted drain, queued deliveries,
503 retry, late rejection and concurrency. Process tests verify drain ordering and
signals/deadline behavior. Affected all-target Clippy completes with five warnings
in unchanged code; changed-file formatting/diff pass. Full workspace/PostgreSQL/
browser suites not rerun. Master rebased, tested tree verified, temporary worktree/
branch removed; unrelated research documents preserved. See LelloAuth's
`docs/step-06-background-tasks.md`.

Pezzottify Downloader `master` is integrated at `e10c767` from `620a0af`, shared
source `06c7531`. 06a owns explicit restarts/status sockets, reserving before
credential writes or state changes. Download streams retain ActivityLease/process
ownership. 06b is Pending for priority/prefetch/queue-bound semantics unsupported
by shared FIFO capacity; 06c N/A for Rust background execution (browser retries
and transport state-machine timers remain outside this module). Baseline 166 tests;
final 168 pass, one existing ignored doctest. Added closed-admission mutation and
real HTTP WebSocket 503 regressions; existing real-process HTTP/WS signal/restart,
child-process and stream-lifetime tests pass. Clippy completes with 12 warnings and
an existing dependency future-compatibility notice. Changed-file formatting/diff
pass. Master rebased, tested tree verified, worktree/branch removed. No Spotify or
browser tests. See downloader's `docs/step-06-background-tasks.md`.

Pezzottflix `master` is integrated at `2d49435` from `f8159f0`, shared source
`06c7531`. 06a owns authenticated sync socket upgrades and cancel-safe drain;
closed admission returns 503 without phantom connections. The original 06b assessment cited dynamic replacement, nonblocking admission,
available-permit reporting and zero concurrency semantics. The subsequent
cron consumer reassessment found those cron registrations only in tests:
06b remains Pending for the production SQLite priority/due-time queue instead. 06c N/A: queue/download retry helpers have no production
callers; worker joins already belong to Lifecycle. Baseline 540 server tests;
final 542 pass, three existing ignored. New real authenticated HTTP rejection and
interrupted drain tests pass; the binary SIGINT/SIGTERM script passes with open
socket, HTTP/metrics and workers. All-target Clippy completes with warnings;
changed-file formatting/diff pass. Master rebased, tested tree verified, worktree/
branch removed. Frontend/browser/container suites not rerun. See Pezzottflix's
`docs/step-06-background-tasks.md`.

Favzetto `master` is integrated at `b8f5260` from `14d5b0d`, shared source `06c7531`.
06a owns catalog runs, ingestion analysis, research turns and three socket entry
points. Admission precedes persistent mutation; accepted work drains after HTTP and
assistant worker within the same deadline. Socket writer children abort and join.
06b remains Pending for durable claim/priority/per-user/workflow scheduling. 06c
shares retry calculation with an explicit legacy fallback for extreme accepted
inputs, preserving attempt/exponent normalization. Baseline 131 unit + 97 API tests
pass with two catalog runtime-bridge failures; separate two-process baseline passes.
Final all-target run: 234 pass, the same two failures. New retry/admission tests,
accepted mock-AI catalog drain and live socket signal/deadline checks pass. Clippy
completes with warnings; existing test-formatting debt retained. Master rebased,
tested tree verified, worktree/branch removed. Frontend/browser/container checks
not rerun. See Favzetto's `docs/step-06-background-tasks.md`.

Quentin Torrentino `master` is integrated at `4229de2` from `a8a921f`, shared source
`06c7531`. 06a owns pipeline jobs and dashboard upgrades with closed admission;
06b shares independent conversion/placement capacities, preserving zero-sized
stage waits, statistics and permit lifetimes. 06c N/A: configured processor retry
fields have no execution callers; manual retries remain domain operations.
Baseline core library 503 pass/one subtitle failure; server 192 pass/one MusicBrainz
failure. Final affected targets 708 pass, the same two failures, 12 ignored docs.
New pool/release/zero and interrupted-drain/admission checks pass, as do existing
pipeline, ticket/audit and real-process SIGINT/SIGTERM socket shutdown tests.
All-target Clippy completes with warnings, formatting/diff pass. Master rebased,
tested tree verified, worktree/branch removed. External/browser/container suites
not rerun. See Quentin's `docs/step-06-background-tasks.md`.

Meteonesto `master` is integrated at `080a852` from `6d032af`, shared source `06c7531`.
06c Partial: production runtime deadlines use ExecutionBudget with unchanged Tokio
clock/cancellation and fenced timeout recording. Configurable floating-multiplier
retry and durable pause remain application-owned gaps. 06a N/A: supervisor handles
and typed worker JoinSets already have structured abort/join ownership. 06b Pending
for durable weighted lanes, leases/fencing and hot configuration. Pinned Rust 1.97.1:
baseline 70 tests, final 72 pass. New cancellation and ready/boundary tests plus
two real-process unittest cases pass, including SQLite integrity and hot-reloaded
shutdown budgets. Strict Clippy's missing-Panics-doc error in unchanged
control_plane.rs:506 is reproduced on master. Formatting/diff pass. Master rebased,
tested tree verified, worktree/branch removed. Full provider/ingestion/browser suites
not rerun. See Meteonesto's `docs/step-06-background-tasks.md`.

SCT `master` is integrated at `6a999be` from `2651b53`, shared source `06c7531`.
06a owns fixed archive/recovery workers with TaskSet, preserving ordered abort/join
before writer release and catalog closure. HTTP-body/read-pin/archive-lease cleanup
remains app-owned. 06b Pending for PostgreSQL clock/claim/fencing authority; 06c
shares maintenance retry delay calculation while transaction clocks, classification
and exhaustion stay authoritative. Baseline 15 ordinary tests and two disposable-
PostgreSQL maintenance tests pass; final 18 ordinary and both PostgreSQL tests pass
with persisted deadline/exhaustion assertions. The same 91 qualification tests are
ignored in the ordinary suite. Strict affected Clippy, formatting/diff, server and
recovery builds pass. Five real-process cases verify active HTTP drain under both
signals, writer release/restart under both signals, and unsuccessful lease-loss exit.
The process harness was updated for current JSON logging/recovery fixtures and an
isolated static page; test containers are removed. Master rebased, tested tree
verified, worktree/branch removed, unrelated validation/CORS files preserved. Full
archive/media/S3/browser qualification not rerun. See SCT's
`docs/step-06-background-tasks.md`.


### Step 06 dynamic cron registry — library extension

Shared revision `0cff4b2` adds `CronRegistry` under the existing `task-scheduling`
feature. It supports runtime registration/replacement/removal, automatic schedule
enable/disable, per-entry skip/catch-up behavior, globally bounded due batches,
inspection and revision-based stale-notification checks. `next_due()` is a
cancellation-safe, application-driven wait; there is no hidden execution task.
Caller-owned execution can overlap and manual work is independent of cron
controls. Close is irreversible and leaves existing application work alone.

Fausto's clean local `master` was inspected read-only. Its jobs register at
startup, its admin API changes automatic cron enablement while running, manual
runs ignore that flag, and its job execution has no configured concurrency bound.
The new registry addresses these timing/control requirements without imposing
bounded execution or moving database history into simple-server. Fausto and all
other consumers are unchanged. **Fausto 06b was Pending (registry adoption) at this library-only checkpoint**;
the subsequent compatibility/E2E-verified adoption is recorded below.
All service matrix totals remain unchanged. This does not resolve durable claims,
fencing, priority queues, model batching or execution-policy gaps in other services.

Validation: untouched baseline **114 tests/doctests passed**; final **123 passed**,
including **nine new registry integration tests**. Coverage includes bounded
catch-up ordering, skip behavior with a full batch, exact UTC boundaries/future
years, enable/disable idempotence, replacement/removal/re-registration revisions,
closed admission, cancellation-safe waits and caller-selected control priority.
A real-clock test verifies delivery and independently held cron/manual executions
surviving disable/close until explicitly released and drained. All nine also pass
with HTTP/default features disabled. Strict all-feature/all-target Clippy,
formatting/diff checks and the HTTP-free `dynamic_cron` example pass.

Implementation used an isolated branch/worktree from `simple-server/main`
`2db31ef`. See the [contract](step-06-background-tasks.md#dynamic-timing-without-execution)
and [example](../examples/dynamic_cron.rs). No consumer changes, pushes or deployments.


### Step 06 consumer reassessment after dynamic cron

Reviewed all 16 other consumer checkouts against shared `0cff4b2`, with no consumer
changes or test reruns. See the [complete source-backed assessment](step-06-cron-consumer-reassessment.md).
Fausto remains the immediate registry candidate. Observo should retain stateless
shared recurrence with database-owned run history; Pezzottify could use the new
component for a separately requested cron feature, whose execution is currently
unimplemented. Other consumers' durable scheduling, priority/batching and existing
capacity integrations are unaffected.

**Correction: Pezzottflix's cron scheduler starts empty in production; job
registration occurs only in tests.** Its actual work runs through the SQLite
priority/due-time queue. Its 06b status remains Pending, now accurately described
as durable queue integration. The dormant cron engine's lookback and deferred
admission behavior are potential future shared-library requirements, not evidence
of current production usage. All matrix status counts remain unchanged.


### Step 06b Fausto dynamic cron adoption — complete locally

Fausto `master` is integrated at `4c89738`, from `44dc1b0`, using reviewed shared
revision `0cff4b24e375eeafb61834f6c03d5da30ae5f5dc`. Its production scheduler uses
CronRegistry through a private adapter for live schedule registration/removal and
skip-missed-tick timing. Job execution, persistence, manual/event triggers and
shutdown remain application-owned. Automatic executions can overlap; disabling
cron leaves admitted jobs and manual runs alone. The driver is joined by reference
so interrupted shutdown retains ownership. Fausto 06b is now **Done (scoped)**;
06a stays Done and 06c N/A. Scheduling totals: **5 Done, 8 Pending, 4 N/A**.

The old cron 0.12 parser remains at the boundary to preserve aliases/named fields;
shared cron calculation intentionally fixes the known future-year field-reset bug.
Tokio-cron-scheduler is removed from the lockfile. All four CI sibling-source pins
and active README instructions now use the reviewed shared revision.

Untouched baseline: **204 server tests pass**, one existing ignored doctest.
Final: **209 pass**, the same ignored doctest. The expanded **seven lifecycle and
six real-process tests pass against both old and migrated implementations**, using
a separate unchanged baseline checkout. New coverage proves automatic overlap,
disable/manual behavior, persisted completion and enabled-state restart, live
re-enable, invalid cron/manual execution, repeated interrupted drain and late
admission rejection. Two occurrence/grammar tests include the explicit calendar
bug regression. Existing HTTP drain/deadline, signal and logging checks pass.
Configured all-target Clippy completes with existing warning-level lint debt;
changed-file rustfmt and diff checks pass. Optional frontend/swagger, complete
workspace/plugin suites and Docker/Python E2E were not rerun.

Master was rebased onto the migration branch; ancestry and identical tested tree
were verified. Migration and baseline worktrees and the temporary consumer branch
were removed. See Fausto's `docs/step-06b-dynamic-cron.md` for the detailed contract
and evidence. No other consumer changed; no push or deployment was performed.


### Step 06B/06C completion extensions — library checkpoint

The shared library now offers independently usable weighted transactional-claim
selection, priority admission with cancellation-safe permits, batching readiness,
resource-demand checks against caller-owned transactional snapshots, fractional
quantized backoff and signed millisecond jitter. Claims, leases and recovery state
remain application-owned. Library validation: 132 tests/doctests, strict all-feature
all-target Clippy and nine HTTP-free new policy/selection tests pass. No consumer
adoption is claimed at this checkpoint; per-service migration evidence follows.

## Step 06b/06c completion pass — 2026-09-23

Shared revision `8edcf14` adds weighted backend claims, cancellation-safe priority
capacity, model batch readiness, transactional resource decisions, quantized
backoff and signed jitter. Its 132 tests/doctests and strict all-feature Clippy pass.

- Meteonesto `master` → `34cca34`: 06b weighted 8:4:2:1 claims; 06c fractional
  microsecond retry calculation now joins existing runtime budgets. SQLite owns
  eligibility, leases and pause. Baseline 56/final 57 targeted Rust tests and two
  real-process lifecycle tests pass. Strict Clippy retains one existing
  control-plane missing-panic-documentation error; provider/browser suites not rerun.
- Downloader `master` → `201f834`: 06b priority/FIFO admission with separate
  prefetch ceiling. Baseline 168/final 169 Rust tests/doctests pass (one ignored).
  Cancellation after grant now returns capacity instead of leaking it. Clippy
  completes with 12 existing warnings; Spotify/browser/container suites not rerun.
- Simple AI `master` → `986438f`: 06b per-model size/minimum/age/saturation
  decisions use BatchReadiness. Runner routing/reservations stay application-owned.
  Baseline 314/final 315 backend tests pass (one ignored doctest), including real
  loopback runner drain and capacity checks. Clippy completes with 12 existing
  warnings; GPU/browser suites not rerun.
- Simple Agents `main` → `0c0af51`: 06b profile/runner concurrency and all four
  resource dimensions use ResourceDemand inside the durable admission transaction.
  Workspace residency, compatibility, affinity and fencing remain authoritative.
  Baseline/final 138 service tests and strict all-target Clippy pass, including
  concurrent admission, reconnect/restart, workspace cleanup and real processes.

Each consumer was committed in its dedicated worktree, its development branch
rebased onto that commit, ancestry and tested tree checked, then temporary branch
and worktree removed. Unrelated Simple AI and Simple Agents edits were preserved.
Nothing was pushed or deployed. These are scoped primitive adoptions, not an
in-memory replacement for durable scheduling or inference/fleet protocols.

Shared revision `429b782` adds caller-owned durable polling and bounded batch
drivers plus composable preference/optional-rank selection. **137 shared tests and
doctests pass**, along with strict all-feature/all-target Clippy. Driver tests
cover per-outcome cadence, closed admission, completing accepted cycles, bounded
concurrency, panic observation and abort-on-drop cleanup.

- Crumbles `master` → `46e7f3e`: 06b production preference/backlog ordering and
  global resource admission use shared primitives inside SQLite reservations;
  06c queue polling and durable dispatcher transport/cancellation retries use
  shared doubling and signed jitter. Baseline 779/final 780 core/integration tests
  pass, including exact legacy seeded retry comparisons. Strict affected Clippy,
  both binary builds and real-process signal/drain/restart checks pass. The HTTP
  app retains one existing unused-import build warning; browser/container suites
  were not rerun.
- Favzetto `master` → `437aad5`: 06b production loop/ticket batches use the shared
  bounded executor. SQL priority/due selection, conditional claims, per-user caps,
  workflow state and retry bookkeeping remain application-owned. Baseline/final
  each pass 234 tests and reproduce the same two existing catalog runtime-bridge
  API failures. Passing cases include multi-ticket worker cycles, lifecycle and
  logging subprocesses. Clippy completes with existing warnings; browser/PDF and
  container qualification were not rerun. This sibling-path consumer has no
  revision checkout file; its migration report records reviewed source `429b782`.
- Pezzottflix `master` → `aa0700d`: 06b all eight SQLite queue workers use the
  shared outcome-sensitive cadence driver. Baseline 542/final 543 tests pass
  (three ignored), including actual workers processing 20 due jobs, preserving
  future eligibility and ceasing admission after stop. Both real-process signal
  cases pass with authenticated WebSocket and worker drain. Clippy completes with
  existing warnings; browser/container/external metadata services were not rerun.
  The unused production cron registry remains untouched; 06c remains N/A.
- SCT `master` → `0c750e7`: 06b archive/recovery polling uses shared completion-
  relative cadence, preserving immediate first execution and two-second delays.
  PostgreSQL due selection, SKIP LOCKED claims, leases and fencing remain in core;
  existing abort/join sequencing owns shutdown. Baseline/final each pass 18
  ordinary tests with the same 91 opt-in database/qualification tests ignored.
  Strict affected Clippy and both binary builds pass. Five real-process cases
  pass with disposable PostgreSQL (signals, HTTP drain, writer release/restart,
  lease loss); containers were removed. Full archive/media/S3/browser and ignored
  qualification suites were not rerun.

All eight consumer migrations are committed and integrated into their original
active development branches (Simple Agents `main`, the others `master`). Tested
trees and ancestry were verified before removing each migration worktree/branch.
SCT's untracked validation/CORS notes and Simple AI/Agents' active unrelated edits
were preserved. **Final 06b: 13 Done, 4 N/A; 06c: 8 Done, 9 N/A; zero Pending or
Partial in either module.** No push or deployment. Per-consumer
`docs/step-06-background-tasks.md` files contain detailed scope and evidence.

## Steps 08/09: combined auth — canary complete locally

One optional `auth` module now supplies synchronous/asynchronous identity and
access flows, explicit header credential parsing and an Axum-independent Tower
gate. Authentication and authorization are migrated together. Step 09 is absorbed;
Step 10 rate limiting and deferred Step 07 database helpers retain their numbers.
See the [contract and source assessment](step-08-auth.md). Favzetto was the first adopted canary; the first rollout below records subsequent adoption. Authentication and authorization are one column/module.

Shared implementation `0a629da`: baseline 137/final 145 tests/doctests pass,
strict all-feature/all-target Clippy passes, and seven auth contract tests pass
with default features disabled. The normal minimal dependency tree has no Axum
or Tokio. Tests cover credentials, ordered rejection, revocation, cancellation,
identity isolation, readiness and real HTTP access/public-route behavior.

Favzetto master `5b852cf` (from `437aad5`) uses shared HeaderCredential and Access
for production API-key verification and every existing admin check. Exact Bearer
case, duplicate-first semantics, fallback, local identity, HTTP errors and socket
query-key forwarding are retained. AuthService Debug deliberately stops printing
the configured key. The auth module and its tests no longer name Axum; other
service HTTP code remains transitional, so this does not claim complete Axum removal.

Baseline: 234 passing tests and two existing catalog runtime-bridge API failures.
A new 13-case credential matrix plus public-route check passes before migration.
Final: 237 pass, the same two failures (134 unit, 99 API, two lifecycle-process,
two logging-process). Added unit coverage proves admin denial, malformed-text
fallback and secret redaction. Configured all-target Clippy completes with existing
warnings and no new auth findings; new sections formatted and diff checks pass.
Frontend/browser/PDF/provider/container qualification was not repeated.

Committed in an isolated worktree; master rebased onto the canary, identical tested
tree and ancestry verified, temporary worktree/branch removed. Shared main includes
the library and tracker commits. At canary completion the count was **1 Done, 16 Pending
assessment/migration**; current totals follow below. Nothing pushed or deployed.

## Steps 08/09: first auth rollout — 2026-09-23

Three GPT-6 Sol agents at medium reasoning assessed LelloStore, Crumbles and
Simple AI in separate migration worktrees. Each service has applicable identity
and access behavior. Shared source remains `0a629da`; no library change was
needed. Application credential, permission and transaction authority is retained.

- Crumbles `master` → `fe46df2`: main HTTP session verification and named
  global/project access checks use shared flows; the integration daemon uses
  shared live-principal/capability evaluation. Exact Bearer/first-header behavior,
  malformed-header rejection without cookie fallback, CSRF, 403/404 concealment,
  revocation and authorization audit ordering remain intact. Baseline focused
  tests: 5 route authorization, 7 extractor and 2 integration auth tests pass.
  Final affected-package suites: 766 pass, 2 ignored, including a real-socket
  authenticated WebSocket test; formatting and strict Clippy pass. An existing
  unused route import was removed. Browser, external OIDC, Docker and production
  data qualification were not repeated. Existing ignored frontend assets were
  used unchanged for compile-time embedding.
- Simple AI `master` → `739cccc`: backend API-key/JWT/LAN identity selection
  uses shared `AsyncAccess`, including the disabled-account check. Shared
  `HeaderCredential` preserves exact Bearer and first-header semantics; any
  supplied header prevents LAN fallback. Shared `Access` covers admin middleware,
  SSE and WebSocket checks. Model/resource policy and the backend's separate
  runner-registration protocol token gate remain application-owned. The inference
  runner has no inbound user identity/access flow to migrate. Baseline: three
  selected LAN/admin HTTP tests and the new credential compatibility matrix pass.
  Final backend all-target suites: 317 pass, including admin role/user-list/denial
  and HTTP compatibility checks. Normal Clippy passes with existing warnings;
  strict Clippy stops on three pre-existing `derivable_impls` warnings in the
  untouched common crate. Unrelated README and semantic-scoring work is preserved.
- LelloStore `master` → `033bc86`: shared header parsing and `AsyncAccess`
  coordinate OIDC/JWT verification, user construction and registry observation;
  admin extractors use `Access`. Only exact Bearer/bearer and first-header
  semantics are accepted, with existing 401/403/500 errors and public routes.
  Resource/acquisition/publication checks retain their transaction boundaries.
  Baseline: 26 auth unit tests and the OIDC expiry/audience fixture pass; the new
  compatibility/registry-outage contract passes both in-process and over real
  HTTP before migration. Final backend all-target suites: 186 pass, 4 ignored,
  including that real HTTP contract, authenticated WebSockets and lifecycle
  subprocesses. Strict Clippy, changed-source formatting and diff checks pass.
  External live OIDC and Android build qualification were not repeated. CI and
  the active README now pin the reviewed shared revision.

All three migrations are committed and integrated into their original `master`
branches. Ancestry and identical tested trees were verified before removing the
migration worktrees/branches and owned temporary build files. Simple AI's unrelated
README and semantic-scoring work remains intact. No push or deployment. Detailed
scope and evidence live in each consumer's `docs/step-08-auth.md`. At completion of
this first wave: **4 Done, 13 Pending assessment/migration**.
These scoped adoptions do not yet claim complete consumer Axum removal.

## Steps 08/09: second auth rollout — 2026-09-23

Three more GPT-6 Sol agents at medium reasoning assessed Pezzottify, Meteonesto
and Simple Agents in separate migration worktrees. All three have applicable
production identity and access flows. Shared revision remains `0a629da`; the
library required no change. The coordinator reviewed production diffs and ran
socket tests where child tool approvals stalled.

- Meteonesto `master` → `6684b37` (implementation `4faff4b`): all three
  production binaries adopt shared auth. Pipeline
  identity resolves against the reloadable AccessControl, then shared access
  checks preserve permission and denial-audit ordering. The gateway retains strict
  single Bearer parsing, OIDC verification and product/rate policy. The weather API
  retains first-header edge credentials, all-hash constant-time comparison,
  loopback-canary bypass, failure metrics/challenge and credential-header removal.
  Baseline gateway 31 and API 70 tests pass; the pipeline baseline reached 46
  passes with four socket-binding failures in the sandbox. Final unrestricted
  pipeline all-targets pass 164 tests, including 22 HTTP cases and process checks
  for hot reload, signals and restart. Gateway 31 and API 71 tests pass, plus the
  explicit real-process gateway-to-API E2E: **267 total passing tests**. Formatting,
  locked builds and gateway/API strict Clippy pass. Pipeline strict Clippy has the
  same pre-existing missing-panics-doc finding before/after; allowing that single
  lint yields a pass. Pipeline cargo-deny passes with existing warnings. Docker
  and external-provider qualification were not repeated.
- Simple Agents migration source `83cd492`, documentation through `ac4b8bc`:
  shared `AsyncAccess` selects the existing bearer/browser verification path;
  `HeaderCredential` preserves exact single Bearer and origin rejection, including
  broker WebSocket parsing. Shared `Access` applies admin/session decisions to
  rows loaded inside existing transactions, preserving live revocation, audit
  order and 403/404 concealment. Browser origin/cookie/CSRF policy, runner leases,
  metrics credentials and runtime peer checks remain application-owned. Baseline
  authorization/session suites pass 32 tests. Final service all-target suites pass
  **139 tests, none ignored**, including live broker sockets, browser/OIDC,
  metrics revocation and process restart checks. Strict all-target Clippy,
  formatting and diff checks pass. External provider and full browser/deployment
  qualification were not repeated.
- Pezzottify `dev` → `3f2271ff`: `AsyncAccess` runs the production OIDC-first,
  legacy-token fallback verifier for HTTP and long-lived transport revalidation.
  Shared `Access` covers every named route permission policy using the existing
  session snapshot. The token68/case-insensitive/multiple-space/optional-raw-token
  parser remains application-owned; duplicate/malformed headers still cannot
  fall back to a valid cookie. Baseline at original consumer/shared revisions:
  32 focused unit tests plus 46 auth/MCP/permission HTTP tests pass. Final: the
  same 32 units plus 47 HTTP tests pass, including the new duplicate-header/cookie
  regression; open-connection revocation and permission refresh are covered.
  Formatting and repository-standard strict Clippy pass. Expanded all-target
  Clippy finds an existing items-after-test-module warning in an unchanged file.
  Unrelated suites, browser/Android and container qualification were not repeated.

All three migrations are integrated into their original development branches:
Pezzottify `dev` at `3f2271ff`, Meteonesto `master` at `6684b37`, and Simple Agents
`main` at `ac4b8bc`. Tested trees and ancestry were verified. Simple Agents initially
waited for coordination because concurrent account-display-name/UI edits overlapped
`auth_http.rs`. After the user authorized continuation, all eight edited/untracked
files were preserved through a temporary stash. The overlapping file merged
cleanly and matched the expected combination; hashes verified the other files
were unchanged. The edits remain uncommitted and separate from migration history.

The auth-only committed tree passed all 139 service tests and strict Clippy as
recorded above. Post-integration `cargo test -p simple-agents-service --all-targets
--locked --no-fail-fast` against the restored user edits completed with **138 pass,
1 fail, none ignored**. The sole failure is
`http_accepts_only_bearer_identity_and_prevents_nonadmin_provisioning`: its exact
JSON assertion expects the old identity response, while the uncommitted display-name
change adds `display_name: null`. The shared-auth credential matrix, browser/OIDC,
revocation, broker sockets, transaction checks and process tests pass. The unrelated
response-shape/test mismatch was left with that uncommitted work.

Migration/baseline worktrees, branches and owned temporary build assets are
removed, including the temporary preservation stash after restoration was verified.
Pre-existing Pezzottify Paravoid and Simple Agents worktrees are preserved. Both
tracker views at completion of this second wave showed **7 Done, 10 Pending assessment/migration**. No push or
deployment. Consumer Axum removal is still a separate completion milestone.

## Steps 08/09: third auth rollout — 2026-09-23

Three GPT-6 Sol agents at medium reasoning assessed Lello Auth, Fausto and Peerlo
in disjoint migration worktrees. All three have applicable production auth paths.
The reviewed shared source is `0a629da`; application-owned credential validation,
permission policy, errors and protocol boundaries remain authoritative.

- Peerlo `master` → `87b17b8`: the optional global bearer middleware evaluates
  shared `Access` with `HeaderCredential`; the optional Torznab query API key
  evaluates a separate shared flow. Exact Bearer, first-header selection, opaque
  and empty values, constant-time bearer comparison, existing 401 JSON and 200
  XML errors, and gate order are preserved. Configuration still disables absent
  gates. Debug output now redacts the configured bearer secret. DHT/tracker UDP
  and metadata TCP have no client identity/permission policy to migrate. Baseline
  old-pin checks: 9 in-process auth and 4 Torznab tests pass; four live HTTP tests
  were socket-blocked in the sandbox. Final unrestricted API all-target suites:
  **170 pass, none ignored**, including both-gate real HTTP coverage. Formatting
  and diff checks pass. Normal all-target Clippy passes with existing warnings;
  strict Clippy stops in untouched DHT/metadata dependencies. Source pin and active
  README instructions are updated. No external network or deployment qualification.
- Lello Auth `master` → `e4872f4`: shared `Access` coordinates optional browser-session identity,
  admin JWT audience/role checks, live-account admin/disabled checks, UserInfo,
  QR approval and UI admin access. Shared header parsing preserves exact Basic
  and Bearer schemes, first-header selection, empty/opaque values and the local
  Basic payload cap. Browser-source checks and CSRF still precede optional
  session lookup; existing anonymous fallback, revocation ordering, OAuth errors,
  redirects, signing/password/session state and transactional grants stay local.
  Server and embedded/external OIDC examples reuse the affected Axum package;
  webhook examples have no auth routes. Baseline: 188 library tests pass against
  both previous and reviewed pins. Full affected all-target suite: **340 pass**
  (190 unit, 150 real-TCP integration). After boxing callback errors for lint,
  190 unit and 39 TCP admin-UI tests pass again. Configured workspace Clippy passes
  on CI Rust 1.88; affected-package Clippy passes on local Rust 1.96 with CI's
  allowances. Two unrelated server-test warnings remain on the newer toolchain.
  Formatting and four CI-contract tests pass; one old router line was formatted.
  CI and README pins reference reviewed source; external-provider/deployment
  qualification was not repeated.
- Fausto `master` → `e40830a` (implementation `d9a3fc1`): HTTP and WebSocket
  identities evaluate shared `AsyncAccess`, with
  existing JWT/JWKS validation and distinct user-provisioning/lookup behavior.
  HTTP header extraction preserves exact Bearer, first-header selection and
  empty/invalid-text errors; WebSocket subprotocol/query precedence remains local.
  Shared `Access` covers admin and instance-write checks; owner/grant/event
  visibility and plugin placement stay application-owned. Baseline: 125 units,
  29 API and 19 audit tests pass; lifecycle tests were socket-blocked. Initial
  final suites: 205 pass, including real HTTP/process lifecycle checks. Follow-up
  tests additionally verify signed-token provisioning/reuse, admin allow/member
  denial and expiration using a local JWKS server and an in-process production
  router; configured WebSocket verification rejects missing/malformed tokens.
  The two HTTP fixture tests and new WebSocket unit test pass. Formatting and
  normal all-target Clippy pass with existing warnings; strict Clippy stops in
  untouched core code. External-provider, browser and WebSocket-upgrade
  qualification were not repeated. CI and active README pins are updated.

All three migrations are committed and integrated into their original `master`
branches. The coordinator reviewed production diffs and verified test evidence;
ancestry and tested trees match (Fausto's follow-up adds only tests/docs).
Migration worktrees/branches, private baseline library sources, targets and logs
are removed. Lello Auth's three unrelated untracked identity-provider documents
and pre-existing stale worktree remain untouched. At the end of that round, both tracker views agreed:
**10 Done, 7 Pending assessment/migration** for combined auth. No shared API
extension was needed; no push or deployment. This does not claim full consumer
Axum removal. Each consumer's `docs/step-08-auth.md` records detailed scope and
qualification limits.

## Steps 08/09: fourth auth rollout — 2026-09-23

Three GPT-6 Sol agents at medium reasoning assessed Observo, Pezzottflix and SCT
in separate migration worktrees. All three use authentication and access checks
in production. Shared source remains `0a629da`; service-owned credential,
permission and transaction policies remain authoritative.

- Pezzottflix `master` → `51fd1be`: shared raw-header extraction preserves
  first-value selection, empty/untrimmed session IDs and cookie fallback only
  for missing/non-text Authorization. `AsyncAccess` covers HTTP session/user
  lookup and the separate cookie-only WebSocket session/expiry flow. Shared
  `Access` covers named permission decisions; role policy, optional-auth error
  suppression, public byte-stream routes and distinct WebSocket behavior stay
  local. Baseline auth suite: 49 pass. Final Rust suites: **546 pass** (493 library,
  53 integration), two ignored doctests. Docker backend auth/WebSocket E2E:
  **10/10 pass**. Adopting the reviewed source exposed let-chain syntax unsupported
  by its Rust 1.87 Docker image; updating the image and documented minimum to
  Rust 1.88 fixed the build, and the release image passed. Normal all-target
  Clippy passes with existing warnings; targeted formatting and diff checks pass.
  Active pin and README are updated. Android/emulator qualification was not run.
  The migration worktree/branch and private target/venv files are removed; the
  E2E container was removed by the fixture. Generic E2E image, shared Docker
  cache and compose volume were preserved rather than pruned.

- Observo `master` → `96beec9`: shared `Access` and `HeaderCredential`
  preserve IP allowlist → API key → JWT fallback, first repeated headers,
  empty configured keys, exact Bearer syntax, trusted-proxy handling, OPTIONS
  and public metrics. All accepted sources retain the same route access;
  JWT verification and constant-time key comparison remain local. Baseline:
  **94/94** server unit tests. Final: **97/97**, plus passing real-process HTTP
  E2E covering auth, CRUD, persistence, restart and shutdown. Changed-source
  formatting and diff checks pass; normal Clippy passes with 36 existing
  warnings outside auth. Strict Clippy and whole-repository formatting retain
  existing unrelated failures. README and the active source pin are updated.
- SCT `master` → `7a0c6bd`: shared `HeaderCredential` and `AsyncAccess` preserve exact Bearer syntax,
  first repeated Authorization, supplied-header precedence over cookies,
  token/session verification and browser-content denial. Shared `Access` handles
  administrator and root-capability decisions after the existing transactional
  queries; live revocation, grants, ownership and database authority stay local.
  Baseline core/server test targets compiled, the nonignored suite passed outside
  the sandbox, and database-backed HTTP management passed before edits. Final:
  **18 nonignored core/server tests pass; 91 environment-dependent tests remain
  ignored in that default invocation**. Three explicitly selected PostgreSQL
  tests also pass: HTTP management (including the new credential matrix), HTTP
  storage/admin configuration, and core credential/identity/revocation/grant
  boundaries. These verify malformed-credential rejection, repeated-header precedence, cookie
  fallback rules, CSRF, admin denial, grant changes and stale-actor revocation.
  Formatting, diff checks and strict all-target/all-feature Clippy for core and
  server pass. Active pin and README are updated. Full archive/media/S3/browser
  qualification was not repeated.

All three migrations are committed and integrated into their original `master`
branches. The coordinator reviewed production changes and verification evidence;
branch ancestry and tested trees match. Owned migration worktrees, branches,
build targets and SCT's disposable PostgreSQL container are removed. SCT's
unrelated `.validation-work/` directory and `docs/step-04c-cors.md` are preserved.
At the end of that round, both tracker views agreed: **13 Done, 4 Pending
assessment/migration** for combined auth. Androidoscopy, Paranza, Pezzottify
Downloader and Quentin Torrentino were still pending. No shared API change, push or deployment was needed. This records scoped
auth adoption, not complete consumer Axum removal.

## Steps 08/09: final auth rollout — 2026-09-23

Four GPT-6 Sol agents at medium reasoning assessed Androidoscopy, Paranza,
Pezzottify Downloader and Quentin Torrentino in separate service worktrees.
The session allows three workers alongside the coordinator, so the fourth
agent started after the downloader assessment finished. The coordinator owns
these trackers and reviews the implementation and integration evidence.

- Pezzottify Downloader `master` → `32f6ff7`: **N/A** after source assessment.
  The production Puppeteer TCP router exposes its pages, status, credential
  upload, restart, proxy, health and WebSocket routes without a caller identity
  or permission gate. `/credentials` validates Spotify provider credentials,
  not the HTTP caller. The child downloader uses a mode-0700 Unix socket;
  filesystem access remains its boundary. Login/librespot credentials are for
  outbound provider sessions. Adding application auth would introduce behavior
  rather than migrate it. Only `docs/step-08-auth.md` changed; the existing
  shared source pin and build instructions remain unchanged. Router/startup,
  credential, proxy, socket and process-test source were reviewed; diff checks
  pass. A targeted process test was stopped during dependency compilation;
  **no runtime test result is claimed** for this documentation-only assessment.
- Paranza `master` → `5a28680` (implementation `5b750bb`): shared `Access`
  resolves runner subject bindings and enforces the claimed node, and verifies
  the PCM sender subject for submit/list/detail/cancel paths. Exact error strings,
  first matching header, active-sender precedence, 404 visibility and request
  ordering are preserved. TLS certificate proof, session replacement and stale
  sessions remain local. The Android signed workload claim retains its local
  registered-slot, canonical payload, binding, fingerprint and signature checks.
  Baseline: 47 core and 46 app tests pass; eight app tests were socket-blocked
  in the sandbox. Final **full workspace suite passes outside the sandbox**,
  including 48 server-core and 55 server-app tests plus desktop Unix sockets.
  All workspace test targets compile. New tests cover runner denial messages
  and PCM missing/empty/duplicate headers and 403 responses. Strict Clippy has
  existing `items_after_test_module` and pagination `collapsible_if` findings;
  the formatting check retains an unrelated app difference. Active revision,
  lockfile and README pin are updated to `0a629da`. Tested tree and ancestry
  verified, original checkout clean, owned worktree/branch/build files removed.
- Androidoscopy `master` → `50b1cda`: shared `Access` protects the v2 controller
  HTTP/WebSocket gate and verifies the LAN `AUTHORIZED` frame. Exact Bearer,
  first-header selection, invalid-Bearer-to-cookie fallback, Host/Origin policy,
  empty 401 responses and protected login/events placement remain unchanged.
  LAN current-session, 32-byte credential and pinned-secret checks preserve
  their errors; TLS, HMAC/exporter pairing and credential storage remain local.
  Legacy v1 is still explicitly separate and unauthenticated. The pre-edit HTTP
  test compiled but was blocked by sandbox listener permissions before assertions;
  no unrestricted runtime baseline is claimed. Final **76/76 Rust tests pass**
  outside the sandbox, including HTTP fallback/route checks, new LAN frame cases,
  TLS pairing, TLS/WebSocket, UDP, logging and WebSocket integration. All test
  targets compile. Library Clippy passes with the existing derivable-impl lint
  allowed; strict all-target Clippy and whole-repository formatting retain
  unrelated existing findings. Changed Rust files are formatted. Pin, lockfile
  and README reference `0a629da`. Android SDK/dashboard/end-to-end suites were
  not repeated.
- Quentin Torrentino `master` → `191a89c`: shared `AsyncAccess` now evaluates the `/api/v1` access
  flow, including WebSocket requests. Metrics and dashboard remain outside the
  gate. The custom verifier retains exact `Bearer`/`bearer` parsing, last text
  duplicate value, invalid-text skipping, x-api-key fallback, empty configured
  key behavior, constant-time comparison and identity creation. Explicit `none`
  still bypasses the custom verifier and installs anonymous identity. There are
  no separate resource/named-permission checks; existing error/metric mapping and
  `AuthUser` fallback stay local. Baseline middleware: **7/7 pass**. Final selected
  checks: **45 pass** (25 server library including real-HTTP public/protected/WS
  coverage, 15 core auth, one existing HTTP E2E health and four process startup
  tests). Credential matrices cover precedence, duplicates, source IP and none
  mode. Test fixtures now hold and clean their temporary databases and await
  aborted HTTP servers. All-target Clippy passes with existing warnings; strict
  mode stops in unchanged agent observer code. Diff checks pass. Active source
  pin and README build guidance are updated. External-provider, full unrelated
  workspace, dashboard/browser and container suites were not run.

All four assessment/migration commits are integrated into their original
`master` branches, with reviewed ancestry and tested trees. Owned temporary
branches, worktrees and build directories are removed. No shared API extension
was needed. No unrelated work was overwritten and nothing was pushed or deployed.
The rollout is complete locally: **16 Done, 1 N/A, 0 Pending**. Application-owned
protocols, cryptography and domain permissions remain as documented; this does
not claim complete consumer Axum removal. Step 10 rate limiting is next.

## Step 10: rate limiting — 2026-09-24

Shared optional `rate-limit` implements 10a budgets/storage, 10b admission/outcome
policies and 10c HTTP adaptation. The contract is in
[step-10-rate-limiting.md](step-10-rate-limiting.md). Built-in budgets are local to
one process. Services retain identity/proxy trust, route placement, wire errors,
cryptographic protocols and transactional business quotas. Existing governor
versions are differential-test dependencies only. Simple AI and Pezzottify HTTP
canaries are integrated. At the canary checkpoint, Step 10 status was: **1 Done, 1 Partial, 15 Pending**. Remaining
services await individual applicability and behavior review.

Library verification: baseline **145** tests/doctests pass; final **163** pass,
including 18 new rate/admission/HTTP tests. The differential test compares 6,000
weighted decisions and retry durations with each of governor 0.8 and 0.10.
Minimal-feature rate suites pass **17** tests without Axum. Strict all-feature,
all-target Clippy and formatting pass. The normal minimal dependency tree has
HTTP, body, Tower and pin-projection primitives only; no Tokio or governor.
Socket tests run outside the sandbox; the initial sandbox baseline failed at
listener binding, not at a library assertion.

### Simple AI canary

`master` advanced from `739cccc` to `f6455b67c6e14fb900c4540902ad0159825aa178`,
pinned to library `925ea25153a35e1fdb748add4880bc0e7d384ae6`. Backend `/v1`
rate middleware now uses shared GCRA budgets and keyed storage. Startup enablement,
configured rate/burst, forwarded-header/peer/unknown key precedence, route scope,
429 body, floor-with-minimum-one Retry-After and logging remain unchanged. The
existing local HTTP wrapper remains; Pezzottify exercises the shared HTTP layer.
The inference runner has no corresponding inbound rate limit. Storage explicitly
preserves the old unbounded policy; a capacity cap is a separate policy change.

Baseline four rate tests passed. Two new identity and real-HTTP tests passed on
the original governor implementation before migration; all six passed afterwards.
Full backend suite: **319 passed**, one pre-existing ignored doctest. All-target
Clippy passes with existing findings in untouched common/backend code; changed
Rust formatting and diff checks pass. Direct governor dependency is removed.
Active build revision and README checkout instructions are updated.

The original development branch was rebased onto the committed worktree branch;
the integrated tree equals the tested tree. Original README edits were restored
and their diff verified; every original untracked file hash was preserved. The
owned worktree, branch and temporary README preservation stash were removed.
See `simple-ai/docs/step-10-rate-limiting.md` in that repository.

### Pezzottify HTTP canary

`dev` advanced from `3f2271ff` to `e428bd67a15ce91e3fd89a56b31ac207ea45eab6`,
pinned to library `925ea25153a35e1fdb748add4880bc0e7d384ae6`. All formerly
Governor-backed HTTP gates now use shared keyed budgets and `RateLimitLayer`:
global, stream, catalog/content, search, writes, user content, per-device analytics
and login IP/account burst/sustained limits. Existing shared instances, integer
millisecond intervals, burst sizes, application identities, route placement and
layer order are retained. Password and OIDC routes still share the IP budgets.
Login cleanup retains its 600-second schedule and only drops replenished keys.
The previous unbounded storage policy is explicit, rather than silently capped.
The local adapter owns only identity and legacy error rendering.

Before migration, **33** focused tests passed. Two new missing-identity and real
HTTP contract tests passed with the old implementation; all **35** passed after
migration. They retain 500/missing-key text, 429/default body, both Retry-After and
X-RateLimit-After with floor rounding, body forwarding and public-route behavior.
Existing tests cover reconnects, cross-IP account limits, user/device identity
and login body restoration. Full backend suite: **1,432 passed**, zero failed,
**36 existing ignored** (including two doctests). This covers real production
routers, auth/permissions, MCP, reports, streaming, sync, WebSocket, lifecycle and
mixed workloads. All-target Clippy passes with findings only in untouched
enrichment/background-task tests and an existing transitive future-compatibility
notice. Formatting and diff checks pass. Docker/release builds were not rerun.
Direct governor/tower_governor dependencies are removed; build revision is updated.

Overall status is **Partial**, not Done: MCP counters share one per-user anchor
across three categories, reset at strictly `elapsed > 60s`, accept zero limits
and expose existing usage counters. Shared fixed windows use independent anchors
and `elapsed >= window`. Migration needs a compatible grouped-window API or
application-owned policy callback; existing MCP behavior remains intact.
Database-backed download/report quotas and outbound enrichment pacing remain
application-owned and are not claimed adopted by this HTTP canary. See
`pezzottify/docs/step-10-rate-limiting.md` in that repository.

The original `dev` branch was rebased onto the committed migration branch; its
tree equals the tested tree. The owned worktree and branch were removed. All
pre-existing Pezzottify worktrees/branches were preserved.

### Integration and cleanup

Shared implementation checkpoint `925ea25153a35e1fdb748add4880bc0e7d384ae6` is
integrated into `main`; the final tracker/roadmap record follows on that branch.
All three repositories used dedicated migration worktrees. Owned worktrees,
branches, temporary build directories, symlinks, scripts and logs were removed
after recording evidence. HTML script execution, local links and Markdown/HTML
status parity were checked: 17 services, 15 step columns, with combined auth
unchanged at 16 Done / 1 N/A. No push or deployment was performed. Consumer Axum
removal remains a later milestone; Step 07 remains deferred.


## Step 10: first parallel rollout — 2026-09-24

Four service assignments: Favzetto, Lello Auth, Meteonesto and Crumbles, using
GPT-6-sol workers. Three workers can run alongside the coordinator in this
session; Crumbles started after Meteonesto freed a slot. The coordinator owns
this record and the HTML matrix; agents own disjoint consumer repositories.

Compatibility extensions were required before changing consumer semantics:

- `8392ef42a686261fa12ca48e83ecbe290f965bb8` adds caller-owned `TokenBucket`,
  preserving the gateway's fractional `elapsed * per_minute / 60` accounting and
  ceiling retry durations. Refill can precede a concurrency gate. The existing
  heterogeneous store, global capacity and permit lifetime remain service-owned.
  50,000 differential attempts include concurrency-denied observations; focused
  tests also cover weighted charges, invalid cost, refill caps and backward time.
  Explicit `ClockRegression::Reanchor` additionally preserves timestamps sampled
  before a mutex and delivered out of order; 20,000 differential observations
  cover this choice. The default remains monotonic clamping.
- `FailureCounter::with_policy` adds explicit blocked-outcome counting and
  independent failure-window retention on cooldown expiry. Defaults stay intact.
  This preserves Crumbles' already-in-flight failed attempts and independent
  window/cooldown behavior. 15,000 differential transitions plus explicit block
  extension, expiry and reset checks cover the compatibility policy.

Lello Auth additionally uses governor 0.6. Its full-burst tolerance and initial
one-interval debt differ from 0.8+: after long idle it can admit an extra unit.
`RefillPolicy::ExtraIdleCredit` explicitly preserves this behavior, while the
shared default remains strict. A direct old-version regression and 15,000
weighted/idle differential transitions validate the compatibility mode.

Shared focused baseline: 12 tests passed. After the compatibility extensions, 21 focused tests
and **172 full-suite tests/doctests** pass. The no-default-feature rate suite
passes **26** tests without enabling Axum. Strict all-feature/all-target Clippy
and formatting pass. Shared source changes were committed in the coordinator's isolated worktree
and integrated into `main`; workers used immutable reviewed library revisions.
Integrated service evidence follows below.

Explicit raw timestamp support also covers `Budget` and `FailureCounter` for
Crumbles' pre-lock observations. Fixed windows retain saturating retry behavior;
independent cooldown preflight retains expired deadlines and does not advance the
failure window. 24,000 out-of-order outcome/preflight transitions and exact window
and deadline regression tests pass. Defaults retain monotonic clamping.


### Meteonesto — integrated

`master` advanced from `6684b372453a5861fd2621aa23e924985200bcbe` to
`e39aa72209bcf730f4b5cda0b8016985f9f50cdb`, reviewed against immutable shared
revision `6fae02f6e409ecc874fee8414ffd2c06fe769364`. Gateway IP and subject
admission now use `TokenBucket` with explicit `ClockRegression::Reanchor`.
The single 10,000-entry map, scope/tier keys, sweep/expiry, concurrency-before-charge,
permit lifetime, rejection metrics and 429 JSON/Retry-After remain gateway-owned.
Existing route boundaries and exemptions are unchanged; no new outer HTTP layer
is installed. Weather API/pipeline have no inbound rate limiter to migrate;
outbound provider retry and internal resource/concurrency controls remain separate.

Baseline gateway check passed 24 unit tests, 7 deployment checks, formatting,
strict Clippy and build. Baseline real HTTP E2E passed outside the socket sandbox;
the new 429 response assertion also passed before migration. Final check against
the pinned source passed **26 unit tests, 7 deployment checks**, strict Clippy,
formatting, build and real HTTP E2E. Admission tests preserve Product/Combined
shared budgets through refill, concurrency denial without charging, and out-of-order
observations. Other binaries' full checks were not rerun for this gateway change.
All three active README source pins and local migration evidence were updated.

The original `master` was rebased onto the migration branch and its tree matched
the tested tree. Original checkout is clean. The owned migration worktree, branch,
frozen library checkout and temporary test artifacts were removed. No push/deploy.
See `meteonesto/docs/step-10-rate-limiting.md` in that repository.


### Favzetto — integrated

`master` advanced from `5b852cf` to
`3339c4a30854ce64f5b0a8a8acdb227b98d5dff2`, tested against immutable shared
revision `8392ef42a686261fa12ca48e83ecbe290f965bb8`. Production outer HTTP
middleware uses `KeyedLimiter` for per-client global and normalized endpoint
budgets. Existing Forwarded/X-Forwarded-For/X-Real-IP/unknown identity precedence,
configuration and integer-millisecond intervals, independent bursts, disablement,
global-before-endpoint charge order, body-limit placement and JSON 429 errors
remain unchanged. Keyed storage deliberately remains unbounded. The local HTTP
wrapper is retained; this is adoption of shared budgets and storage.

Focused baseline: one unit and three HTTP tests pass. Final focused checks:
one unit and **four HTTP tests** pass, including charging order, independent
endpoint budgets and error schema. Broader backend results: **134 unit, 100 API,
two lifecycle and two logging tests pass**. Two API failures remain:
`catalog_research_runtime_bridge_approves_runtime_draft` and
`catalog_research_runtime_bridge_rejects_runtime_draft`. A targeted run on the
untouched starting revision reproduced both: HTTP 400 for terminal `completed`
flow transitions to `completed` and `waiting_user`, respectively; a neighboring
bridge case passed. These are documented baseline failures, not a fully green
API suite. All-target Clippy passes with `--cap-lints warn`; this is not strict
warning-free validation. Whole-repository formatting has existing differences;
changed limiter/new test formatting and diff checks pass.

The original `master` was rebased onto the migration branch, tested tree verified,
and the consumer worktree/branch removed. Original checkout is clean. The detached
reviewed library checkout was also removed. Active build source requirements are
recorded in `favzetto/docs/step-10-rate-limiting.md`; this repository has no root
README or CI checkout workflow to update. No push/deployment was performed.

### Crumbles — integrated

`master` advanced from `4f83d48bb8f5402fbdcf22afa0e15d9bbfe4406a` to
`dea9e042042ceae6b0e62b25a7d9e576699a99bf`, tested against immutable shared
revision `b89fcfe5807e9a2bfc1c73380e114c01cc8c3ae7`.
Login, password-change and personal-token failure accounting now uses
`FailureCounter` with counted in-flight failures, independent window retention
and raw timestamp reanchoring. Peer/account maps, capacity eviction, success
reset scope and the threshold attempt's 401 response remain unchanged.
Authenticated MCP admission uses shared fixed-window `Budget`; per-user keys,
10,000-entry cap, floor-based retry seconds and protocol ordering are preserved.
Durable dispatcher admission remains database-transaction-owned, including
reservations and sliding quotas: **Partial**, with that scope still pending.

The broad baseline login filter passed seven tests (five limiter-module tests,
one configuration and one metrics test); the existing MCP HTTP budget test also
passed. Two added login regressions passed against the original implementation.
Final focused checks passed seven login-module and 27 MCP tests. The full
workspace/all-target suite passed **1,441 tests, two ignored** outside the socket
sandbox. Socket fixture failures in the sandbox were resolved by that rerun.
Strict workspace/all-target Clippy, workspace formatting and diff checks pass.

The original `master` was rebased onto the tested migration branch and is clean.
The migration worktree/branch and private build artifacts were removed.
README, `simple-server.rev`, abuse-control documentation and the local
`crumbles/docs/step-10-rate-limiting.md` record the adoption and source pin.
No push or deployment was performed; pinned local shared revisions must be
published before a fresh remote CI checkout can fetch them.

### Lello Auth — integrated

`master` advanced from `e4872f48c8bdaf3bb5e22cb9c31cc4e1bc29f48b` to
`1533adb24dc0d1a2d33e730eb600064919e32937`, tested against immutable shared
revision `b239e3728670a5362f63707956f410314aeee6e6` on the CI Rust 1.88 toolchain.
Production endpoint limiters now charge shared `Budget` with
`RefillPolicy::ExtraIdleCredit`, preserving governor 0.6 refill and idle behavior.
The single cross-endpoint capacity, per-class overflow buckets, hashed bounded
identity components, eviction, metrics, handler order and errors remain local.
Configuration still restores one token per full configured window. Direct
governor usage and its dependency were removed; CI checkout pins and README
were updated. Database-backed device-code polling remains authoritative for
OAuth `slow_down`, so adoption is **Partial** with that durable scope pending.

Focused baseline passed 20 tests; final focused suite passed 22, adding refill
boundary and governor 0.6 long-idle coverage. Exact pinned all-target workspace
validation passed **962 tests, 14 ignored** with socket access; ignored tests
require the optional PostgreSQL fixture. Strict Clippy passes on CI Rust 1.88,
workspace formatting and four CI contract tests pass. Local Rust 1.96 strict
Clippy reports two pre-existing `cmp_owned` warnings in unchanged server code;
this is documented separately from the successful CI-toolchain check.

The original `master` was rebased onto the tested migration branch and its tree
verified. The three pre-existing untracked identity-provider research files and
pre-existing detached retest worktree were preserved. Owned migration/review
worktrees, branch and build artifacts were removed. See
`lello-auth/docs/step-10-rate-limiting.md` in that repository. No push/deployment.

### First batch completion

All four service migrations were committed in dedicated worktrees and integrated
by rebasing their original `master` branches onto the migration branches. Owned
worktrees, branches and temporary build artifacts were removed. The coordinator
committed both trackers and integrated its branch into `simple-server` `main`.
First-batch checkpoint: **3 Done, 3 Partial, 11 Pending**. Partial rows retain
explicit remaining scope; they are not N/A. No changes were pushed or deployed.
Step 07 remains deferred until after rate limiting and the Axum-removal milestone.

## Step 10: second parallel rollout — 2026-09-24

Four GPT-6-sol assignments: Peerlo, Fausto, LelloStore and Pezzottflix. Three
workers run alongside the coordinator; Pezzottflix started after LelloStore's
assessment freed a slot. The coordinator owns shared code and both trackers.
All four assignments are integrated; the remaining seven unassessed service
rows stay pending.

### Shared compatibility extensions

`6da4492d4914a62c9185ffaf87ea504fd16673f0` adds `PerSecondTokenBucket`, preserving
Fausto's fractional per-second arithmetic, accepted zero/nonfinite/negative
configuration values, exact retry conversion, construction/admission observations
and caller-owned cleanup. It deliberately leaves validated GCRA and per-minute
bucket semantics unchanged. 72,000 differential observations plus explicit
fractional, backward-clock, zero-capacity and nonfinite cases pass.

`df622c8605a12d4e61865b69a87e057232dd1a8b` adds `FailureWindow` (expiry anchored
at the first failure, preflight does not charge) and `FailureLatch` (expiry cleanup
explicitly owned by the application, late outcomes never extend/restart a latch).
They preserve Pezzottflix's zero configuration and late-outcome behavior. Tests
include 36,000 independent old-model sequences plus exact threshold/window,
explicit reset, expiry and clock-range cases. Failed deadline arithmetic does
not consume the threshold outcome.

Shared baseline: 21 focused tests. Final focused suite: **28 tests** with no
default features and only rate-limit enabled. Full all-feature suite: **179
tests/doctests** pass, along with strict all-feature/all-target Clippy, formatting
and diff checks. The focused suite was rerun after a lint-only simplification.
Both shared commits were integrated into original `main`; consumer agents verify
immutable source revisions independently.

### LelloStore — assessed N/A

Active clean `master` advanced from `e2f4cd5c5e502233e0918b2f5301d4ebc5bb916f`
to `d032a7db04efc540c144b6b4d1536004caa5631d`. Assessment reviewed shared source
`1d4fd30a20c789dc2797dba08432cd837860749c`. Production API and metrics routers
have no inbound quota/concurrency admission policy, configuration, limiter
dependency, 429 or Retry-After mapping. No rate-limit feature was added.
The SPEC's rate-limited JWKS refresh refers to a 30-second outbound fetch
cooldown for unknown key IDs plus a refresh mutex; it is an authentication-cache
safeguard returning KeyNotFound, not an inbound request quota. This remains local.

The documentation-only assessment is recorded in
`lellostore/docs/step-10-rate-limiting.md`. Baseline/final diff checks passed;
no executable build/runtime test was needed. Original `master` was rebased onto
the dedicated migration branch; ancestry/tree equality verified, checkout clean,
and owned worktree/branch removed. No push/deployment.

### Fausto — integrated

Clean `master` advanced from `e40830a90eac115c35f23c248136e1072d463a23` to
`2adbdcf596093cae39f5e0c49b7e874f0cf792b7`, verified against immutable shared
revision `6da4492d4914a62c9185ffaf87ea504fd16673f0`. All five production API tiers
(auth/admin/search/general/federation) use `PerSecondTokenBucket`. Configuration,
per-IP DashMap locking, trusted-XFF choice, route placement, disabled mode,
300-second sweep and 600-second idle eviction stay local. Existing 429 JSON and
exact retry seconds are retained, including unusual accepted float values.
Other binaries have no rate-limit path; existing exempt routes remain exempt.

Baseline: 15 limiter unit tests and one production-router audit passed; two new
retry contracts also passed against legacy code. Final workspace suite passed
**659 tests, ten ignored** outside the socket sandbox, including the active router
audit. An additional response-body contract was added afterward; all **15 final
focused limiter tests** passed. Four removed private bucket algorithm tests are
superseded by shared arithmetic/oracle coverage. Workspace formatting and
all-target Clippy pass with existing warnings (not a warning-free Clippy run).
An initial sandbox OIDC listener failure passed in the full socket-enabled rerun.

Active README/CI shared-source pins and `fausto/docs/step-10-rate-limiting.md`
were updated. Original `master` was rebased onto the tested migration branch,
ancestry/tree equality verified, checkout clean, and owned worktree/branch,
frozen library snapshot and private target removed. No push/deployment.

### Peerlo — integrated, Partial

Clean `master` advanced from `87b17b8badaf39788251aa4787b48155cc48e544` to
`861552bddca4a2da303036d532ac81cf39ef6274`, tested against immutable shared
revision `1d4fd30a20c789dc2797dba08432cd837860749c`. Production API global and
per-IP admission now uses `KeyedLimiter` with governor 0.6 idle-credit behavior
and integer-nanosecond refill. Global-before-IP charging, socket identity,
unbounded map and clear-all cleanup, route placement and fixed 429 JSON plus
Retry-After: 1 remain unchanged. Rates above one billion per second retain the
old zero-interval unlimited behavior, including per-IP map tracking; an actual
governor oracle checks that edge. Governor remains test-only in the API crate.

The BEP 51 crawler's loop-anchored minute quota/zero maximum and DHT bootstrap's
persisted epoch gap/calendar-day quota remain pending a compatible design;
independent process-local budgets cannot substitute for those policies.

Baseline: nine limiter tests pass. Final: **11 focused tests, 170 API tests and
two HTTP tracing tests** pass, including real HTTP rejection, response shape,
startup and middleware composition. The full workspace passed before the final
high-rate adjustment; the full API suite passed again after it. Socket-denied
sandbox failures passed in the unrestricted rerun. Formatting, diff checks and
API all-target Clippy pass with existing warnings. Active README/source pins and
`peerlo/docs/step-10-rate-limiting.md` were updated. Original `master` was rebased
onto the tested branch, ancestry/tree equality verified, clean original checkout
retained, and owned worktree/branch, snapshot and build artifacts removed.
No push/deployment.

### Pezzottflix — integrated, Partial

Clean `master` advanced from `51fd1be20fae694c4288074c42108b5d1ed6e57b` to
`06824916af4ef8c9ae7f6da6539c507659c9d151`, tested against immutable shared
revision `df622c8605a12d4e61865b69a87e057232dd1a8b`. The active login limiter uses
`FailureWindow` per IP and `FailureLatch` per normalized email. Production
preflight/outcome calls preserve cleanup order, IP-first rejection, floor retry
seconds, success reset and unbounded identity storage. Exact timestamp tests
exercise late thresholds, window expiry, zero limits/duration, and email outcomes
arriving after a lockout expires but before preflight cleanup.

The existing governor HTTP middleware is unregistered, so it is not counted as
production adoption and remains unchanged. Active TMDB semaphore waiting and
SQLite UTC-calendar-day download quotas remain application-owned and pending;
shared rejecting/in-memory admission would change those contracts.

Baseline on the earlier shared pin passed five existing and three added legacy
contract tests, then **548 tests, three ignored** in the full suite. Final pinned
suite passed **549 tests, three ignored**, including nine focused login tests.
The initial sandbox WebSocket bind failure passed outside the socket sandbox.
An intermediate timing test exposed precision problems with adjusting wall-clock
origins; final production helpers accept explicit observations and deterministic
boundary tests pass. All-target server Clippy completes with existing warnings;
targeted Rustfmt and diff checks pass. Active Cargo feature, README and source pin
were updated along with `pezzottflix/docs/step-10-rate-limiting.md`.

Original `master` was rebased onto the tested branch and its identical tree and
base ancestry verified. The original checkout is clean. Owned migration branch,
worktree, library snapshots and private build target were removed. No push/deploy.

### Second batch completion

All four service assignments were committed in dedicated worktrees; original
`master` branches were rebased onto the migration branches and tested trees
verified. Owned worktrees, branches and temporary build artifacts were removed.
The coordinator integrated both shared extensions and committed both status
trackers on `simple-server` `main`. Second-batch checkpoint: **4 Done, 5 Partial, 1 N/A,
7 Pending**. Shared source pins are local commits; nothing was pushed or deployed.
Step 07 remains deferred until after rate limiting and verified Axum removal.

## Step 10: third parallel rollout — 2026-09-24

Four GPT-6-sol assignments: Observo, Simple Agents, Pezzottify Downloader and SCT.
Three workers run alongside the coordinator; SCT started when Observo completed.
Each assessment inspects actual production paths, not just limiter names, HTTP
429 codes or transitive dependencies. The coordinator owns both central trackers.
Reviewed shared revision: `15ad34b5ee7c2f36720a13467984ff63c042b326`.

### Observo — assessed N/A

Clean `master` advanced from `96beec99c5f93ec44b5b26f8fd2a8fbb09a30c87` to
`2427747ef6e21e8ebaf41296ee8bd93bdd3e832e`. Production startup, routes,
authentication, configuration, errors and dependencies have no request quota,
cooldown, concurrency admission or rate rejection. Crawl pace settings are data
managed by the service; webhook WorkTracker admission closes during shutdown.
Neither is a request limiter. Upstream 429 recognition selects an extraction
fallback. The MCP program uses stdio; express-rate-limit is only transitive in its
lockfile. The assessment also searched workers, extension, scripts and deployment
files. No rate-limit feature, dependency or source pin was changed.

`observo/docs/step-10-rate-limiting.md` records the evidence. Baseline/final diff
checks passed; no executable tests were needed for this documentation-only change.
Original `master` was rebased onto the isolated assessment branch; ancestry and
identical tree verified, original checkout clean, owned worktree/branch removed.
No push/deployment.

### Simple Agents — assessed N/A

Clean `main` advanced from `a4c4d499ce083a4432b8e657e96a129124695faf` to
`9ce77fb8c00317109f3254d65775fe12b6bc8013`. Production service/Runner code
has no request quota, cooldown or Step 10 HTTP admission policy. The HTTP 429
`EvidenceFull` response means durable evidence storage capacity is exhausted,
not that a request-rate budget was exceeded. Evidence promises and writes retain
their SQLite transaction and per-session storage limits.

Fleet reservations are Step 06 task admission: authorization, replay/generation,
profile/Runner compatibility and several resource dimensions share the transaction
with attempt creation, state/journal writes and durable reservation/audit records.
A process-local limiter would change authority; wrapping the transaction in a
single policy callback would not extract any behavior. The standalone Runner
CapacityCoordinator has no production call sites, and GitHub 429 parsing handles
an outbound provider response. No Step 10 feature or active source pin changed.

Baseline checks against the immutable reviewed shared source passed **13 fleet
and 22 session tests** using a private target. The local evidence file is
`simple-agents/docs/step-10-rate-limiting.md`; the result changes documentation
only. Original `main` was rebased onto the isolated assessment branch and its
tested tree verified. Original checkout remains clean; the pre-existing
`/tmp/cr173-simple-agents` worktree reference was preserved. Owned migration
worktree/branch, frozen shared archive and private target were removed.
No push/deployment.

### Pezzottify Downloader — assessed Pending

Clean `master` advanced from `32f6ff745252a6bcaa7e96bacea46e9163e2f7ea` to
`66c68948fd00aff6d06860889a182cc1a3dfa56a`. The Python production entry point
`scripts/cron_downloader.py --config ...` enforces durable SQLite sliding quotas
on completed download attempts: strict `finished_at > now - window` over optional
30-minute, 2-hour and 24-hour windows. The minimum remaining budget controls the
run; no configured limits yields 100. Checks run before authentication and each
album, and only successful completed upload attempts consume allowance. This is
a used capability, not N/A. Rust fixed-window/process-local budgets cannot
preserve its sliding history, completion accounting and language boundary.
A compatible durable integration remains **Pending**; no Step 10 adoption is claimed.

Rust HTTP rate budgets are N/A. Downloader's sole 429 reports a full priority
queue after cache lookup; its shared Step 06 PriorityCapacity already preserves
waiting order, prefetch capacity, cancellation and streaming permit lifetime.
Puppeteer forwards downstream responses without an independent limiter. Replacing
that scheduler with immediate rejection would change its contract.

`pezzottify-downloader/docs/step-10-rate-limiting.md` records the assessment.
Baseline Cargo tests compiled against an immutable shared archive: **119 passed,
11 failed because the sandbox denied Unix/TCP socket binds**. The escalated retry
did not complete. Python tests could not start because `pytest` is unavailable.
These are validation limitations, not a passing full suite. The result changes
only documentation, with a clean diff check; executable code, dependencies and
active source pins remain unchanged. Original `master` was rebased onto the
isolated assessment branch; ancestry and identical tree verified, original
checkout clean, owned worktree/branch and temporary build/archive removed.
No push/deployment.

### SCT — assessed N/A

`master` advanced from `7a0c6bd5ad77f6f3c12bc18aaf8afeb9b53c357f` to
`3b1144fe2ef5ca2f042b093fcad4f0f60ccb70a2`. The production server, configuration,
routes, core paths and offline recovery tool have no request-rate budget or
failure cooldown. Its two `RateLimited` sites enforce PostgreSQL row cardinality:
10,000 global live OIDC flows and 10,000 saved cursors per principal. Their 429
and one-second Retry-After mapping do not make them elapsed-time request quotas.
Expiry, consumption and persisted state remain database-owned. Upload storage
reservations, payload-I/O waiting capacity, workers and archive record limits
remain storage/task policies; adding a request limiter would introduce behavior.

`sct/docs/step-10-rate-limiting.md` records the
production review. Baseline/final diff checks pass; no executable tests were
needed for a documentation-only assessment. No dependencies, active pins or
runtime configuration changed. Original `master` was rebased onto the isolated
assessment branch, ancestry and identical tree verified. All 86 existing local
commits were preserved, along with untracked `.validation-work/` and
`docs/step-04c-cors.md`. Owned worktree/branch removed; no push/deployment.

### Third batch completion

All four assessments were committed on isolated branches and integrated by
rebasing their original development branches onto those branches. Simple Agents
uses `main`; the other three use `master`. Owned worktrees, branches, test targets
and archives were removed; pre-existing work and worktree references remain.
Both central trackers were validated and committed on `simple-server` `main`.
This batch changes documentation only: no shared API, executable behavior,
consumer dependency feature or active build pin changed. Third-batch checkpoint:
**4 Done, 5 Partial, 4 N/A, 4 Pending**. Downloader's durable Python sliding quota
is explicitly Pending; Androidoscopy, Paranza and Quentin Torrentino await review.
No push or deployment. Step 07 remains deferred until after rate limiting and
verified consumer Axum removal.

## Step 10: fourth parallel rollout — 2026-09-24

Four GPT-6-sol assignments cover the four previously Pending services:
Androidoscopy, Paranza, Quentin Torrentino and Pezzottify Downloader. Three workers
run alongside the coordinator; Quentin started when Androidoscopy completed.
The coordinator owns both trackers and any shared API changes. Reviewed shared
revision: `b8a53f877f37eb762950aa4f85e0b9a914ea891d`.

### Androidoscopy — assessed Pending, Kotlin enforcement

Clean `master` advanced from `50b1cda3cb45140d53f9c5db9d3b3a4f0883edff` to
`2c6f9a8c7fb05e4c87db8c1fe35516bcc0d22ca7`. The Android SDK's production
`SessionRuntime.serve` applies a five-second PAIR attempt limit using Kotlin
`SystemClock.elapsedRealtime()`. The admitted timestamp is global to the runtime,
updated before commitment validation, unchanged on denial and retained across
start/stop. RESUME follows a separate credential path. Denials close the socket
with `PAIRING_RATE_LIMITED` and retain the existing UI reason. This is a real
quota at the device enforcement boundary, not an optional desktop check.

The Rust controller/legacy server/MCP bridge has no request quota; its Rust-only
scope is N/A. Adding a desktop limiter would not cover other LAN callers.
Kotlin currently has no binding to the shared Rust library, so whole-consumer
adoption remains **Pending** for a separately chosen language boundary. Existing
socket/tool capacity and protocol pacing are unrelated. No dependency feature,
active pin or runtime behavior changed.

The local `androidoscopy/docs/step-10-rate-limiting.md` records source/config
inspection and baseline/final diff checks. Runtime/device suites were not run for
this documentation-only change. Original `master` was rebased onto the isolated
assessment branch; ancestry and identical tree verified, original clean status
retained, and owned worktree/branch removed. No push/deployment.

### Paranza — assessed N/A

Clean `master` advanced from `5a28680dff3cdb2ec31bfa43c812ea9f74b2fabb` to
`4c254ff10b561cd4b904df4a219ce6df279f7fe3`. Production server and Linux/Android
runners have no request rate or concurrency admission policy. PCM submit
performs authentication and target/link/priority/payload/TTL validation before
SQLite enqueue. There are no quota settings or live quota-exceeded/429/Retry-After
paths. Polling, heartbeat, process restart and delivery pacing are scheduling or
lifecycle behavior. The PCM plan mentions future sender quotas with undecided
defaults: that is future product work, not an existing migration gap.

`paranza/docs/step-10-rate-limiting.md` records inspected paths and the decisions
needed before introducing a sender quota. Baseline/final diff checks pass; no
Rust/Android/Docker suite was needed for this documentation-only change. Active
features, pins and behavior are unchanged. Original `master` was rebased onto
the isolated assessment branch; ancestry/tree equality verified, checkout clean,
and owned worktree/branch removed. No push/deployment.

### Pezzottify Downloader — Pending, concrete language-boundary plan

Clean `master` advanced from `66c68948fd00aff6d06860889a182cc1a3dfa56a` to
`1db3b72f5f5482a34841331976182b1eae22f534`. The reassessment records a concrete
shared policy boundary: rolling-window durations, strict cutoffs, per-window
remaining counts and minimum budget must be owned by a shared policy calling
an application SQLite count callback. Python would retain its exclusive run lock,
check placement, run status and success-only upload accounting. Merely wrapping
`min(limit - count)` is not adoption of the actual quota policy.

A CLI subcommand on the existing downloader binary is the narrowest proposed
bridge; it still needs packaging/version compatibility, timestamp/SQL equivalence,
structured results, failure handling and tests at both cron check sites. An HTTP
bridge would change startup order; a native Python extension creates a separate
build/distribution path. None was introduced without a language-boundary choice.
Existing production quotas remain **Pending**, with Rust HTTP budgets N/A and
priority admission already shared under Step 06. No executable, feature or active
source-pin changes were made.

Cron and test-module syntax checks pass. Focused standard-library SQLite probes
pass no-limit fallback, empty history, strict 30-minute cutoff, failed-attempt
exclusion and the minimum across three windows. `pytest` remains unavailable;
the complete Python suite was not run. Earlier Rust socket-test limitations remain
in the local record; that unchanged Rust code was not retested. Documentation diff
checks, integration ancestry and tree equality pass. Original `master` was rebased
onto the isolated branch, original checkout remains clean, and owned worktree,
branch and probe artifacts were removed. No push/deployment.

### Quentin Torrentino — Done, MusicBrainz pacing

Clean `master` advanced from `191a89ca00ce17d4742d123718ea0a562a65a075` to
`2e0489445fa2bf7987ae5d93e3ff4f675b6866ee`. Production MusicBrainz search and
get now share a strict burst-one replenishing `Budget`. The existing Tokio mutex
continues to serialize admissions and stays held during waits; the budget is
rechecked after waking. The first admission is immediate, cancellation does not
consume quota, and zero configured delay explicitly remains unlimited. Default
spacing remains 1100 ms. Application code retains transport, waiting and error
mapping. The separate searcher token bucket has no production callers and was
left unchanged; upstream 429 handling is not a local quota.

The `rate-limit` feature and `simple-server.rev` pin adopt shared revision
`b8a53f877f37eb762950aa4f85e0b9a914ea891d`. The lockfile gains only the two
required shared dependency entries. Before migration, 16 existing external-catalog
tests and all five MusicBrainz tests passed. The latter cover paused-clock
spacing, concurrent admissions, idle behavior, cancellation, zero delay and
loopback HTTP search/get/error behavior against the original implementation.
After migration, all five pass; the locked core run reports **508/509 unit tests
and 11/11 pipeline lifecycle integration tests passed**, with 12 ignored doctests.
The sole failure, `content::tests::test_post_process_dispatches_to_video`, was
reproduced at the same assertion on untouched starting commit `191a89c`.

Core all-target Clippy exits successfully with 11 existing warnings. Changed-file
Rustfmt and diff checks pass. Workspace formatting checks expose pre-existing
formatting debt in untouched files; no bulk formatting was applied. The full
workspace test suite was not run. Local evidence is in
`quentin-torrentino/docs/step10-rate-limit-migration.md`. Original `master` was
rebased onto the isolated migration branch, ancestry/tree equality verified and
checkout clean. Owned worktree, branch, frozen shared snapshot and build artifacts
were removed. No push/deployment.

### Fourth-batch completion

All four original development branches were integrated locally and owned temporary
worktrees and branches removed. Both central trackers were updated in their own
isolated worktree. No shared runtime changes were required in this batch.
Fourth-batch checkpoint: **5 Done, 5 Partial, 5 N/A, 2 Pending**. Every service has
been assessed; the two Pending cross-language quotas need a language-boundary
choice, and the five Partial services retain the gaps recorded above. Step 10 is
not yet complete. No push or deployment; Step 07 remains deferred until after
rate limiting and verified consumer Axum removal.

## Step 10: remaining cross-language quotas — 2026-09-24

### Shared rolling-window policy

The isolated `feat/rate-cross-language` worktree starts from clean `main` at
`66b5259b22c6c48687f822beca497f03ad12c2f7`. The shared rate-limit module now
provides `evaluate_rolling_windows`, `RollingWindowStore` and typed window/usage
results. This read-only policy owns signed-microsecond cutoff calculation,
per-window clock samples, strict-after count requests, saturating subtraction
and minimum/fallback budgets. The application retains transactions and successful
event accounting; errors propagate and evaluation does not reserve capacity.
No database or language-runtime dependency was added.

Baseline: all 28 existing rate-limit policy tests passed. After the extension,
33 policy tests pass without default/HTTP features, including five new rolling
window tests. The full all-feature suite passes **184 tests/doctests**. New tests
cover strict boundaries, failed-event exclusion, future timestamps, negative
epochs, per-window time sampling, zero/no limits, complete diagnostics after
exhaustion, clock/storage failures, arithmetic range and repeated history-model
comparisons. All-feature/all-target strict Clippy, formatting and diff checks
pass. Consumer integration and final cleanup evidence follow below.

### Pezzottify Downloader — Done, Python/SQLite bridge

Clean `master` advanced from `1db3b72f5f5482a34841331976182b1eae22f534` to
`463f811c8c0def6ba12955b83b2878d2bcf8294b`. Both production cron quota checks
now invoke the configured downloader binary's `quota` subcommand, which adopts
shared revision `36b57d3bc02f10f84d37dbd2cb4a4083d79706ad`. The versioned
stdio protocol delegates time/count callbacks to Python's existing SQLite
connection while Rust computes window cutoffs, remaining counts, minimum and
no-window fallback. The same connection preserves uncommitted-event visibility;
strict ISO-formatted cutoffs, fresh time per window, successful-only accounting,
run locking and check placement are retained.

The helper starts no server/authentication/runtime. Each exchange has a ten-second
deadline, bounded messages and strict sequence/schema checks; failures stop the
run and the child is reaped. Supported null keys and unknown keys remain ignored;
zero/negative integer limits preserve zero remaining. Limits now explicitly
require signed 64-bit integers. Cron and the quota-capable binary must be updated
together, including when `manage_downloader_process` is false. The local README
and `docs/step-10-rate-limiting.md` explain this build/runtime requirement.

Baseline Python syntax checks passed; system pytest was absent, and a pre-edit
Rust suite was not run. Final verification uses an isolated cached pytest install:
**100 Python tests pass** against the real helper, including strict cutoffs,
uncommitted rows, completed/failed accounting, no/zero/negative/null/unknown limits,
database errors, missing/malformed/slow helpers and pre-authentication failure.
The Rust binary builds; **169 Rust tests pass, one ignored**. Normal Clippy passes
with 11 pre-existing warnings in unchanged library code; strict Clippy fails on
those warnings. Diff checks pass. Original `master` was rebased onto the isolated
migration branch; tested hash/tree equality and clean checkout verified. Temporary
worktree and branch were removed. No push/deployment.

### Androidoscopy — Done, device-side JNI bridge

Clean `master` advanced from `2c6f9a8c7fb05e4c87db8c1fe35516bcc0d22ca7` to
`ca813fe2e338b4171f87d646831414965c423ed8`. The actual Kotlin `SessionRuntime.serve` PAIR gate
now invokes a small JNI bridge using shared `Budget` and a strict single-unit
five-second quota. Shared revision `66b5259b22c6c48687f822beca497f03ad12c2f7`
was frozen during implementation and is recorded in `simple-server.rev`.
The bridge crate enables only `rate-limit` without default features. It retains
no native handle: Kotlin owns the last admitted elapsedRealtime timestamp and
Rust reconstructs the budget for each check. Denials leave that timestamp
unchanged. The gate remains per runtime, survives start/stop and runs before
commitment validation; RESUME and existing error/socket/UI behavior are retained.

Gradle builds and packages the native library for armeabi-v7a, arm64-v8a, x86 and
x86_64. JNI shrinker rules preserve symbol lookup. SDK CI, JitPack and README
instructions include the pinned shared source, Rust targets and NDK 27.0.12077973;
SDK users receive the native libraries in the AAR. No runtime network bridge or
new quota on the desktop controller/legacy server was introduced.

Baseline SDK unit tests passed. Final checks: **118 SDK unit tests passed**,
including three loading the actual host JNI library, and **two Rust bridge tests
passed**. Debug/release SDK AAR, demo debug APK and Android instrumentation APK
builds passed. Coordinator independently verified all four ABI libraries in the
release AAR and demo APK, 16 KiB ELF PT_LOAD alignment and uncompressed APK ZIP
data offsets. Instrumented tests compiled but were **not run: no device was
attached**. Full TLS pairing/restart E2E was not executed; gate placement,
lifecycle retention and RESUME bypass were verified in source. Bridge formatting,
Clippy and diff checks pass.
Local details are in `androidoscopy/docs/step-10-rate-limiting.md`.

Original Androidoscopy `master` was rebased onto the migration branch, verified
against the tested tree and left clean. The owned temporary branch, worktree,
frozen shared snapshot and build artifacts were removed. No push/deployment.

### Final integration and cleanup

Downloader follow-up `3e04e15719671b9fbe40d8d7016f87e38a5318ec` rejects
non-object `rate_limits` before spawning the helper, preserving fail-closed
behavior for malformed list/string/null configuration. Its focused regression
cases pass (three passed, 100 deselected), along with Python syntax and diff
checks; the broader suites were not repeated for this Python-only type guard. Both consumer base branches were
rebased onto their dedicated migration branches, original checkouts are clean,
and owned temporary branches/worktrees, snapshots, builds and Python test tools
were removed. Shared `main` was rebased onto the tested library worktree branch;
the final tracker commit also strengthens the history-model test to verify
both exhaustion and replenishment as events age out (five rolling tests and
strict targeted Clippy pass). Tracker script/rendering, row counts, local links
and Markdown/HTML status parity are verified before integration and cleanup.

Cross-language checkpoint: **7 Done, 5 Partial, 5 N/A, 0 Pending**. The formerly
Pending cross-language consumers are now adopted; remaining work belongs to the
five explicitly Partial services. No push or deployment. Step 07 remains deferred
until after rate limiting and verified consumer Axum removal.

## Step 10: final five partial consumers — 2026-09-24

Five dedicated GPT-6-sol assignments completed Pezzottify, Crumbles, Peerlo, Lello
Auth and Pezzottflix, running at most three workers alongside the coordinator.
The coordinator owned shared API changes and both trackers. All five remaining
Partial scopes are adopted and integrated: **12 Done, 0 Partial, 5 N/A, 0 Pending**
across all 17 services. This supersedes earlier Step 10 rollout checkpoints.

The shared worktree starts from clean `main` at
`73d841398523423dc77154f7a8bac6140e6a77ef`. Baseline policy suites pass 33 tests.
`RollingWindow::cutoff_at` and `remaining` now expose the existing policy in two
phases, allowing asynchronous SQL queries inside caller-owned transactions
without blocking an async runtime or duplicating cutoff/remaining arithmetic.
The existing evaluator reuses those methods. All 34 policy tests pass with
only `rate-limit` enabled, including split-phase parity and range/error cases.
Further shared extensions and final consumer evidence follow below.

Shared anchored/grouped window counters, persisted calendar counters/gap gates,
and ordered signed/projected snapshot checks add eight contract/model tests.
All 42 policy tests pass without default/HTTP features, covering strict/exact
boundaries, idle ticks, separate attempt charging, zero limits, backward samples,
category resets, persisted snapshots, date rollover and error precedence.
Consumer source snapshots used the committed extension revision.

The persisted signed-epoch `PollingGate` and opt-in Tokio
`DelayedReleaseLimiter` complete the remaining shared API requirements. The base
`rate-limit` feature stays runtime-independent; `rate-limit-async` explicitly
adds FIFO waiting and one release timer per admission. Four paused-clock tests
verify bursts, delayed replenishment, cancellation, FIFO and closed/zero capacity;
polling models cover persisted state, signed intervals and backward time. The full
all-feature shared suite passes **198 tests/doctests**; all-feature/all-target
strict Clippy, formatting and diff checks pass.

### Crumbles — Done, transactional dispatcher quotas

Clean `master` advanced from `dea9e042042ceae6b0e62b25a7d9e576699a99bf` to
`25a7ef1953ec2b7c5b2195322505af9a288eedd3`, pinned to shared
`340e17ca50d1c45b264e1ef3967c617fb04ff5d6`. Global and per-policy dispatcher
quotas use shared rolling cutoff/remaining calculations in both preview and
reservation paths. Strict SQL boundaries, all-status reservation accounting,
legacy COUNT narrowing and the existing SQLite write transaction remain intact;
outstanding concurrency capacity is a separate application policy.

Baseline admission tests: six passed; two new contract cases passed before the
adapter change. Final admission tests: nine passed; full core suite: **675 passed**.
Tests cover strict cutoffs/future rows, settled reservations, per-policy/global
preview and concurrent reservation of a single rate slot. Production package
check passed after providing the original ignored web dist in the isolated
worktree (the initial missing-dist error was a fixture issue). Strict core
Clippy, formatting and diff checks pass. Original master was rebased onto the
tested branch, tree/ancestry verified and clean; owned branch/worktrees, snapshot,
dist and build artifacts were removed. No push or deployment.

### Peerlo — Done, crawler and persisted bootstrap

Clean `master` advanced from `861552bddca4a2da303036d532ac81cf39ef6274` to
`f4fa36767a5133109f2edc3b11ef2bc6654ac36d`, pinned to shared
`799e94b47ddab1c04ce908c2afa6ea1c28e6e5d4`. The BEP51 crawler uses shared
`WindowCounters<1>` with a loop-initialized anchor, >=60-second reset on every
tick, non-consuming preflight, zero-limit denial and recording after actual
query attempts. RoutingTable owns `CalendarGate<String>` for the persisted
600-second gap followed by the 24-per-date quota. Snapshot JSON fields and
missing-field compatibility are preserved; calendar formatting stays local.
A pathological persisted `u32::MAX` counter now saturates rather than wrapping
or panicking; ordinary admission behavior and check/record separation remain.

Baseline: 282 DHT library and 82 binary tests passed, two ignored. Final full
workspace: **796 passed, six ignored**. Oracle/snapshot tests cover exact minute
and gap boundaries, idle ticks, attempted/denied/no-candidate accounting,
backward clocks/date changes, quota exhaustion and snapshot restoration.
Formatting, diff checks and all-target Clippy pass with existing warnings.
Original master was rebased onto the tested branch, exact hash/tree/ancestry
verified and clean. Owned worktree/branch, frozen shared worktree, archive and
build target were removed; unrelated worktrees were preserved. No push/deployment.


### Pezzottify — Done, grouped MCP, durable quotas and enrichment pacing

Clean `dev` advanced from `e428bd67a15ce91e3fd89a56b31ac207ea45eab6` to
`4a7e9ee190820352ba278fa663da1f11666432ef`, pinned to shared
`799e94b47ddab1c04ce908c2afa6ea1c28e6e5d4`. MCP uses shared grouped counters
with the existing strict-after-60-second boundary, shared category anchor,
zero-limit behavior and rounded Retry-After. Download admission uses ordered
signed snapshot checks; local-day SQL and the separate enqueue/count sequence
remain application-owned. Report quotas use shared rolling and projected checks
inside the existing immediate transaction, preserving strict cutoffs, future-row
accounting, replay precedence and metadata/attachment capacity reservations.
MusicBrainz and LastFM use a shared single-slot budget under the existing mutex;
first admission is immediate, actual post-wake admission sets the next deadline,
and failed HTTP requests still consume admission. The OIDC JWKS refresh cooldown
remains an auth cache recovery policy, outside this quota rollout.

Baseline MCP/report/download suites: 5/11/189 passed; three new MCP oracle cases
also passed against the original code. Final focused suites: 9/13/190 passed.
Full suite: **1,440 passed, 36 existing ignored**. After the final deterministic
pacing test seam, both pacing tests and all-target Clippy passed; existing lint
warnings remain. Formatting and diff checks pass. Docker/release builds were not
rerun. Original dev was rebased onto the tested branch, exact tree/ancestry
verified and clean. Owned worktree, branch, frozen snapshot and build target were
removed; pre-existing Paravoid worktrees were preserved. No push or deployment.


### Pezzottflix — Done, durable daily quota and delayed-release pacing

Clean `master` advanced from `06824916af4ef8c9ae7f6da6539c507659c9d151` to
`b507fe91fb2ffb683bdf0d0cacb8538a397698d2`, pinned to shared
`c07bb5f5d3ccb4da75bb33d5568781df22857d63`. TMDB uses the optional shared
`DelayedReleaseLimiter`: 45 immediate slots, each returned after a 22 ms timer,
independent of HTTP completion. This preserves the existing pacing; it does not
claim a sustained 45 requests/second ceiling. Throughput logging remains local.
SQLite download counts are passed to `CalendarCounter`, retaining the UTC date,
legacy signed-to-unsigned cast and separate check/increment/request-insert order.
A failed request insert still consumes quota; the existing non-atomic sequence
is unchanged. Unregistered Governor middleware is not a production quota.

Before edits, the two existing TMDB pacing tests and daily quota test passed
against the frozen shared snapshot. Final full server suite: **501 library tests
passed, one ignored**, all integration groups passed, two doctests ignored.
Cases cover actual HTTP 500 charging, persisted quota after recreation, exact
exhaustion, other dates, zero limits, negative stored counters and failed inserts.
The focused HTTP test passed again after isolating its fixture from the periodic
logger. All-target Clippy passed with existing warnings; targeted Rustfmt and
diff checks passed. Repository-wide Rustfmt retains unrelated existing differences.
Original master was rebased onto the migration branch, exact tree/ancestry
verified and clean; owned worktree, branch, shared snapshot and build artifacts
were removed. No push or deployment.


### Lello Auth — Done, persisted device-code polling

`master` advanced from `1533adb24dc0d1a2d33e730eb600064919e32937` to
`dee2d2bac70d46c235a0ba5fd12155cf95b8f107`, pinned to shared
`c07bb5f5d3ccb4da75bb33d5568781df22857d63` in active CI and build instructions.
`PollingGate` reconstructs stored interval/timestamp state, admits with the existing
signed-second clock and supplies the timestamp for the existing repository write.
Lookup and expiry checks still precede clock sampling; denied polls do not mutate
stored state. The service retains interval validation, status precedence, database
errors and OAuth `slow_down` mapping, without automatic interval escalation.

Baseline focused device-code suite: 30 passed; final: **34 passed**, plus one
production HTTP token regression test. Cases cover exact boundaries, backward
time, reconstructed state, denied timestamp stability, missing/expired/status
precedence, invalid persisted intervals and a failed database write. Rust **1.88.0**
workspace formatting and strict all-target Clippy pass. With loopback access, the
full all-target run passed **191 axum unit tests** and all completed HTTP integration
suites, then core passed **559/560**: the untouched
`token_manager::tests::test_concurrent_refresh_creates_exactly_one_successor`
failed with `grace-period retry failed`. That test passed on an isolated rerun;
this suggests timing sensitivity but is not a baseline-confirmed failure. The
full workspace gate is therefore not claimed green or complete. Optional PostgreSQL
fixtures were not exercised. The initial sandbox HTTP failure was a loopback
permission issue.

Original master was rebased onto the migration branch and the exact tested tree
and ancestry verified. Owned worktree, branch, frozen snapshot and build target
were removed; an unrelated pre-existing prunable worktree was preserved. Three pre-existing
untracked `docs/IDENTITY_PROVIDER_*` research files were preserved, so the original
checkout is not wholly clean. No push or deployment.


### Final integration checkpoint

All five consumer commits above are on their active development branches. The
coordinator verified the original checkouts and worktree registrations; only
pre-existing unrelated untracked files/worktrees remain. Shared API commits
`340e17c`, `799e94b` and `c07bb5f` are integrated on `main`. Both status matrices
agree on **12 Done, 0 Partial, 5 N/A, 0 Pending**. HTML script rendering, table
dimensions, local links, Markdown parity and summary counts were checked.
Final tracker and roadmap changes add no runtime behavior; the shared 198-test
suite and strict Clippy results remain applicable.

Next is completing the public HTTP abstraction and removing consumer Axum usage
and the transitional re-export, including tests. Step 07 database helpers remains
deferred until after that milestone. No agent or coordinator push or deployment
was performed for this work.


## AuthLayer cookie/header selection — 2026-09-25

Shared implementation adds ordered header/cookie credential sources, explicit
malformed-input fallback, required/optional authentication and verified identity
source metadata. The existing Parts-based constructor remains available. The
strict credential cookie parser adds no dependency; feature-only builds remain
Axum/Tokio independent. This paragraph records the initial library-only checkpoint.
Pezzottify integration subsequently completed; see the
[follow-up evidence](#pezzottify-cookie-auth-integration--2026-09-25). The subsequent
[session extraction migration](#pezzottify-session-extraction--2026-09-25) also
removes its Axum Session bridges.
See the [contract](step-08-auth.md#cookie-and-header-credentials--2026-09-25).

Validation: baseline auth suite 7/7; final minimal-feature auth suites 13/13.
The full all-feature suite passes **206 tests/doctests**, including real HTTP
cookie/header admission, duplicate cookies and body preservation. All-feature,
all-target strict Clippy, formatting and diff checks pass. The normal dependency
graph with only `auth` contains HTTP/Tower primitives and no Axum/Tokio. The
initial sandbox run could not bind loopback; the full suite passed with socket
access. HTML rendering, observation parity and links were checked. These are the
initial library checkpoint results; the follow-up below records consumer adoption
and the expanded shared suite. No deployment was performed.


## Pezzottify cookie auth integration — 2026-09-25

Clean `dev` advanced from `4a7e9ee190820352ba278fa663da1f11666432ef` to
`5a1db7c95d4a0c8dddfe3c3c4d91b7881ab63d06`. Its active CI/build pin is shared
`d7e8d133bfc67b62b93d2679fe27610de0dfda99`, already integrated on simple-server
`main`. `AuthLayer::credentials` now owns header/cookie selection and calls the
existing Pezzottify verifier through `authenticate` at the current lazy Session
extraction points. Source metadata is retained as `Identity<Session>` and fresh
validation runs each time; public requests do not gain global auth side effects.

Authorization retains priority, duplicate/invalid-text rejection without cookie
fallback, token68 grammar, extra Bearer spaces and the legacy-raw setting.
Application validation still tries OIDC then database sessions. Required sessions
retain 401 behavior; optional sessions retain anonymous invalid-credential behavior;
database failures still use the existing error responses. Shared Optional mode
itself remains strict about supplied invalid credentials; the consumer explicitly
maps errors at its existing extraction boundary.

The optional `auth-cookies` feature supplies decoded cookie compatibility and
framework-independent Cookie/SameSite values. Percent decoding, empty values,
ignored malformed pairs and last-duplicate selection match the original cookie
jar. Session/CSRF reads and cookie issuance/expiration no longer require
`axum-extra`; that dependency is removed from source, manifest and lockfile.
Cookie attributes, CSRF checks/exemptions and login/logout response contracts
remain application-owned and unchanged. The Axum Session/Option<Session> bridges
and error response traits remain until the public handler API exists; this is
not complete consumer Axum removal.

Baseline unchanged consumer code against shared `9ff4476`: 32 session-focused
tests and auth/MCP/permission HTTP suites 22/5/22 passed. Two added HTTP regressions
passed before migration, covering encoded cookies, duplicate order, whitespace,
empty cookies, extra Bearer spaces and optional anonymous sessions. Final full
Rust suite with `fast` fixture features: **1,443 passed, 36 existing ignored**.
This includes auth 22, MCP 5, permissions 22, CSRF, login/logout, executor error
mapping, streaming, lifecycle and WebSocket checks. Production-target strict
Clippy passed; all-target Clippy passed with warnings in unchanged enrichment
and background-task tests plus the existing num-bigint-dig compatibility notice.
Formatting and diff checks passed. Docker/browser/Android and external OIDC
provider deployments were not run.

Shared extension: **208 tests/doctests** and all-target strict Clippy passed;
minimal auth tests also pass and the base auth dependency graph remains HTTP/Tower
only. The new decoded mode is opt-in; strict cookie parsing remains the default.
Its CookieValue can own decoded bytes without changing HeaderCredential's borrowed
Credential interface. Lazy authenticate and Tower serving use the same gate.

The original dev branch was rebased onto the migration branch and exact tree and
ancestry verified. Its owned migration worktree/branch were removed; pre-existing
Paravoid worktrees were preserved. At this checkpoint, the observation columns
distinguished completed cookie authentication from remaining Axum bridges; the
subsequent extraction migration below removes those bridges. No push/deployment.


## Pezzottify session extraction — 2026-09-25

Shared source **`ce37b3dc80e2c7bd334898e79c0f2e6eceacf38c`** introduces the opt-in
`extract` module: `FromRequestParts<S>`, `Extract<T>`, `IntoRejectionResponse` and
buffered `RejectionResponse`. Application contracts use standard HTTP parts,
borrowed application state and a Send future. Only the internal adapter depends
on Axum. With default features disabled, `extract` depends only on `http`, `bytes`
and `itoa`. See the [contract](request-extraction.md).

Pezzottify `dev` is integrated at **`c27e1bdc`**, from clean **`5a1db7c9`**,
and its active checkout/CI pin is `ce37b3d`. Both required and optional Session
implementations now implement the shared trait. All session handler arguments,
permission and rate-limit identity middleware, MCP/sync WebSocket upgrades and
direct extraction in report admission use this path. `session.rs`, including
its tests, contains no Axum import or trait implementation.

Existing behavior remains: fresh validation at each extraction, no identity
cache, OIDC-first legacy fallback, credential priority, missing/invalid required
session → 401, missing/invalid optional session → anonymous, database failure →
existing 503/500. CSRF, permissions, public routes and transport revalidation
remain application-owned. ApiError renders one buffered response for extraction
and its existing transitional response adapter, preserving JSON bytes, content
type, request ID, Retry-After and opaque internal errors.

Verification:

- Baseline consumer `5a1db7c9` against shared `2c63c61` (documentation descendant
  of the old pin): **32** session-focused library tests; real HTTP auth **22**,
  MCP **5**, permissions **22**, all passed before consumer edits.
- Final full consumer `cargo test --offline --locked --features fast`:
  **1,443 passed, 36 existing ignored**, no failures. This includes required and
  optional cookie/header admission, CSRF, permission concealment, revocation,
  streaming, WebSockets, signals/restarts and mixed workloads.
- Separately after that run, the ApiError suite passed **7** tests, including one
  new differential test comparing the previous JSON renderer's status, complete
  headers and exact body bytes for executor failures and escaped/Unicode errors.
  Subsequent source cleanup preserved existing handler-fragment formatting.
- Strict production-target Clippy passed. All-target Clippy completed with
  existing warnings in unchanged enrichment/background-task tests and the
  existing num-bigint-dig future-compatibility notice. Changed standalone Rust
  modules pass formatting; existing included-handler formatting is preserved.
  Both repositories pass diff checks.
- Shared all-feature suite **211 tests/doctests**, strict all-target Clippy,
  formatting and standalone extraction checks passed. New contracts cover
  borrowed state across await, optional error policy, repeated extraction,
  request-head mutations, body preservation and exact rejection metadata/bytes.
- Builds use private targets, two jobs and disabled dev debug info. Runtime tests
  use local loopback fixtures. Docker/browser/Android and deployed OIDC providers
  were not exercised.

Both base branches were rebased onto their dedicated migration branches, exact
integration trees/ancestry verified, and owned worktrees, branches, build targets
and logs removed. Unrelated Pezzottify Paravoid worktrees are preserved. Both
observation columns list only remaining exposure. No push or deployment.

This checkpoint completed the custom Session extraction slice, not full Axum
removal. The HTTP-core canary below subsequently adds ordinary routing, built-in
extraction and responses for the embedding API. Other route groups, general
middleware and streaming remain outstanding. Step/module totals are unchanged.


## Pezzottify HTTP core canary — 2026-09-25

Shared implementation **`64f31b41f36617f0269d14248f284c34c7dffe17`** adds opt-in
`web`: owned Router/MethodRouter and Handler contracts, state/substate, path/query,
JSON/bytes/string/raw-request extraction, response conversion, private body
representation and Tower serving. With lifecycle enabled, the shared router serves
HTTP with explicit graceful shutdown. Existing shared body limits can be applied
without backend types. See the [contract and remaining scope](web-core.md).

Pezzottify `dev` is integrated at **`bf825a5d`** from clean `c27e1bdc`; its
active checkout/CI pin is `64f31b4`. Five embedding handlers and both route
constructors now use shared APIs. The embedding source has no Axum imports or
public type/trait exposure. An ApiError implementation delegates to the existing
single buffered renderer. Two opt-in `web-compat` conversions remain only at
legacy route assembly, preserving state, permissions, rate limits, CSRF and route
placement. No other services were changed.

Verification:

- Baseline consumer `c27e1bdc` with shared `9e1c067`: auth **22**, permissions **22**,
  route composition **3**, all passed. Three new real HTTP embedding contract
  tests were committed first as `f85af6a3` and passed against the original handlers.
- The same **50** targeted tests pass after migration. New coverage exercises
  CRUD/search, percent-encoded namespace, optional vector query, response data,
  204/404/error IDs, anonymous/permission/CSRF rejection, media type/JSON/query
  errors, body limits and application validation.
- Final full consumer suite: **1,447 passed, 36 existing ignored**, no failures. Production strict Clippy passed;
  all-target Clippy passed with existing warnings in unchanged enrichment and
  background-task tests plus the num-bigint-dig compatibility notice. Changed-file
  formatting and both repositories’ diff checks passed. Tests use private targets,
  two jobs, disabled dev debug information, offline locked dependencies and the
  `fast` fixture feature. Docker/browser/Android and deployed OIDC providers were
  not exercised.
- Shared full suite: **222 tests/doctests passed**, including nine new HTTP-core
  tests and two compile-fail body-ordering checks. Strict all-target Clippy,
  formatting, standalone `web` and standalone `extract` checks passed.
  Differential tests preserve nested routing, HEAD/405/Allow, static route
  priority, JSON/path/query error bytes, limits and response metadata. Tests also
  cover ordered custom extraction, early rejection without body reads, explicit
  rejection capture, substate, sixteen arguments and real HTTP serving/shutdown.

Both base branches were rebased onto their worktree branches and exact trees and
ancestry verified. Owned worktrees, branches, build targets and logs were removed;
unrelated Pezzottify worktrees remain. Both tracker observations list outstanding
scope only. No push or deployment. Existing numbered-step totals are unchanged:
this is a partial rollout of the HTTP abstraction, not full Axum removal.

## Pezzottify routing completion — 2026-09-25

- Applicability: every production route group still used backend routing after
  the embedding canary. Shared extensions preserve existing middleware ordering,
  application state, permissions, cookies/CSRF, admission and rate-limit keys.
- Shared source: `d3b559ad7a3597a29c4a5be77c82531f2e1e8c25`. Adds router/route
  Tower layers accepting standard response bodies, shared middleware/extraction,
  Extension/MatchedPath/ConnectInfo, static fallback services, lazy body streams,
  and lifecycle serving with direct TCP peer metadata. Optional protocol adapters
  explicitly retain the remaining multipart/WebSocket/SSE/observer contracts.
- Consumer: `dev` started at `bf825a5d`; baseline test commit `370f25c4`;
  integration `fd6585e2`. `simple-server.rev` records the reviewed shared source.
- Adoption: all route groups and ordinary handlers, substate extraction,
  response/error contracts, custom range extraction, middleware composition,
  main/metrics serving and common HTTP test fixture use shared APIs. No
  `into_axum_router` conversion or application `FromRef` implementation remains.
- Before: **1,447 passed, 36 ignored**. Two additional real HTTP tests passed
  against the old router before production edits: multipart auth/parser/field
  errors and HEAD/405/Allow/404 behavior.
- After: complete `cargo test --offline --locked --features fast` suite:
  **1,449 passed, 36 existing ignored**. Includes authentication, permissions,
  route contracts, reports, body limits, tracing, rate limits, ingestion, audio
  ranges, search/SSE, MCP and sync WebSockets. Strict production Clippy passes
  with `fast,slowdown`; formatting and diff checks pass. The existing
  num-bigint-dig future-compatibility notice remains.
- Shared: **231 tests/doctests** and strict all-target/all-feature Clippy pass.
  Nine new composition tests cover ordering, rejection, metadata, readiness,
  standard service/body integration, lazy cancellation, trailers/errors and real
  peer-aware HTTP/shutdown. Minimal web: **14 tests**; extract-only: **1 test**.
- Remaining Axum exposure: multipart fields/errors, SSE producers, MCP/sync
  WebSocket sockets/messages, tracing observer response and independent mock or
  differential-test fixtures. Audio body/range handling and ordinary body-reading
  middleware are now shared, so they are removed from remaining observations.
- Scope limits: no Docker, browser, Android or external OIDC-provider runs; no
  deployment or push. Step 11 is Done; full Axum removal remains incomplete.
- Integration: original `main` and `dev` rebased onto their dedicated migration
  branches; tested trees and ancestry verified. Owned worktrees, branches and
  `/tmp/pezzottify-routing` build/log files removed. Unrelated worktrees preserved.

## Androidoscopy routing completion — 2026-09-25

- Applicability: controller API, dashboard asset/SPA responses, legacy app and
  dashboard WebSocket routes, public router factory and HTTP/TLS entry points
  required shared routing. Active branch `master` started clean at `ca813fe`.
- Shared source `cdb9e6304811d7cb0ae8de8333d4975a00597f42`: standard HTTP request
  bodies and a shared Tower service factory support the existing TLS server
  directly. Header-array response tuples preserve login cookies, replacement
  semantics and invalid-header response contracts. No backend router conversion.
- Baseline test commit `1f19ad0`; consumer integration `5e9a396` on `master`.
  `simple-server.rev` and active README build instructions use the reviewed source.
- All production route groups, ordinary handlers, State/Path/Json, response
  adapters, auth middleware, asset fallback and HTTP serving/test helpers use
  shared APIs. Bearer/cookie fallback, Host/Origin policy, route placement, TLS
  certificates/settings, lifecycle shutdown and task ownership are preserved.
- Baseline **76 server tests**; two new real HTTP contract tests passed before
  migration; final **78 server tests**. Covers login cookie attributes, malformed
  JSON/media types, 405/Allow/HEAD, dashboard assets/SPA fallback and invalid
  upgrades, plus existing controller auth, TLS registration, WebSocket message
  flows, ownership drain/late-upgrade rejection, logging and UDP discovery.
- Full-stack crate: **12 passed before and after** with mock Android/dashboard
  clients. It has no tracked lockfile; an ignored local lockfile was generated
  offline. Pairing-rate-limit Rust bridge: **2 passed** after the shared pin update.
- Shared baseline **231**, final **234 tests/doctests**; strict all-feature,
  all-target Clippy and **3 minimal-web tests** pass. New differential tests check
  valid/invalid header arrays; service-factory test uses standard request bodies.
- Normal consumer all-target Clippy passes with existing warnings. Strict Clippy
  remains blocked by unchanged Default/dead-code/PathBuf/test-conversion debt.
  Changed Rust files pass formatting; unrelated protocol/session/TLS formatting
  remains untouched. Diff and dependency checks pass: Axum 0.8.9 via simple-server.
- Remaining exposure: backend WebSocket Message/WebSocket and upgrade adapter;
  axum-server TLS configuration/server/handle and its TLS fixture. Step 11 is Done;
  full Axum removal is not claimed. Android SDK/device and browser suites were
  not run; those sources are unchanged and committed dashboard assets were used.
- Both original development branches (`master`, shared `main`) rebased onto the
  dedicated migration branches; ancestry and tested trees verified. Owned
  worktrees, branches and `/tmp/androidoscopy-routing` targets/logs removed.
  No push or deployment; unrelated user work preserved.

## Crumbles routing completion — 2026-09-25

- Applicability: main API/metrics/static/WebSocket routes, MCP service mounting,
  and crumbles-integration control routes still exposed backend routing. Active
  `master` started clean at `25a7ef1`; integration commit **`23e47e0`**.
- Shared source **`347e06efcdef9423b481922dce929014db3f281d`** adds `nest_service`,
  `get_service`, method-specific fallbacks, Uri/optional Extension extraction and
  `Correlation::run_http` over arbitrary standard bodies. Existing backend
  correlation entry points remain compatible. Shared pin/README updated.
- Both production servers, ordinary handlers/extractors/responses, auth/CSRF
  middleware, metrics/peer metadata, static/SPA responses, health, HTTP fixtures
  and mocks now use shared APIs. MCP mounts its standard RMCP Tower service with
  unchanged auth, quotas, host/origin and body policy. No legacy router conversion.
- AuthError implements shared rejection rendering; ordinary errors and extractor
  errors use the same canonical buffered JSON payload. Required/optional session
  policy, database lookups, permissions and correlation IDs are preserved.
- Workspace baseline/final: **1,444 passed, two existing ignores**. Includes main
  server, core and integration suites, generated method/path route contracts,
  MCP auth/tools, multipart failure/rollback/limits, downloads, browser CSRF,
  login abuse/peer rates, metrics, headers/CORS, WebSockets and tracing.
- Strict `cargo clippy --offline --locked --workspace --all-targets -- -D warnings`,
  formatting and diff checks pass. Production-process correlation script passes
  **15 rejection cases**, safe HTTP tracing, query redaction and clean shutdown.
- Shared: baseline **234**, final **239 tests/doctests**, strict all-feature,
  all-target Clippy; **three minimal-web tests** pass. Five new regression tests
  cover nested service prefix/query/auth behavior, fallback HEAD/405 contracts,
  optional extensions, opaque-body correlation and shared health-service routing.
- Remaining exposure: multipart field/error types, realtime WebSocket socket and
  message types, and the explicit tracing compatibility adapter. Full Axum removal
  remains incomplete; Step 11 production routing is Done for both components.
- Builds used offline/locked dependencies, two jobs, isolated targets and disabled
  dev debug information. Existing local frontend build assets were copied into
  the worktree and used unchanged before/after. Browser/Android suites were not
  run. Dependency tree confirms Axum 0.8.9 via simple-server.
- Original `master` and shared `main` rebased onto dedicated migration branches;
  ancestry and tested trees verified. Owned worktrees/branches and
  `/tmp/crumbles-routing` build/log files removed. No push or deployment.


## Fausto routing completion — 2026-09-25

- Applicability: server route groups, plugin API and both blog/test-echo plugins
  still exposed backend routers/extractors/responses. Active branch `master`
  started clean at `2adbdcf`; shared `main` started clean at `9d8c7e1`.
- Implemented in sibling isolated worktrees, based on those branches. Shared
  source commit `5deb6debd98a1ec4912824656d857799a0088b16`; consumer `b106e99`.
  README and CI sibling-checkout references now select the tested shared source.
- Migrated ordinary routing/handlers, authentication extraction, validated JSON
  and canonical error rendering, rate-limit/correlation middleware, static files,
  health, streamed blob/federation responses, and peer-aware HTTP startup.
- Plugin API and both plugins return shared routers. API version advances to 2;
  the loader's existing version gate rejects old binaries before constructing
  their plugin instance. Dynamic plugins must be rebuilt with matching sources.
- Swagger uses its independent asset API through shared GET/HEAD handlers, with
  slash redirect, wildcard assets, OpenAPI and static fallback preserved. The
  optional utoipa Swagger Axum feature and direct axum-extra dependency are removed.
- Shared extensions: `Correlation::run_selected` now accepts arbitrary standard
  HTTP bodies; WebSocket compatibility forwards subprotocol selection; optional
  `multipart-owned` supplies owned fields with runtime exclusivity, retaining the
  prior parser, limits and rejection semantics rather than changing consumers.
- Baseline workspace: **660 passed, ten existing ignores**. Final workspace:
  **663 passed, the same ten ignores**. Existing auth, CORS, rate limit, tracing,
  correlation, CRUD, query, lifecycle and process suites pass. Additional direct
  shared-router checks cover plugin GET/HEAD/405, validation envelopes, exact
  legacy parser error messages and structured validator details.
- Optional Swagger routing contract: **four passed**, including redirect, HTML,
  CSS and OpenAPI. Server all-features check passes with a temporary compile-only
  `web/dist/index.html` fixture, removed afterwards; no frontend build is claimed.
  Workspace all-features is invalid because core storage backends are exclusive.
- Built the server and test-echo cdylib together. An isolated loopback process
  loaded the API v2 plugin, served ping/info with correlation headers, and exited
  cleanly on SIGTERM. Temporary database/blob/plugin directories were removed.
- Shared baseline **239**, final **243 tests/doctests**. New differential checks
  cover multipart fields, malformed/missing boundaries, 2 MiB limits and owned
  field exclusivity; real HTTP verifies WebSocket server-preference negotiation.
  Standard-body opaque correlation preserves body and task-local scope.
- Formatting/diff checks pass. Shared strict all-target/all-feature Clippy and
  minimal `multipart-owned` build pass. Fausto workspace/all-target Clippy passes
  with warnings; it is not a strict warnings-as-errors pass. Docker/browser E2E
  was not rerun; historical Step 01 results are not current verification.
- Remaining: owned multipart field/error types, WebSocket socket/message types,
  tracing compatibility, axum-test transport and legacy parser test oracle. No
  production router conversion back to Axum. Step 11 is Done; complete protocol
  abstraction is still pending.
- Integration: Fausto `master` rebased onto `b106e99`; shared `main` rebased onto
  its source/tracker migration commits. Verified ancestry and equality with the
  tested trees. Owned worktrees/branches and `/tmp/fausto-routing` targets/logs
  removed. No push or deployment; unrelated branches/worktrees preserved.


## lello-auth routing completion — 2026-09-25

- Applicability: standalone server, public `lello-auth-axum` integration crate,
  embedded example, external OIDC example and webhook example exposed backend
  routers, handlers, extractors and responses. The independent auth-helper has no
  HTTP server and is outside this migration. Active branch `master` started at
  `dee2d2b`; shared `main` started at `a2270c2`.
- Implemented in isolated sibling worktrees. Shared source revision:
  `18482f43f705c27c336fa9ebd0a6812ce957d566`; consumer commit `3d9569f`.
  Active README and all CI sibling-checkout pins select that shared revision.
- All route assembly, OAuth/OIDC forms, JSON, redirects, Askama HTML, static files,
  health/metrics, authentication/session middleware and custom request extractors
  now use shared contracts. Server and HTTP test harness retain direct transport
  peer metadata for the unchanged trusted-proxy policy. All three examples use
  shared routing and lifecycle serving.
- Extended simple-server with `Form`, `Html`, `Redirect`, standalone header-array
  responses, method-router layers, response mapping, tuple text rejections and
  JSON field access. Optional `tower-cookies` extracts the existing jar without
  changing cookie policy or response delta handling. Its backend extractor
  feature is disabled; unused axum-test/Axum macros/body-test dependencies removed.
- No direct Axum interfaces remain in production Rust, examples or tests. The
  reverse normal dependency tree shows Axum 0.8.9 only under simple-server. The
  public crate name remains `lello-auth-axum`; consumers must now compose its
  routes with `simple_server::web::Router`. No legacy router adapter is used.
- Baseline initially encountered SQLite `database is locked` in
  `sqlite_native_approval_contract`, before any source edits. Targeted rerun passed
  and all initially skipped suites were completed. Completed baseline: **989
  passed, 22 ignored**, including doctests. Final full `cargo test --workspace
  --no-fail-fast`: **989 passed, 22 ignored**, including that concurrency test.
  Existing external PostgreSQL fixtures and ignored doctests were not enabled.
- Existing real-HTTP suites cover login/logout, consent, OAuth/device/token flows,
  account/profile/admin operations, cookies/CSRF, trusted proxies and lifecycle.
  All three standalone examples compile before/after using offline resolution;
  they have no tracked lockfiles. Existing external-OIDC example warnings remain.
- Strict workspace/all-target Clippy passes on declared Rust **1.88.0**, using
  CI's existing `large_enum_variant` and `too_many_arguments` allowances. Full
  tests used the installed stable toolchain. Formatting/diff checks and all four
  `tests/ci_contract.py` checks pass.
- Shared baseline **243**, final **248 tests/doctests**. Five new contracts cover
  GET/HEAD/body form parsing, missing/wrong content type, malformed/duplicate
  fields, limits, HTML/encoded forms, redirect/error responses, multiple cookies,
  removal/attributes, missing cookie layer, response mapping on extractor failure
  and per-method body-limit scope. Strict all-feature/all-target Clippy and a
  minimal `tower-cookies` feature build pass.
- Docker/browser release gates, external PostgreSQL fixtures and unrelated
  auth-helper suites were not run; previous migration E2E results are historical.
- Integration: `master` rebased onto `3d9569f`; shared `main` rebased onto its
  source/tracker migration commits. Ancestry and tested-tree equality verified.
  Three pre-existing untracked identity-provider research files were preserved
  byte-for-byte. The existing release branch and old detached worktree record
  were preserved. Owned worktrees/branches and `/tmp/lello-auth-routing` build/log
  artifacts removed. Nothing pushed or deployed.


## lellostore routing completion — 2026-09-25

- Applicability: API/metrics servers, public/admin upload and delivery routes,
  custom auth extractors, static/range responses, mock OIDC binary and fixtures
  used backend interfaces. Active `master` started clean at `2292617`; shared
  `main` started clean at `4a28159`. Isolated sibling worktrees used throughout.
- Shared source commit `4db239946d21d385d8c8835538710aff6c322189`; consumer
  `af3661e`. README and CI sibling-source references updated to the tested source.
- Migrated all ordinary routes/handlers, custom and built-in extraction, response
  rendering, auth/metrics middleware, static fallbacks, health, streaming/range
  downloads and API/metrics serving. Mock OIDC routes and fixture serving use
  shared APIs; axum-test router conversion is confined to test transport.
- Added `web::multipart::{Multipart, Field, OwnedMultipart, OwnedField,
  MultipartError}`. Public parser/field/error contracts no longer name Axum or
  axum-extra. Borrowed fields enforce exclusivity statically; owned fields retain
  runtime exclusivity. Both expose metadata, standard headers, bytes/text,
  chunks and Stream with shared errors. Limits and parser wire behavior preserved.
- LelloStore APK/AAB, icon and VPK handlers use the new borrowed field API.
  Streamed file writes, bounded metadata reads, file/application limits,
  validation, storage and temporary-file cleanup remain consumer-owned. No
  complete-upload buffering is introduced. Legacy compatibility multipart
  adapters remain unchanged for consumers not yet migrated to the new API.
- Added shared RawQuery preserving undecoded query strings and absence/empty
  distinctions. WebSocket compatibility forwards existing frame/message limits
  and retains subprotocol selection; socket/message types remain a separate gap.
- Baseline default and all-feature suites: **189 passed, eight ignored** each.
  Final all-feature suite: **190 passed, the same eight ignores**. Added real-HTTP
  contract covers over-64-KiB metadata, duplicate files, missing files, invalid
  UTF-8 and temporary-directory cleanup after every rejected upload. Existing
  suites verify authenticated uploads/catalog events, publication/acquisition
  policy, ranges, delivery WebSockets, OIDC and tracing.
- All-target/all-feature strict Clippy, formatting and diff checks pass. Embedded
  builds use an unchanged copy of the original ignored frontend/dist assets;
  frontend sources were not rebuilt or changed. Six Android/toolchain/upstream
  integration cases and two doctests remain ignored. Docker, Android and frontend
  suites were not run.
- Shared baseline **248**, final **254 tests/doctests**. Differential borrowed and
  owned parser tests preserve metadata, binary content, malformed/boundary errors
  and body limits. Streaming/drop test proves early chunk delivery and body
  release. Raw query tests preserve encoding/duplicates; real HTTP tests verify
  both oversized frames and fragmented-message limits. Strict Clippy and minimal
  `web,multipart` / `multipart-owned` builds pass independently.
- Remaining observations: WebSocket protocol types, tracing compatibility and
  axum-test transport/helpers. Multipart is no longer a production Axum exposure
  in lellostore; other services retain their current multipart observations until
  they adopt the new shared field/error API.
- Integration: lellostore `master` rebased onto `af3661e`; shared `main` rebased
  onto its source/tracker migration commits. Verified ancestry and exact tested
  tree equality. Owned worktrees/branches and `/tmp/lellostore-routing` targets,
  copied assets and logs removed. Existing backup branch preserved. Nothing
  pushed or deployed.

## Favzetto routing completion — 2026-09-25

- Applicability: production routes/handlers, rate-limit middleware, file/static
  responses, multipart fields passed across ingestion/assistant/research modules,
  WebSocket upgrades and HTTP fixtures used backend types. Clean `master` started
  at `3339c4a`; shared `main` at `b9ed649`. Isolated worktrees used throughout.
- Shared source `c5ffbf21bee1b84dfbd065970f59842c2466024b`; consumer `685c55a`.
  Current lifecycle build instructions and the new consumer Step 11 record pin
  the reviewed sibling source. No root README or CI checkout pin exists.
- All ordinary routing, extraction, response conversion, middleware, server
  lifecycle and mock/test HTTP serving use shared APIs. Static fallback remains
  GET/HEAD-only. Multipart readers/fields/errors are shared public types;
  application buffering, storage, authorization and 128 MiB limit are preserved.
- Shared `Option<Json<T>>` added instead of changing the consumer contract.
  Absence requires missing Content-Type; declared invalid JSON still rejects.
  Differential tests compare exact status/headers/body for absence, valid JSON,
  +json, syntax/shape errors, unsupported media types and body limits.
- Consumer baseline **238 passed, two failed**; final **239 passed, the same two
  failed**, zero ignored. Known failures are
  `catalog_research_runtime_bridge_approves_runtime_draft` and
  `catalog_research_runtime_bridge_rejects_runtime_draft`: terminal transitions
  completed→completed/waiting_user return 400. Reproduced before edits.
- New real-HTTP contract verifies static POST rejection, missing multipart
  boundary, missing file, empty file and graceful shutdown. Existing suite covers
  successful upload/storage/approval, auth/rate limits and actual WebSocket flows;
  lifecycle/logging process tests pass. Final full run uses `--no-fail-fast`.
- Shared baseline **254**, final **255 tests/doctests**; strict all-feature,
  all-target Clippy and minimal `web` build pass. Consumer Clippy completes with
  existing warnings; full formatting has pre-existing differences. No broad
  reformat. Existing ignored web/dist assets copied unchanged for compilation;
  frontend, Android and Docker builds were not run.
- Remaining boundary: WebSocket socket/message types and upgrade compatibility.
  There is no remaining multipart field/error or HTTP fixture Axum exposure.
- Integration: Favzetto `master` rebased onto `685c55a`; shared `main` rebased onto
  source/tracker migration commits. Ancestry and exact tested source trees
  verified; temporary worktrees/branches and `/tmp/favzetto-routing` removed.
  Existing backup and Android branches preserved. Nothing pushed or deployed.

## Observo routing completion — 2026-09-26

- Clean active `master` started at `2427747`; shared `main` at `4d909ee`.
  Isolated worktrees preserved sibling source paths. Consumer commit `7217da1`;
  shared source `5ba465899f3cce6e08cda60283882b464edc8716`. Updated
  `simple-server.rev` (consumed by the existing checkout script), README and the
  consumer Step 11 record.
- Production routing, handlers, JSON/forms/path/query/state/peer extraction,
  HTML/redirect/error responses, auth middleware, plugin request/body proxying
  and lifecycle serving use shared APIs. `serve_with_connect_info` retains TCP
  peer identity; only the application's explicit trust-proxy policy interprets
  forwarded headers. Public metrics, CORS and 64 MiB body limits are preserved.
- Shared State adds Deref/DerefMut, preserving the existing metrics helper call
  without changing consumer application logic. No direct Axum import, Cargo
  dependency or compatibility adapter remains in Rust sources/manifests. Axum
  remains internal to simple-server; CLI tools/JavaScript components did not
  acquire new HTTP dependencies.
- Baseline and final Rust suites: **97 passed**, zero failed/ignored. Existing
  real-binary HTTP/SQLite suite passed both times: auth, public metrics,
  validation, CRUD/filtering, restart persistence, cron admission and shutdown.
- New real-binary routing contract suite passed on unchanged baseline and final
  implementation: HTML content type, exact 307/Location, URL-encoded form errors,
  unsupported form media type, binary proxy body/status/headers/raw query,
  missing plugins and forwarded-address allowlists with trust disabled/enabled.
  Tests use temporary databases/plugin manifests and loopback upstreams only.
- Shared baseline/final: **255 tests/doctests** each; strict all-feature/all-target
  Clippy, formatting and minimal web build pass. Normal consumer Clippy is blocked
  by the existing denied `approx_constant` in `src/indexing/embeddings.rs:387`;
  the file is byte-identical to starting master. `--cap-lints warn` completes.
  Consumer full formatting also has pre-existing differences; no broad reformat.
- Docker, browser/worker, external OIDC/LLM and standalone content-extractor or
  link-scorer suites were not rerun. No protocol compatibility gap remains for
  Observo in the audited Rust source scope.
- Observo `master` rebased onto `7217da1`; shared `main` rebased onto its source
  and tracker commits. Ancestry and exact tested source trees verified. Owned
  worktrees/branches and `/tmp/observo-routing` build/log files removed. Both
  original worktrees clean; nothing pushed or deployed.

## Paranza routing completion — 2026-09-26

- Clean active master started at `4c254ff`; consumer migration `4739150`.
  Reviewed shared source `f1999c47dc231add3ca00f236bf03e7f296cc966` is pinned in
  simple-server.rev and referenced by the README/checkout script. Isolated
  migration worktrees preserved sibling dependency paths.
- Server application's private routing, state/path/query/raw-body extraction,
  response conversion, health endpoint and lifecycle HTTP serving use shared web
  APIs. Internal helper names no longer describe Axum. Domain dispatch, PCM
  authorization/header precedence, query reconstruction and wire responses remain
  unchanged. Runner TLS and binary protocols remain application-owned.
- No direct Axum API/import/dependency or compatibility adapter remains in Rust
  sources/manifests. Axum remains internal to simple-server. No shared API
  extension was needed; other workspace crates do not own these HTTP routes.
- Baseline full all-feature workspace: **286 passed**, zero failures/ignores.
  Final: **287 passed**, zero failures/ignores. New real-HTTP regression passed
  on original and migrated routing over loopback and temporary SQLite state:
  health/nodes, JSON error content type/code, malformed raw JSON, PCM missing
  credentials, authorized query pagination, message-path 404, method 405 and
  graceful shutdown. Existing suites retain TCP/TLS/Unix-socket coverage.
- All-feature/all-target workspace Clippy completes with existing warnings.
  Full formatting has pre-existing differences; no broad reformat. Diff checks
  pass. Shared library code is unchanged; its prior 255-test result is historical
  and was not rerun for this consumer-only change.
- Docker fleet scenarios, Android/release builds and real-machine wake/poweroff
  operations were not run. No protocol compatibility gap remains in the audited
  Rust scope.
- Paranza master rebased onto `4739150`; shared main rebased onto tracker commit.
  Ancestry and exact tested-tree equality verified. Both original worktrees clean;
  owned worktrees/branches, temporary builds/logs and timestamped artifacts from
  these test runs removed. Nothing pushed or deployed.

## Peerlo routing completion — 2026-09-26

- Clean active master started at `f4fa367`; migration `5c93e78`. Reviewed shared
  source `75c74993da0865ab2e2f9e2e14207b0e1d800725` is recorded in simple-server.rev;
  README and consumer Step 11 record updated. Isolated worktrees preserved
  sibling dependency paths. Shared library implementation did not change.
- Public router constructors, REST handlers/extractors/responses, custom errors,
  auth/rate-limit/metrics middleware and serving use shared web APIs. Peer-aware
  serving preserves per-IP rate budgets. Test routers/body helpers use shared
  types and the unused direct http-body-util test dependency was removed.
- Baseline full all-feature workspace: **796 passed**, six existing ignores.
  Final: **797 passed**, same six ignores, no failures. New real-HTTP production
  composition test passed on original and migrated APIs: CORS preflight bypass,
  ordinary unauthorized request budget charging, bearer success and outer rate
  denial, preserved CORS/JSON/Retry-After and TCP peer extraction.
- Existing HTTP tests retain health, bearer/Torznab access ordering, throttling,
  draining and server-failure behavior. Tracing integration tests verify status
  severity, metrics, safe labels and final body completion. Two real loopback
  Peerlo processes pass DHT bootstrap, HTTP runtime log-filter updates,
  SIGTERM/SIGINT and ordered subsystem cleanup.
- All-feature/all-target workspace Clippy completes with existing warnings;
  formatting and diff checks pass. Shared code unchanged; prior 255-test result
  is historical and was not rerun here. Docker fleet/load/chaos scenarios,
  deployment and public DHT operations were not run.
- Remaining boundary: existing tracing Observer callback still accepts the
  backend Response through web::compat::trace_with_observer. Ordinary routes,
  handlers/extractors, middleware Next, serving and test transport are shared;
  tracing is not falsely marked fully abstracted in the observations column.
- Peerlo master rebased onto `5c93e78`; shared main rebased onto tracker commit.
  Ancestry and exact tested-tree equality verified. Both original worktrees
  clean; owned worktrees/branches and /tmp/peerlo-routing build/log/temp files
  removed. Nothing pushed or deployed.

## Meteonesto routing completion — 2026-09-26

- Clean active master started at `e39aa72`; consumer migration `e69685f` plus
  temporary-target-link removal `b9381ae` (integrated tip). Shared source
  `364f57688200a35c26fa159661e5a902d4f62f40`. Isolated worktrees preserved sibling
  dependency paths. README and current lifecycle build instructions reference
  the reviewed source; historical migration references are preserved.
- API, gateway and pipeline control-plane routing/handlers/extraction/responses,
  auth/correlation/deadline middleware and startup use shared HTTP APIs.
  Gateway peer-aware serving retains loopback/forwarded-client policy. Static
  service mounts, map-frame bytes, proxy headers, local provider fixtures and
  HTTP test routers use shared types. No direct Axum API/import/dependency or
  compatibility adapter remains in these Rust sources/manifests.
- Shared Router preserves the backend's must_use annotation; no runtime behavior
  changes. Explicit mock-server Shutdown values satisfy consumer pedantic lints.
  The touched control-plane constructor gains required panic documentation for
  its existing static response expect, and its prior formatting difference is
  normalized. Application policy and protocols remain unchanged.
- Baseline/final all-feature Rust: API **71**, gateway **33**, pipeline **164**;
  **268 total passed**, one unchanged gateway ignore, zero failures. Existing
  contracts verify proxy/client identity, token budgets, request correlation,
  pipeline auth/audit, backups, conditional configuration, byte/static responses,
  provider mocks, task drain and watchdog schedulability.
- Baseline/final Python: four API process/map-sync tests and two pipeline process
  lifecycle tests pass. They exercise real loopback processes with temporary
  configuration/data, signals, listener release and administrative drain.
- Strict all-feature/all-target Clippy and formatting pass in all three Rust
  components; diff checks pass. Shared minimal web baseline check and final
  strict library Clippy pass with Rust 1.97.1. Shared runtime suite was not rerun
  for the annotation-only change; previous 255-test result remains historical.
- Full Android/renderer/user-service/JavaScript, external providers/OIDC,
  Docker/release/deployment checks were not run. No production data used.
- Meteonesto master rebased onto migration/cleanup commits; shared main rebased
  onto source/tracker commits. Ancestry and exact integrated-tree equality
  verified. Both original worktrees clean; owned worktrees/branches and
  /tmp/meteonesto-routing build/log/temp files removed. Nothing pushed or deployed.

## Pezzottflix routing completion — 2026-09-26

- Active development branch: `master`, clean baseline `cd5eb29`; migration
  `c60517a9bcb186e3c7ef3ccba049062fffe703cc`, integrated into `master` with
  matching trees and verified ancestry.
- Reviewed shared source: `982df2612c08035b65f8dab4f3d86c4025fe9c40`.
  Root `simple-server.rev` updates the existing CI/Docker checkout workflows.
- All production route groups, HTTP/metrics serving, ordinary handlers,
  custom authentication/language/pagination extractors, middleware, response
  adapters, streaming bodies and HTTP tests use `simple_server::web`.
  Existing authorization, peer addresses, upload headers, compression/static
  middleware and JSON error contracts are preserved.
- Shared extensions: lazy `Body::into_data_stream`, per-method `route_layer`
  preserving 405 fallbacks, and header-only `IntoResponse`. Differential tests
  verify chunks/errors/trailers, cancellation, middleware and repeated headers.
  A consumer test verifies streaming permits survive until EOF or cancellation.
- Baseline: **598 passed**, three ignores. Final: **599 passed**, same ignores.
  Shared baseline/final: **255/259 passed**. Shared formatting, strict
  all-feature/all-target Clippy and minimal `web` feature check pass.
  Consumer Clippy completes with warnings; workspace formatting already fails
  on the untouched baseline, and unrelated formatting is preserved.
- Debug server build passes. Real-server diagnostics pass SIGINT and SIGTERM:
  HTTP/metrics, request IDs, log redaction, authenticated socket draining,
  worker shutdown and port release. The unchanged lifecycle script fails
  intermittently on the shutdown close-reason assertion, reproduced on clean
  baseline: unregister closes the event channel and races the send-task close
  notification, producing either an empty close or the shutdown reason.
  A separate temporary diagnostic accepted both baseline variants; the
  original script and application behavior were not changed to mask the issue.
- Remaining compatibility: WebSocket upgrade adapter/socket/message/close-frame
  protocol types and tracing observer's backend response callback. HTTP core
  Done does not claim complete Axum removal.
- Release/frontend builds and Docker E2E were not run; debug build retains its
  existing frontend compilation skip. Details are in consumer
  `docs/step-11-routing.md`.
- Both development branches rebased onto isolated migration branches;
  ancestry/tree equality verified. Owned migration/baseline worktrees,
  temporary branches, build outputs and scratch files removed after integration.
  Nothing pushed or deployed.

## Pezzottify-downloader routing completion — 2026-09-26

- Active branch: `master`; clean baseline `3e04e15`. Migration
  `a367fb191d360c64197d85a0bffd6bc4210b372c` is integrated into `master`
  with matching trees and verified ancestry.
- Reviewed shared source: `982df2612c08035b65f8dab4f3d86c4025fe9c40`;
  `simple-server.rev` updates the existing reproducible checkout/Docker workflow.
  No new shared-library code was required.
- Parent TCP and child Unix HTTP routers, all ordinary handlers/extractors,
  responses, bodies, middleware and HTTP mock/test routers use shared `web` APIs.
  Parent startup uses `web::serve`. The child serves the shared Router through
  its existing Hyper accept loop; 0700 socket permissions, upgrades, graceful
  connection draining and socket cleanup are preserved.
- Hyper Unix proxy requests/responses use shared bodies without buffering;
  application scheduler, correlation, CORS, JSON errors, idle connections and
  activity ownership remain unchanged. Login and cron quota CLI have no HTTP
  routing and remain unchanged.
- Baseline/final: **169/170 Rust tests passed**, one ignored doctest. Final:
  131 library tests, three CORS, 16 error contracts, three logging, six parent/
  Unix proxy integration, four CLI/server E2E and seven doctests passed.
  Existing streaming proxy tests verify body/header/status forwarding and
  headers arriving before upstream completion; new EOF/cancellation coverage
  verifies streamed response activity is retained and subsequently completed.
- Real binary E2E covers HTTP health/status/method handling, correlation and
  redacted logging, upgraded status WebSocket, SIGTERM/SIGINT shutdown, and
  Linux parent-death cleanup, without Spotify credentials or external services.
  Cargo test builds both executable targets. All-target Clippy completes with
  documented existing warnings. Workspace formatting fails on untouched
  baseline; unrelated formatting was preserved, added test source formatted,
  and diff whitespace checks pass.
- Remaining: WebSocket upgrade compatibility, backend socket/message types
  across downloader/parent/proxy, and tracing observer's backend response
  callback. Standard Hyper Unix transport and Tungstenite clients remain.
- Spotify-backed child startup/downloads, OAuth browser flows and release/Docker
  builds were not exercised. Python cron tests were not rerun: pytest is absent
  from the available interpreter; those sources are unchanged. See consumer
  `docs/step-11-routing.md` for scope and evidence.
- Both base branches rebased onto isolated migration branches and verified.
  Owned worktrees, temporary branches, build outputs and scratch files removed
  after successful integration. Nothing pushed or deployed.

## SCT routing completion — 2026-09-26

- Active branch: `master`; baseline `3b1144f`. Consumer migration
  `2a33b85444a7a5aea0321fe91c9c7daa7dd989d1` integrated into `master`
  with verified ancestry and matching trees. Original unrelated untracked
  `.validation-work/` and `docs/step-04c-cors.md` preserved; neither was staged.
- Reviewed shared source: `23626b234eb1e854c3b3df5d62fe63375ec7a197`;
  `simple-server.rev` and active README references updated.
- Every server route group, ordinary handler, custom auth extractor, request/
  response/body contract, middleware, production serving mode, HTTP test/mock
  fixture and qualification HTTP example uses shared `web` interfaces.
  Static fallback, body limits, health, headers, correlation, auth/cookies/CSRF,
  worker ownership and transfer policies remain unchanged.
- JSON/query rejections retain application mapping to InvalidRequest. One
  canonical buffered ApiError preserves exact protocol JSON bytes, server UUID
  request ID/header, no-store, retry-after and ErrorCode response extensions.
  New coverage verifies both extraction and ordinary response paths.
- Shared Method head extraction supports archive ISO GET/HEAD without backend
  handler traits. Differential GET/HEAD/POST/custom-method coverage and request
  head mutation ordering pass. Streaming ISO/checkpoint/transfer and range/HEAD
  policies retain existing implementation and integration coverage.
- Baseline/final workspace: **55/56 passed**, 91 explicitly ignored. Dedicated
  disposable PostgreSQL integration runs **83 tests each** before and after,
  covering auth/OIDC/CSRF, safe errors, trees/lifecycle/storage, archives,
  checkpoint/recovery, observability, SDK and real processes. Shared all-feature
  suite: **261 passed**. Strict all-target/all-feature Clippy and formatting
  pass in both repositories. Debug workspace build and generated contracts pass.
- Real-process Python lifecycle suite passes both OS signals, HTTP draining of
  an 8 MiB streamed static file, writer lease release/restart and nonzero exit
  after lease loss. All database/static fixtures and containers are disposable.
- Remaining Axum exposure: tracing observer backend response callback via
  `web::compat::trace_with_observer`; ordinary routing has no backend conversion.
- Live S3/MinIO, heavy scale qualification, frontend/Chromium and release builds
  were not run; the repository-pinned MinIO image is not cached locally.
  Existing ignore/qualification boundaries remain. No production database or
  external identity provider was used. See consumer `docs/step-11-routing.md`.
- Both development branches rebased onto isolated migration branches;
  ancestry/tree equality verified. Owned worktrees, branches, test containers,
  build outputs and scratch files removed after integration. Nothing pushed
  or deployed. Quentin Torrentino remains pending at the user's request.

## SimpleAI routing completion — 2026-09-26

- Active branch `master`, baseline `9648e2d`; consumer
  `672c586d0583972627732b22ef225395fc93db05` integrated with verified ancestry
  and matching trees. Original semantic-evaluation work and modified README
  preserved. Integration ran through the clean worktree while the original
  checkout retained its dirty tree; the README content hash is unchanged.
- Shared source `fb6a9e5a91f40529c761a7a7e44c33a442143ed5` is recorded in
  `simple-server.rev` for existing CI/Docker/README checkout workflows.
- Gateway and runner route groups, handlers, extractors, responses/bodies,
  middleware, serving and HTTP fixtures use shared `web`. Gateway peer metadata
  uses `serve_with_connect_info`. Multipart image/audio fields and errors use
  shared types; auth, quotas, auditing, inference ownership and streaming persist.
- Shared `MethodRouter::with_state` binds the runner WebSocket route independently
  of the enclosing router, preserving application state ownership. Differential
  coverage verifies method/outer state, GET/HEAD and method rejection behavior.
- Baseline/final workspace: **462 pass each**, one existing ignored doctest.
  Existing ignored `scripts/configs/rtx.toml`, referenced by a runner config test,
  was copied from the original checkout before baseline and used unchanged.
  It is excluded from commits; fresh checkouts without it cannot compile that
  pre-existing test. API tests cover Responses/chat, quotas/errors, streaming,
  multipart, health/CORS/headers and safe tracing. Real runner E2E covers mock
  discovery/chat, SIGINT/SIGTERM draining and gateway connection behavior.
- Debug workspace build and ordinary all-target Clippy pass with warnings.
  Strict Clippy is blocked by unchanged common-crate warnings; full formatting
  fails on the untouched baseline. Unrelated formatting is preserved; diff
  whitespace checks pass. Shared 262 tests, strict Clippy and formatting pass.
- Remaining: gateway WebSocket sockets/messages, admin SSE events/keepalive via
  compatibility response, and tracing observer backend callback. Multipart is
  fully shared. No ordinary backend router conversion remains.
- Docker/browser/Android, live model downloads and GPU inference were not run.
  Details: consumer `docs/step-11-routing.md`. Both base branches integrated;
  owned worktrees/branches, fixture copy, build files and scratch outputs removed.
  Nothing pushed or deployed.

## Simple-agents routing completion — 2026-09-26

- Active branch `main`, baseline `9ce77fb`; consumer
  `0efa5a77c17769867b9d22d3973f2fdc90037dad` integrated with verified ancestry
  and matching trees. Unrelated pre-existing publication worktree/ref preserved.
- Shared source `fb6a9e5a91f40529c761a7a7e44c33a442143ed5` recorded in
  `simple-server.rev`; README links the consumer routing record.
- All service route groups, handlers/extractors, request/response/body contracts,
  per-route response mapping, middleware, serving and coding-crate GitHub/GitHub
  App HTTP fixtures use shared APIs. Browser/native/OIDC auth, cookies, capability
  checks, SQLite transactions, no-store/body limits, release streams, session
  admission, worker ownership and broker subprotocol remain unchanged.
- Baseline full workspace: **367 pass, one fail, one ignore**. The stale identity
  assertion omitted the existing `display_name: null`; its exact expected JSON
  was corrected without changing production. Final: **368 pass, zero fail,
  one ignore**. Public-client fixtures now request graceful shutdown and await
  connection draining before closing/reopening SQLite; existing replay/restart
  tests verify this. Abrupt cancellation does not promise connection-task drain.
- Existing HTTP tests cover authorization, limits/no-store, SSE replay/revocation,
  broker handshake/subprotocol/reconnect, releases, native/browser auth, SDK,
  distributed work and real executable shutdown under both signals. Strict
  all-target Clippy, formatting and debug workspace build pass. Shared **262
  tests**, strict Clippy and formatting pass. **Five signed-runtime-asset Python
  tests and 19 frontend Node tests** pass; frontend build reproduces tracked
  web/dist exactly. No web source or dependency lock changes are included.
- Remaining: session SSE event/keepalive producer via compatibility response and
  broker WebSocket upgrade/socket/message protocol types. No ordinary backend
  routing conversion remains.
- Extraction-parity script fails on the unchanged stale missing `web/app.mjs`
  destination; inventory and web sources were not changed by migration.
  Android Gradle, Chromium and worker-image qualification were not run.
  Details: consumer `docs/step-11-routing.md`.
- Both base branches integrated; owned worktrees/branches, installed dependencies,
  build files and scratch outputs removed after verification. Nothing pushed or
  deployed. Quentin Torrentino remains pending, skipped at user request.

## Owned WebSocket API — 2026-09-26

Implemented in an isolated `feature/owned-websocket` worktree from shared `main`
at `9f673c4`. The new `web::ws` module owns every public protocol type and wraps
the internal transport privately; `web::extract::WebSocketUpgrade` is also
available. Existing compatibility callers retain their API. Configuration covers
buffers, frame/message limits, unmasked frames, requested/selected protocols,
manual selection and failed-upgrade callbacks. Stream/Sink split operation,
control-frame processing, error sources and application close reasons are preserved.

Verification: baseline all-feature suite passed (262 tests). Eight new tests
exercise real TCP negotiation, split text/binary echo, Ping/Pong, client/server
close reasons, fragmented UTF-8, size limits, masking opt-in, malformed text,
HTTP/1 and HTTP/2 rejection parity and deterministic background upgrade failure.
Final all-feature suite: **270 tests passed**, zero failures. Strict all-feature,
all-target Clippy and formatting pass. Production-only library checks for
`ws` and `web,ws` with default features disabled both pass. HTML script syntax,
links, service counts and matching observations validated.

Library capability only: no consumer has been migrated in this change, and the
service observations/counts remain unchanged. Consumer authentication, heartbeat
policy and lifecycle ownership remain local. Nothing pushed or deployed.

## Pezzottify owned WebSockets — 2026-09-26

Applicability: production `/v1/ws` synchronization and `/v1/mcp` both used the
compatibility upgrade extractor and backend socket/message types. Both now use
`web::ws::{WebSocketUpgrade, WebSocket, Message}`, including their split streams.
No backend WebSocket imports or compatibility upgrades remain in the service.
Session/device checks, MCP lifetime authorization, payload schemas, sync broadcasts,
connection registration, control-message handling and tracked shutdown are preserved.
Transport defaults are unchanged.

Reviewed shared source: `46c724315a3ed35e35cb086b2328bb4040cd0531`.
Consumer commit: `9115a8a4`, integrated into **dev**, based on
`fd6585e2`. `simple-server.rev` updates the existing CI/Docker checkout mechanism.
The independent Tungstenite client and dependency lockfile remain unchanged.

Verification:

- Baseline pinned-source suite: **1,449 passed, 36 existing ignores**.
- Two new real-TCP tests pass before and after migration: Ping/Pong payloads,
  ignored binary messages, malformed-JSON errors and subsequent valid requests
  on both endpoints.
- Final `cargo test --locked --features fast`: **1,451 passed, 36 ignores**.
  Includes sync/reconnect/auth, MCP permission refresh/session revocation, and
  real-process SIGINT/SIGTERM/admin reboot closing both WebSocket endpoints.
- Formatting, strict production Clippy, default-feature debug build and
  database-boundary checks pass. The existing num-bigint-dig future-compatibility
  notice and test-suite unused-import warnings remain. Docker and Android
  qualification were not repeated for this migration.

Multipart fields/errors, SSE producers, tracing observer and independent HTTP
mock/differential tests remain explicit Axum exposure. Other services' WebSocket
adoption has not changed. The HTML and Markdown observations match, and the
routing count stays 16 Done / 1 Pending (Quentin Torrentino remains skipped).

The original clean `dev` branch was rebased onto the committed migration branch;
ancestry and tree equality were verified. The consumer worktree, temporary branch
and build artifacts were removed. Pre-existing Paravoid worktrees and branches
were preserved. Nothing pushed or deployed.

## Owned WebSocket rollout — 2026-09-26

Reviewed shared source for this batch:
`46c724315a3ed35e35cb086b2328bb4040cd0531`. Service agents use isolated
worktree branches and preserve application auth, protocol payloads, transport
configuration, task ownership and shutdown. Completion evidence belongs here;
remaining-exposure cells list pending work only.

### Favzetto

- `master`: `685c55a` → `d8cadbd9a92d57455e962c09d7bea510d9afdce8`.
  Assistant, catalog research bridge and native catalog research sockets now
  use owned upgrades/sockets/messages. Removed `web-compat`; the active shared
  checkout instructions identify the reviewed revision. No remaining Axum or
  compatibility imports in backend production or tests.
- Baseline: **239 passed, two existing API failures**. Final: **240 passed,
  the same two failures** (`catalog_research_runtime_bridge_approves_runtime_draft`
  and `catalog_research_runtime_bridge_rejects_runtime_draft`, terminal runtime
  transitions return 400). These are not reported as a fully green suite.
- New real-connection regression passes before/after across all three routes:
  bad-key rejection, successful upgrade, exact Ping/Pong payload, ignored binary
  messages, JSON heartbeat and close. Final focused WebSocket suite: **11 passed**.
  Both process lifecycle and both logging tests passed in the full suite.
- Clippy passes with existing warnings capped; repository formatting has existing
  differences. No frontend/Android/container rebuild. Existing ignored web assets
  were copied unchanged. Service evidence: `docs/step-11-websockets.md`.
- Original clean master rebased onto the migration; ancestry and tree verified.
  Owned worktree, branch, source archive, build artifacts and retained logs removed.
  No push/deployment. Remaining exposure: **None**.

### Androidoscopy

- `master`: `5e9a396` → `5f25aa8aea49169bc7064f02d77bf805fb8fbc58`.
  Both controller and legacy servers use owned WebSocket upgrades, sockets and
  messages; `web-compat` removed. README and reviewed-source pin updated.
- Baseline/final server all-target tests: **78 / 79 passed**. Full-stack tests:
  **12 passed before and after**. New real controller regression passes on both
  implementations: unauthenticated/foreign-origin 401, bearer/cookie upgrades,
  SYNC/event broadcast, malformed-input tolerance, Ping/Pong, shutdown task drain
  and late-upgrade 503. Existing legacy WebSocket/TLS registration tests pass.
- Normal all-target Clippy and build pass. Strict Clippy stops on the unchanged
  `config.rs` derivable-Default warning; formatting retains baseline differences
  in protocol/session/TLS files. Changed files pass formatting. Initial native
  dependency build issue recovered with debug information disabled; loopback
  checks passed after sandbox escalation.
- Rebased original clean master onto the migration; ancestry and exact tested
  tree verified. Owned branch, worktree, source archive, builds and logs removed.
  Evidence: `docs/step-12-websockets.md`. No push/deployment.
- Remaining: `axum-server` TLS configuration, serving and shutdown handle in the
  legacy main and TLS fixture. No backend WebSocket/compatibility imports remain.

### Crumbles

- `master`: `cfc97f3` → `20ca527cad890da2e2959da530339cb32978aba8`.
  Production `/api/ws` uses owned upgrade/socket/message types; the integration
  server has no WebSocket endpoint. README, source pin and service routing
  evidence updated to the reviewed shared revision.
- Baseline against reviewed shared source: **59 WebSocket tests passed**.
  Final locked workspace suite: **1,445 passed, two existing ignores**. Existing
  real TCP tests cover upgrade admission, replay/reconnect ordering and gaps,
  live permissions/revocation and shutdown joining of tracked connections.
- Formatting, strict workspace/all-target Clippy, workspace build, diff checks
  and single Axum 0.8.9 dependency verification pass. Baseline initially lacked
  ignored frontend assets; copied unchanged original assets before source edits.
  Browser, Android and container suites were not rerun.
- Rebased master onto the migration through the clean linked worktree. Tested
  tree and ancestry verified; unrelated `ANDROID_CLIENT_PLAN.md` changes and
  ten untracked Android/design/documentation files preserved with byte/hash
  checks. Existing unrelated worktree preserved. Owned service/shared worktrees,
  migration branch, build artifacts and scratch directory removed.
- Evidence: `docs/SIMPLE_SERVER_ROUTING.md`. Remaining: multipart fields/errors
  and tracing compatibility adapter. No push/deployment.

### Fausto

- `master`: `e24e6f5` → `4e1820f0433660d97ed12fbe84f3a2f7b7aedd5c`.
  Event WebSockets use owned upgrade/socket/message types. Active README and
  all four CI source pins updated to the reviewed shared revision. Existing
  subprotocol-token preference, query fallback, auth, event payloads, connection
  limits, tracking and shutdown preserved.
- Baseline workspace suite: **663 passed, ten ignored**. Final: **664 passed,
  ten ignored, zero failures**. The new real TCP regression passes before/after:
  `fausto-ws` selection, no-protocol fallback, connected/application heartbeat,
  ignored binary frames, transport Ping/Pong, close, session drain and late 503.
  Existing fail-closed/token-precedence tests remain green.
- Formatting passes. Strict Clippy reproduces exactly the same **299 pre-existing
  fausto-core findings** as the baseline; not reported as strict-lint clean.
- Remaining: owned multipart fields/errors, tracing compatibility, and
  `axum-test` transport/parser oracles. Dynamic plugin rebuilding requirements
  remain documented separately from pending Axum exposure.

- Workspace build and warning-mode all-target Clippy pass; no new warning in the
  transport regression. Docker/browser E2E and live OIDC were not rerun. Evidence: `docs/simple-server-migration.md`.
- Original clean master rebased onto the migration; ancestry and exact tested
  tree verified. Owned service/shared worktrees, branch, builds and logs removed.
  No push/deployment.

## Remaining owned WebSocket rollout — 2026-09-26

Reviewed shared source: `46c724315a3ed35e35cb086b2328bb4040cd0531`.
Five GPT-6 Sol agents own disjoint service migrations; the coordinator owns these
trackers. Each service uses a dedicated branch/worktree, verifies protocol behavior,
commits and rebases its active development branch onto the migration. Remaining
cells contain pending exposure only. Quentin Torrentino remains skipped.

### LelloStore

- Clean `master`: `4ecc2d3` → `551054049961a239a2520175fc5fe8030685c10b`. Catalog and Paravoid delivery endpoints
  use owned upgrade/socket/message types. README and CI pins select the reviewed
  source. Authentication, subprotocol selection, 4096-byte limits, subscription
  deadline, grants/revocation, reconnect hints and shutdown ownership preserved.
- Baseline/final all-feature suite: **210 passed, eight existing ignores**.
  Expanded real endpoint Ping/Pong and Paravoid negotiation assertions pass
  before and after the import change. Existing auth/publication/shutdown/reconnect
  and wrong-scope checks remain green.
- Strict all-target/all-feature Clippy, formatting, all-feature build and diff
  checks pass. Sandbox listener denial resolved by approved localhost testing.
  Existing ignored frontend assets copied unchanged; frontend, Android and Docker
  suites not rerun. Evidence: `docs/STEP_11_WEBSOCKETS.md`.
- Remaining: tracing compatibility adapter and `axum-test` transport/helpers.

- Original clean master rebased onto the migration; original ancestry and tested
  tree verified. Service worktree and branch removed; coordinator completed
  remaining exact-source archive cleanup. No owned scratch artifacts remain.
  Nothing pushed or deployed.

### Pezzottify-downloader

- Clean `master`: `a367fb1` → `41c31bb354d6caffc92dac88a9093354d29b755a`.
  Parent status/download proxy and child download sockets use owned upgrades,
  sockets, messages and split sinks. Source pin and README updated. The Unix
  Tungstenite client remains independent, with explicit owned-message conversion.
- Baseline and final suite including new contract: **164 Rust tests and seven
  doctests passed; one doctest ignored**. New real TCP → production proxy → Unix
  peer check passes before/after: exact path, UTF-8 text, binary, actual forwarded
  Ping and unsolicited Pong, echoed payloads and close. Existing process checks
  verify status JSON and upgraded-socket closing on SIGINT/SIGTERM.
- Both production binary builds pass. Non-strict all-target Clippy passes with
  existing warnings; strict mode reproduces the same 12 baseline findings.
  Existing formatting differences remain; new test and diff checks pass. Python
  cron checks could not import pytest. Live Spotify/OAuth, release/Docker and
  deployment checks were not run. Evidence: `docs/step-11-websockets.md`.
- Remaining: backend response callback and compatibility adapter for HTTP tracing.
- Original clean master rebased onto the migration; coordinator verified ancestry
  and identical trees. Coordinator completed cleanup of the service worktree,
  migration branch, exact-source archive and enumerated logs/build artifacts.
  Nothing pushed or deployed.

### Pezzottflix

- Migration commit `19935d21da02a8d179653608c263a6e1ee9c92a5` from `c60517a`;
  integrated `master` is `744fea60c3fea2ccfbf418829c94f7098f9b648e`. A concurrent
  search fix (`08a0b1d`) was replayed after the migration and preserved in
  `recovery/pre-owned-websocket-08a0b1d` before rewriting its parent.
- Production WebSocket upgrade/socket/message/close-frame contracts adopt the
  owned API, including the application shutdown close code. No transport defaults
  or message policies change.
- Baseline: **599 passed, three ignored**; production server build passes.
  Real-server transport checks cover authentication, connected JSON, ignored
  binary/malformed messages, subscription/progress messages, user isolation,
  Ping/Pong and client close.
- The unchanged lifecycle script reproduces its known close-reason race: an
  empty close frame fails its reason assertion. An isolated diagnostic accepting
  that existing behavior passes both SIGINT and SIGTERM drain checks.
  Strict baseline Clippy reports 39 existing findings; formatting has baseline
  differences. These are not treated as migration regressions.

- Final migration suite: **599 passed, three ignored**; after integrating the
  concurrent search change, **601 passed, three ignored**, with the combined
  production build passing. Final real transport checks and both signal-drain
  diagnostics pass; unchanged lifecycle reason assertion reproduces its baseline
  failure. Strict Clippy/formatting retain the same baseline findings; ordinary
  Clippy completes with warnings. README and source pin updated.
- Remaining: tracing observer backend response callback. Evidence:
  `docs/step-12-websocket.md` and `scripts/test-websocket.py`.
- Clean master integration and combined tested tree verified. Migration/shared
  worktrees and migration branch removed; coordinator removed the remaining
  target/log/diagnostic scratch directory. The recovery branch for the concurrent
  search commit is intentionally retained. Nothing pushed or deployed.

### SimpleAI

- `master`: `672c586` → `95efd02`. Gateway and admin sockets use owned
  upgrades/sockets/messages; source pin updated. Runner outbound Tungstenite
  remains an independent client. Auth, registration, heartbeat, subscriptions,
  transport settings and application task/shutdown ownership preserved.
- Baseline workspace: **462 passed, one ignored**. Final workspace: **464 passed,
  one ignored**. Real gateway/admin checks pass before/after; final targeted
  rerun after test formatting/assertion edits: **18 passed**. Covers rejected
  runner secret, registration acknowledgment, peer-derived address, Ping/Pong,
  registry cleanup and malformed/invalid admin authentication.
- Workspace build and diff checks pass. Strict Clippy repeats three existing
  common-crate findings; full formatting retains existing differences. The
  ignored `scripts/configs/rtx.toml` fixture was copied unchanged for the existing
  include-str test. Successful JWT refresh, real GPU/Wake-on-LAN and Docker
  gateway E2E were not rerun. Evidence: `docs/simple-server-migration.md`.
- Coordinator reviewed and committed the agent implementation, rebased master
  through the clean linked worktree, verified tree/ancestry, and verified original
  README and semantic-evaluation untracked files with hashes/status before and
  after integration. Owned worktrees, branch, fixture, builds and logs removed.
- Remaining: admin SSE event/keepalive compatibility and tracing observer
  backend-response callback. Nothing pushed or deployed.

### Simple-agents

- Migration commit `3f1e013be74d86c86ee40bb927d0ab19e494cac2`, based on current
  `main` at `2d76212`. Broker upgrade/socket/message types use the owned API;
  active source pin updated. Auth, subprotocols, payloads, connection policy and
  application lifecycle remain unchanged; outbound clients are independent.
- Baseline workspace: **370 passed, one ignored**. Final workspace: **371 passed,
  one ignored**. New real TCP regression passes before/after, covering auth and
  subprotocol rejection/selection, Welcome/Poll messages, malformed/binary/oversized
  messages, peer-close behavior and shutdown admission. Its initial close-frame
  expectations were corrected on the original implementation before migration;
  no production close policy was changed to satisfy the test.
- Strict all-target workspace Clippy, formatting, diff checks and workspace debug
  build pass. Remaining: session SSE event/keepalive producer and response adapter.

- Original main gained an unrelated `docs/RUNNER_RELEASE_DELIVERY.md` edit during
  execution. Integration detached the original checkout, rebased in the clean
  linked worktree, and restored main with that edit preserved. Tested tree and
  ancestry verified. Existing prunable publication worktree/ref preserved.
- All five services' original branches now contain the migration commits; the
  final source audit finds no remaining direct Axum WebSocket/compatibility
  upgrade imports in the active rollout. Quentin Torrentino's backend dashboard
  WebSocket remains intentionally untouched. Nothing pushed or deployed.
- Simple-agents service/shared worktrees and migration branch removed; coordinator
  removed the seven retained baseline/final logs. Service evidence:
  `docs/step-12-websocket.md`. Live Runner adapters and browser/Android checks were
  not rerun. All five owned migration scratch directories are now absent.

## Owned HTTP tracing API — 2026-09-26

- Shared library implementation only; consumer adoption remains pending.
  Existing remaining-exposure cells are unchanged. Nine services have tracing
  boundaries recorded: Pezzottify, Crumbles, Fausto, LelloStore, Peerlo,
  Pezzottflix, Pezzottify-downloader, SCT and SimpleAI.
- Added `web::tracing` behind `web,http-tracing`, without `web-compat`.
  Owned entry points accept shared request/response bodies. Observer callbacks
  receive read-only `ResponseInfo`: status, HTTP version, headers and application
  extensions, without access to the backend response or body. SCT's extension
  metadata and consumer-selected event policies remain supported.
- Legacy observers and compatibility entry points remain source-compatible.
  Both APIs share the existing lifecycle engine and default event implementation;
  no buffering, event-level, target, upgrade or cancellation policy changes.
- Isolated branch/worktree based on simple-server `main` at `39e1c17`.
  Baseline all-feature suite passed (270 tests). Final suite: **285 passed**.
  Fourteen existing contracts now run against both legacy and owned APIs,
  plus a metadata-view test. Coverage includes production router templates,
  sensitive-data exclusion, status, extensions, streaming/trailers, timing,
  errors, cancellation, protocol-bodyless responses, upgrades and correlation.
- Strict all-feature/all-target Clippy and formatting pass. Minimal
  `web,http-tracing` suite: **13 passed** without compatibility or correlation.
  Standalone `http-tracing` and `web` feature checks pass.
  A transient linker failure cleared on retry; sandbox-blocked socket tests
  passed when the full suite was rerun with local socket access.
- Migration instructions: [Owned HTTP tracing](web-core.md#owned-http-tracing).
  Consumers have not been edited or newly marked Done. Nothing pushed or deployed.

## Pezzottify owned HTTP tracing canary — 2026-09-26

- Applicable production path: request logging middleware in
  `pezzottify-server/src/server/http_layers/requests_logging.rs`. All enabled
  logging modes now call `web::tracing::trace_with_observer` with its owned
  `Observer` and metadata-only `ResponseInfo`. None mode still bypasses tracing.
  INFO header events, stable target, default terminal events, metrics and
  diagnostics remain unchanged. No shared-library changes were needed.
- Active branch `dev`, clean baseline `9115a8a4`; isolated migration commit
  **`ff7af524`**. Source pin **`ca98a4159e1cb0dd7b9db2faa9a076d198b7973b`**
  recorded in `simple-server.rev`, consumed by existing CI/build scripts.
- Baseline old pin and final new pin: **1,451 passed, 36 existing ignores** each.
  Strengthened production-router HTTP test passed before and after: exactly one
  INFO header event and completion for enabled modes, no events in None mode,
  stable target, safe templates, secret exclusion, unchanged auth response and
  authenticated 404 behavior. Existing streaming/range and process lifecycle
  suites also pass in the full run.
- Strict production Clippy, formatting, database-boundary checks and the
  default-feature debug build pass. Existing test unused-import warnings and
  num-bigint-dig future-compatibility notice remain. Docker, browser, Android and
  release builds were not rerun. Consumer evidence:
  `docs/step-03c-http-tracing.md` and `docs/simple-server-migration.md`.
- Rebased original `dev` onto the migration branch; exact tested tree and
  ancestry verified, original checkout clean. Temporary service worktree,
  migration branch and pinned dependency worktree removed; existing Paravoid
  worktrees/refs preserved. Nothing pushed or deployed.
- Remaining exposure: multipart fields/errors, SSE search producers,
  independent HTTP mocks and error-renderer differential test. Other services'
  tracing adoption remains pending.

## Owned HTTP tracing rollout — 2026-09-27

Reviewed shared library source:
`ca98a4159e1cb0dd7b9db2faa9a076d198b7973b`.
Each consumer uses an isolated branch/worktree; its development branch is rebased
onto the tested migration. No push or deployment is included.

### Crumbles

- Active master baseline `aea6aeb`; migration `a30a324`, documentation correction
  `f080220`. Concurrent Android commit `ea29607` and unrelated local edits preserved.
- Production correlation middleware uses the owned default observer. Integration
  daemon remains N/A (no HTTP tracing). Source pin and README updated.
- Baseline seven correlation tests and production tracing/WebSocket canaries pass.
  Final focused seven plus two pass; full workspace/all-target suite:
  **1,445 passed, two ignored**. Formatting, strict all-target Clippy,
  all-target workspace build and exact-pin helper pass.
- Evidence: consumer `docs/step-03c-http-tracing.md`.
  Remaining: multipart fields/errors only. Tested tree/ancestry verified;
  temporary migration/dependency/documentation worktrees, branches, artifacts and
  logs removed. Existing deployment worktree/ref preserved.

### Fausto

- Clean master baseline `4e1820f`; integrated migration `3a7ae23`.
  Production API middleware uses the owned default observer with identical
  target/severity, correlation, timing and streaming lifecycle.
  README and all nine active GitHub/Forgejo CI pins updated.
- Baseline/final workspace: **664 passed, ten ignored**. Production tracing,
  correlation and lifecycle contracts pass. Socket tests rerun with loopback access.
- Formatting, locked workspace build and warning-mode all-target Clippy pass.
  Previously measured 299 fausto-core strict-lint findings remain; no strict-lint
  pass is claimed. Evidence: consumer `docs/step-03c-http-tracing.md`.
- Remaining: owned multipart fields/errors and axum-test transport/parser oracles.
  Ancestry/tree verified, clean master; temporary worktrees, branch, target and
  logs removed with no retained task scratch.

### Peerlo

- Clean master baseline `5c93e78`; integrated migration `5ac6afd`.
  Custom observer receives `ResponseInfo`; original header severity/latency and
  default terminal events preserved. API crate uses `web` instead of
  `web-compat`; source pin updated.
- Baseline/final workspace: **797 passed, six ignored**. Both production tracing
  contracts pass. Formatting, locked build, warning-mode all-target Clippy,
  pin helper and source audit pass. Existing Clippy warning debt remains;
  strict lint, Docker swarm and live deployment checks were not rerun.
- Evidence: consumer `docs/step-03c-http-tracing.md`. No remaining direct Axum
  API references found in Rust source/manifests. Ancestry/tree verified, clean
  master; temporary worktrees, branch, target and logs removed.

The interrupted session lost uncommitted temporary directories and tracker
drafts; these three completed migrations were recovered from their committed
consumer evidence. LelloStore, Pezzottflix, downloader, SCT and SimpleAI were pending at recovery
and are completed in the records below.

### Pezzottflix

- Clean master baseline `744fea6`; integrated `eb9bfb5`.
  Custom observer uses shared `ResponseInfo`, preserving INFO/WARN headers and
  default terminal events. Source pin and README updated; `web-compat` removed
  and explicit `web` enabled. Source audit finds no remaining backend tracing
  or compatibility exposure.
- Existing production tracing contract passes before/after. Final full suite:
  **601 passed, three ignored**. Initial sandbox run had nine bind EPERM failures;
  exact rerun with socket permission passed. Workspace build and changed-file
  formatting/diff checks pass.
- Global formatting debt and 39 strict Clippy test-target errors (34 library)
  remain; no clean repository-wide lint/format pass is claimed. No frontend,
  release or live deployment qualification was performed.
- Tested tree/ancestry integrated and master clean; temporary migration/source
  worktrees, branch, artifacts and parent directory removed. Existing
  `recovery/pre-owned-websocket-08a0b1d` retained. Remaining exposure: None.

### Pezzottify-downloader

- Clean master baseline `41c31bb`; integrated `32d7ab9`.
  Both parent Puppeteer and child Unix HTTP tracing paths use owned observers
  and `ResponseInfo`. Existing header policy, fields, correlation and body
  lifecycle preserved. Source pin/README updated; `web-compat` removed.
- Baseline/final: **164 Rust tests plus seven doctests passed, one ignored**.
  Owned telemetry contract and both production binary builds pass.
  Socket-dependent tests pass with loopback permission.
- Strict Clippy reproduces twelve baseline errors; repository-wide formatting
  retains existing debt. Changed tracing module is rustfmt-clean.
  Source and feature audits find no compatibility/backend tracing exposure;
  intentional log target filters keep the `simple_server::http_tracing` string.
  Spotify/OAuth, Python cron and release/deployment checks were not rerun.
- Remaining exposure: None. Integrated master is clean; tested tree/ancestry
  verified. Owned service/dependency worktrees, branch, target, logs and parent
  task directory removed; no retained task scratch.

### LelloStore

- Clean master baseline `1097345`; integrated `9b256d5`.
  Production routes use the owned default tracing observer with unchanged
  events, middleware order, correlation and streaming. Active CI and README
  source pins updated to the reviewed shared revision.
- Baseline/final full suite: **196 passed, eight ignored**. Focused production
  tracing contract, formatting, diff check, strict all-target/all-feature locked
  Clippy and all-feature locked build pass. A final sandbox-only loopback bind
  EPERM was resolved by rerunning the suite with socket permission.
- Original ignored frontend/dist copied byte-identically before baseline and
  unchanged. Consumer evidence: `docs/STEP_11_HTTP_TRACING.md`; routing and
  WebSocket documents now omit the completed tracing boundary.
- Remaining: test-only axum-test transport/multipart/WebSocket helpers and
  explicit test router adapters. Original master clean; tested tree/ancestry
  verified. Owned worktrees, branch, target/logs and task directory removed;
  unrelated `/tmp/cr183-release` registration preserved.

### SCT

- Master baseline `2a33b85`; integrated `d4d5318`.
  Owned observer and `ResponseInfo` preserve ErrorCode extensions, exact JSON
  schema/request IDs, bounded route/method/status metrics, histogram and
  header-time active gauge. No subscriber is required. Source pin and README
  updated; explicit `web` replaces unused `web-compat`.
- Focused observability contracts: **three passed**. Baseline/final locked
  workspace: **54 passed, 91 intentionally ignored**. PostgreSQL, S3 and extended
  qualification tests were not rerun. Formatting, strict workspace/all-target/
  all-feature Clippy and workspace/all-target/all-feature build pass.
- Consumer evidence: `docs/step-03c-http-tracing.md` and routing documentation.
  No remaining Axum/backend API exposure found in SCT production source.
  Tested tree/ancestry verified on master; original untracked
  `.validation-work/` and `docs/step-04c-cors.md` preserved. Owned worktrees,
  branch, target/logs and task directory removed.

### SimpleAI

- Master baseline `3eedbd6`; integrated `e12969b`.
  Backend logging uses the owned observer and metadata callback, retaining
  all-status INFO headers and default terminal events. Active source pin and
  README updated; compatibility remains only for the separate admin SSE boundary.
- Baseline/final full workspace: **464 passed, one ignored**. Both focused
  tracing tests pass status/privacy/header/response/lazy stream/completion/
  cancellation contracts. Workspace build and warning-mode all-target Clippy pass.
  Strict lint stops at the same three common-crate derivable_impls findings;
  global formatting retains unrelated debt. Changed Rust/diff checks pass.
- Sandboxed local socket/process failures cleared on an unrestricted full rerun.
  Android/browser/GPU/model-download/Docker/deployed flows were not qualified.
  Consumer evidence: `docs/step-03c-http-tracing.md`.
- Original README semantic edit overlapped the file containing the pin update.
  Clean-worktree integration followed by a README-only preservation stash restored
  the exact semantic diff; all 18 untracked semantic-file checksums match the
  pre-integration snapshot. The owned stash was dropped after verification.
  Original master contains the migration; unrelated dirty/untracked work remains.
- Owned worktrees, migration/recovery branches, build outputs, log and task
  directory removed. Remaining exposure: admin SSE event/keepalive compatibility.

### Rollout completion

All eight remaining applicable services now contain the owned tracing migration
on their established master branches, following the earlier Pezzottify dev canary.
No legacy observer/backend response or compatibility-tracing calls remain in
these nine consumer source trees. Completed tracing items have been removed from
both trackers' remaining-exposure cells. Existing lint/format debt and unexecuted
external-service qualifications are recorded above; no push/deployment performed.

Final coordinator audit verifies migration ancestry and the expected active
branch for all nine applicable consumers (Pezzottify dev, the other eight master),
and finds no legacy tracing callbacks/compatibility calls in their Rust source.
Crumbles has since advanced to concurrent commit `d40ef07` with additional native
authentication/Android edits; these remain untouched and contain the migration.
Tracker JavaScript, local links, seventeen service rows, sixteen completed routing
rows and exact HTML/Markdown remaining-exposure agreement are verified.
All rollout-owned service/dependency worktrees, branches, build outputs and logs
are absent; pre-existing worktree/recovery registrations are preserved.

## Owned SSE API — 2026-09-27

- Library scope only: optional `sse` enables `web::sse` and `web`, without
  `web-compat`. No consumer dependency or production endpoint was changed.
- Owned `Event`, `EventError`, `EventDataWriter`, `KeepAlive` and `Sse` hide
  backend types from public signatures. Existing wire framing, JSON failures,
  validation, headers and keepalive behavior are preserved.
- Streams are demand-driven and accept non-Unpin producers; each event is one
  frame. Errors propagate through the shared body. Dropping the body releases
  its stream; detached producer tasks, auth, replay/cursors and event schemas
  remain application-owned. Keepalive is opt-in and starts at HTTP response
  conversion, requiring a Tokio runtime with time enabled.
- Before implementation: **285 all-feature tests passed**. Final: **297 passed**,
  including **11 SSE tests** and the new API doctest. Minimal `sse` alone:
  **10 SSE tests passed**, without lifecycle/compatibility features.
- Tests compare exact bytes/headers to the pinned backend, cover Unicode,
  multiline and formatted data, comments, JSON, IDs, retry bounds, duplicate
  validation, lazy polling, backpressure, errors and drop cleanup. Paused-time
  checks verify heartbeat defaults/customization, resets and data/error/EOF
  priority. A real loopback HTTP router checks auth rejection, reconnect IDs,
  named JSON events, incremental delivery and disconnect cleanup.
- The complete `bash scripts/check` passes: formatting, strict all-target/
  all-feature Clippy, default/all-feature/minimal-feature tests, no-default
  feature check and warnings-denied rustdoc. Existing extraction/auth rustdoc
  comment warnings were corrected without runtime changes. Tracker links and
  HTML script syntax pass. See [the contract](step-11-sse.md#verification).
- Implemented in isolated `feat/owned-sse` from `main` at `c7f1a7f`; integrated
  locally using the development-branch rebase and worktree cleanup workflow.
- Inspected active SSE consumers: Pezzottify search, SimpleAI admin events and
  simple-agents session events. Their SSE observations remain in the HTML and
  Markdown trackers until production migrations and consumer checks pass.
  Quentin Torrentino remains intentionally skipped. No push or deployment.
- API contract and consumer instructions: [Owned SSE](step-11-sse.md).

## Owned SSE consumer rollout — 2026-09-27

Reviewed library: `96c542c2935606cbae48573e6d5ee634ed24970c`. Active SSE
consumers are Pezzottify search, SimpleAI admin events and simple-agents sessions.
Quentin Torrentino remains intentionally skipped. Application auth, schemas,
producers and replay ownership are preserved.

### Pezzottify canary

- Active `dev`, starting `ff7af524`; integrated migration **`df37837e`**.
  Production search uses owned `web::sse` and direct shared response conversion.
  The capacity-32 channel, tracked task ownership, search phases/section JSON,
  final Done and 15-second heartbeat are unchanged. Existing multipart still
  needs `web-compat`. CI/build pin updated in `simple-server.rev`.
- Baseline full fast-feature suite: **1,451 passed, 36 existing ignores**.
  Eight strengthened production real-HTTP SSE contracts pass before/after,
  including strict auth, no missing-route skips, exact headers/framing, Unicode
  queries, ignored reconnect header and exactly one final Done. Final suite:
  **1,452 passed, same ignores**. Formatting, DB-boundary checks, strict
  production Clippy and default build pass. Existing test unused imports and
  num-bigint-dig future-compatibility notice remain. No Docker/Android/browser
  or release rebuild is claimed. Consumer evidence: `docs/step-11-sse.md`.
- `dev` rebased onto `migration/owned-sse`; tested tree/ancestry verified.
  Original checkout remains clean. Owned branch/worktree/build files removed;
  unrelated Paravoid worktrees/refs preserved. Remaining exposure: multipart
  fields/errors, independent HTTP mocks and the error-renderer differential test.

### SimpleAI

- Active `master`, starting `e12969b`; integrated migration **`f2baca1`**.
  Admin runner events use owned Event/KeepAlive/Sse and direct shared response
  conversion. Existing query JWT/admin auth, four named JSON event types,
  broadcast lag skipping and 15-second heartbeat are unchanged. `web-compat`
  is replaced by `web`. Reviewed pin and active README instructions updated.
- Baseline backend/common: **374 passed, one existing ignored doctest**.
  Two new production admin SSE checks pass before/after: real HTTP uses signed
  test JWTs for 401 invalid token/audience and 403 non-admin, exact headers, all
  four named event JSON schemas including Unicode/newlines, incremental
  delivery and shutdown after disconnect. Paused-time tests preserve heartbeat
  defaults/reset and lagged broadcast recovery. Final: **376 passed, same
  ignore**. Locked full workspace build and non-strict backend/common all-target
  Clippy pass; no new SSE-test findings. Strict Clippy still stops at the three
  unchanged common `derivable_impls` findings. Repository formatting retains
  existing differences; new tests are rustfmt-clean and diff checks pass.
- Full fresh-checkout workspace tests fail before SSE edits because a runner
  test includes ignored deployment `scripts/configs/rtx.toml`. It was not copied
  or committed; no full-workspace test pass is claimed. Docker, Android, GPU,
  browser and deployed OIDC checks were not rerun. Evidence: consumer
  `docs/step-11-sse.md`, routing/tracing and migration documents.
- `master` rebased onto the migration; tested tree/ancestry verified. Only the
  dirty README was temporarily stashed; it was restored and the owned stash
  removed. All **20** original local files hash-verified as preserved, allowing
  the intended source-pin change in README. Semantic-scoring edits and ignored
  deployment configs remain untouched. Owned branch/worktree/build files
  removed. Rust source audit finds no backend/compat imports; remaining exposure:
  **None**.

### simple-agents

- Active `main`, starting `58723d4`; integrated migration **`ffdf45e`**.
  Session events return owned Sse directly and use owned Event/KeepAlive.
  Existing access/revocation checks, sequence IDs/JSON, Last-Event-ID fallback,
  explicit query precedence, 250ms polling, error event then EOF, 15-second
  keepalive and no-store policy are unchanged. Reviewed pin updated; workspace
  `web-compat` replaced by `web`. Source audit finds no backend/compat imports.
- Baseline service: **140 passed**. Three focused SSE checks (one existing, two
  new) pass before/after, including exact ordered event names/IDs/JSON, resume
  and query precedence, malformed cursor/auth rejection and real TCP delivery
  followed by exact error/EOF after revocation. Bytes authorized and sent
  before revocation are consumed before testing revocation; no retraction claim.
  Final service: **142 passed**. Full final Rust workspace: **373 passed, one
  existing ignore**. Formatting, strict all-target workspace Clippy and locked
  workspace build pass. Browser/Android, Docker, runtime-asset packaging and
  production Runner qualification were not rerun. Evidence: `docs/step-11-sse.md`.
- Original dirty checkout was detached without editing its tree, and `main`
  rebased onto the migration in the clean worktree. Original checkout restored
  to `main`; both unrelated `main.rs`/`releases.rs` edits hash-verified unchanged.
  Tested tree/ancestry verified. Owned worktree/branch/build files removed;
  unrelated publication/recovery refs/worktrees preserved. Remaining exposure:
  **None**.

### Final scope

**3 of 3 active SSE consumers adopted**, with Quentin Torrentino still skipped.
Both trackers remove completed SSE items from remaining observations. Isolated
shared-library checkouts, owned logs/snapshots/build files and temporary migration
worktrees/branches are removed after integration. Existing user work is preserved.
Nothing pushed or deployed.


## Axum exposure re-audit — 2026-09-27

Refreshed all 17 last-column observations after owned WebSocket, tracing and SSE
rollouts. **11 services have no remaining direct consumer Axum API/dependency;
5 active services still have specific boundaries; Quentin Torrentino is skipped.**
Of the five active services, four have production boundaries and LelloStore has
test-only boundaries. Completed features are not repeated as remaining work.

### Scope and verification

Inspected local active development branches, branch/tracking history, committed
Rust source (including tests/examples/standalone manifests), all Cargo manifests
and relevant tracked working-tree changes. Broad searches for Axum dependencies,
re-exports, aliases and explicit compatibility bridges were followed by source
inspection of each hit. Reviewed multipart facade definitions so shared
Multipart/Field/Error imports are not mistaken for backend types. No consumer
files, refs or user work were changed. No builds, Cargo dependency-resolution
commands or runtime tests were rerun; this is a source/manifest audit, not a new
runtime qualification. Generated outputs, ignored artifacts and unrelated
untracked work are outside the source snapshot.

The table below records the inspected branch tips. simple-agents advanced during
the audit to its release-preflight commit; its newer committed tree was rechecked
and still has no Axum/compatibility imports. Existing dirty/untracked work in
other services is preserved.

| Service | Active branch | Inspected commit | Remaining Axum exposure |
| --- | --- | --- | --- |
| pezzottify | `dev` | `df37837e` | Production: compatibility Multipart for ingestion uploads (backend fields/errors). Tests: raw Axum media/source/work-knowledge mocks and legacy JSON error-renderer oracle. |
| favzetto | `master` | `d8cadbd9` | None. |
| androidoscopy | `master` | `5f25aa8a` | Production: axum-server TLS configuration, serving and graceful-shutdown handle. Tests: the same TLS adapter. |
| crumbles | `master` | `be920825` | Production: compatibility Multipart for attachment uploads and backend MultipartError mapping. |
| fausto | `master` | `3a7ae237` | Production: compatibility OwnedMultipart for node uploads (backend fields/errors). Tests: axum-test/router adapters and legacy JSON parser oracle. |
| lello-auth | `master` | `e4ce30ed` | None. |
| lellostore | `master` | `9fbf140a` | Tests only: axum-test transports, multipart/WebSocket helpers and into_axum_router adapters. |
| meteonesto | `master` | `b9381ae4` | None. |
| observo | `master` | `7217da13` | None. |
| paranza | `master` | `4739150e` | None. |
| peerlo | `master` | `5ac6afde` | None. |
| pezzottflix | `master` | `eb9bfb5c` | None. |
| pezzottify-downloader | `master` | `32d7ab93` | None. |
| quentin-torrentino | `master` | `63b919f3` | Skipped: backend routing/handlers/extractors/responses, auth/metrics middleware and serving; torrent multipart, chat SSE, dashboard WebSockets, static-file routing and HTTP test fixtures. |
| sct | `master` | `d4d5318c` | None. |
| simple-agents | `main` | `81f5f5eb` | None. |
| simple-ai | `master` | `f2baca18` | None. |

### Exact remaining boundaries

- **Pezzottify:** production ingestion imports `web::compat::Multipart` in
  `pezzottify-server/src/server/ingestion_routes.rs:12`; returned fields/errors
  expose the backend. Test-only direct re-exports remain in
  `pezzottify-server/src/media/tests.rs:40`, source-knowledge mocks
  (`pezzottify-server/src/background_jobs/jobs/source_knowledge.rs:540`), work-knowledge mocks
  (`pezzottify-server/src/background_jobs/jobs/work_knowledge.rs:232`) and the previous JSON
  rejection renderer (`pezzottify-server/src/server/api_error.rs:229`). The compatibility feature
  is still used. SSE, WebSockets and tracing are absent from remaining work.
- **Crumbles:** `crumbles/src/server/routes/attachments.rs:7` imports the
  compatibility Multipart; `:301` explicitly accepts backend MultipartError.
  The workspace compatibility feature is still used. Other Axum text is a
  comment or test name, not additional API exposure.
- **Fausto:** `server/src/api/nodes.rs:16` imports compatibility OwnedMultipart
  for node uploads, exposing backend owned fields/errors. `server/Cargo.toml:85`
  has dev-only `axum-test`; integration tests and
  `server/src/plugin_files.rs:282` convert shared routers with
  `into_axum_router`. `server/tests/routing_contract.rs:139` retains the legacy
  JSON parser oracle. These test dependencies are separate from production
  multipart work.
- **Androidoscopy:** `server/Cargo.toml:26` depends on `axum-server`.
  `server/src/main.rs:197` uses its Handle, `:208` serves TLS through
  `from_tcp_rustls`, and the existing shutdown hook calls graceful_shutdown.
  `server/tests/tls_websocket_integration.rs:4` exercises the same adapter.
  Router/WS types are already shared; TLS transport/configuration/shutdown is
  the remaining production boundary.
- **LelloStore:** all backend Rust production code uses owned shared APIs.
  `backend/Cargo.toml:69` has dev-only `axum-test` with WebSocket support.
  `backend/tests/e2e_auth.rs`, `integration.rs`, `paravoid_distribution.rs`,
  `publications.rs` and `support/paravoid_{device,http}.rs` use its transport,
  upload forms, WebSocket helpers and explicit shared-router adapters.
  Workspace `web-compat` supports these tests; it is not production adoption.
- **Quentin Torrentino:** still intentionally skipped. The public router and
  ordinary handlers/extractors/responses/middleware are backend APIs in
  `crates/server/src/api/routes.rs` and sibling route modules. Torrent uploads
  (`torrents.rs:6`), chat Event/Sse (`chat.rs:14`), dashboard sockets/messages
  (`ws.rs:5`), static file route assembly and HTTP test fixtures remain.
  Existing completed earlier modules are not reclassified.

### What does not count as pending Axum work

Transitive backend packages in lockfiles remain internal to simple-server and
are not consumer API exposure. Shared owned Multipart/Field/Error, WebSocket,
SSE and tracing facades are complete where production uses them. HTTP/Tower
utilities and outbound tungstenite clients do not by themselves expose Axum.
LelloAuth's retained crate name `lello-auth-axum` and its
`axum_middleware` aliases refer to shared APIs, not pending backend integration.
Comment-only references (including Pezzottflix's Query test comment) are omitted.

The shared borrowed/owned multipart wrappers already cover the three active
production multipart consumers: this is consumer migration work, not a missing
module. Androidoscopy still needs a shared TLS abstraction. Test transports,
explicit router bridges and legacy comparison oracles need a separate decision
about replacement before declaring complete source-level Axum removal.

Both trackers have matching pending-only observations. HTML links/script syntax,
17-row consistency and documentation whitespace are checked before commit and
integration into simple-server main; the audit worktree/branch are then removed.
No consumer migration, push or deployment is included.

## Owned HTTP test harness — 2026-09-27

**Library implemented; consumer adoption pending.** Optional `test-harness`
provides `testing::TestServer`, request builders, buffered responses/assertions
and ordered binary multipart forms. Optional `test-harness-ws` adds a real TCP
WebSocket client using owned `web::ws` messages. Neither enables `web-compat`.
See [the API contract](test-harness.md) for bounds, lifecycle and limitations.

Applicability was checked against actual fixtures: Fausto (`master`, `3a7ae23`)
uses axum-test/router adapters for HTTP/auth/upload tests; LelloStore (`master`,
`9fbf140`) uses in-process and HTTP transports, multipart forms and WebSocket
helpers; Pezzottify (`dev`, `df37837e`) uses raw Axum upstream mocks for
media/source/work knowledge. These consumer repositories were inspected only.
Their test boundaries remain in both trackers. Production multipart/TLS and
legacy parser/error comparison oracles are not marked migrated by this addition.

The library work started from clean `main` at `cb25f08` in dedicated branch
`feature/test-harness`, isolated worktree `/tmp/simple-server-test-harness`.
Baseline all-feature suite: **297 passed**. The first sandbox run could not bind
loopback sockets; the permitted rerun passed before implementation began.
Final `bash scripts/check`: **passed**, including formatting, strict all-target
all-feature Clippy, default/all-feature suites, the existing feature matrix,
no-default build and warning-free all-feature documentation. Final all-feature
suite: **316 passed**, including **13 HTTP harness tests**, **5 WebSocket harness
tests** and the new API doctest. Minimal `test-harness`: **12 passed**; minimal
`test-harness-ws`: **17 passed**, demonstrating no compatibility feature is needed.
The multipart parsing contract additionally runs with the production multipart
feature in the all-feature suite.

Contracts cover both transports, JSON/form/query encoding, auth headers, repeated
response headers, explicit cookie behavior, redirect suppression, raw binary
responses, HEAD/404, invalid request metadata, bounded response size/dispatch/body
collection, ordered repeated multipart fields and binary files, TCP external
clients/peer information, listener release on shutdown/drop and graceful-shutdown
deadlines. WebSocket tests cover rejected/authenticated handshakes, protocol
headers, text/binary/ping/pong/close frames, idle deadlines and incoming limits.

Finite responses are buffered (five seconds/8 MiB by default, configurable); SSE
stream tests use an application-owned streaming client against a TCP fixture.
WebSocket rejection body bytes may be incomplete; status/headers are available.
Application tasks/upgraded sockets remain consumer-owned. No TLS abstraction or
consumer integrations were performed here.

Implementation, contract and both trackers are committed together, then integrated
by rebasing original `main` onto the tested worktree branch. Integration is verified
by ancestry and identical tree; the temporary worktree/branch and owned validation
logs are removed. No push or deployment is part of this change.

## Fausto owned HTTP test harness canary — 2026-09-28

**Adopted for HTTP test fixtures; other boundaries remain.** Fausto's active
`master` was clean at `3a7ae23` before work. Its four axum-test integration
suites (`api_tests`, `audit_tests`, `auth_access`, `query_endpoints_tests`) and
plugin-file unit fixture actually exercised the production-owned routers via
`web::compat::into_axum_router`. They now use `simple_server::testing::TestServer`
directly, with explicit fallible request sends, response decoding and header
checks. `axum-test` was removed from the dev dependency and lockfile. The
shared dependency enables optional `test-harness` in dev builds; production
router behavior and dependency features were otherwise unchanged. Active CI
checkout pins and README source requirements reference reviewed library
`d61c049aa49d89de6936de9e93a68bd18685f373`.

Baseline on the unmodified branch: four suites **70 passed**; plugin-file
fixture **1 passed**. After migration: the same **71 passed**. The full locked
Fausto workspace suite passed **664 tests**, zero failures and ten existing
ignored cases. `cargo fmt --all --check`, locked workspace check and warning-mode
server all-target Clippy passed; Clippy reported existing warnings and is not
claimed warning-free. The fixture checks retain auth error/status contracts,
CORS request/response headers, request IDs, CRUD/query responses, static-file
content/cache headers, HEAD behavior and 404s. Full browser/Docker E2E and
live OIDC were not rerun for this test-only migration.

Fausto commit **`83a5ce4`** was made on `migration/owned-test-harness` in
`/home/lelloman/lelloprojects/fausto-harness-canary`; original `master` was
rebased onto it. Ancestry and identical trees were verified. The temporary
worktree and branch were removed; Fausto `master` is clean at `83a5ce4`. No
push or deployment was performed. Remote CI cannot fetch the new shared source
pin until that library revision is published.

**Remaining Fausto exposure:** production `web::compat::OwnedMultipart` for
node uploads still carries backend fields/errors, and the legacy Axum JSON
parser comparison oracle remains test-only. The former axum-test transport and
router-conversion item is removed from the current HTML observation cell.
LelloStore and Pezzottify have not adopted the harness in this canary.

## LelloStore owned HTTP test harness rollout — 2026-09-28

**Adopted; current remaining Axum exposure is None.** LelloStore's active
`master` was clean at `9fbf140`. Actual test use included axum-test in-process
and TCP fixtures, binary multipart APK/VPK uploads, authenticated catalog and
Paravoid WebSocket clients, subprotocol/Ping/Pong/close assertions, and an opt-in
installed-device fixture. The migration uses reviewed simple-server source
`d61c049aa49d89de6936de9e93a68bd18685f373` with dev-only
`test-harness-ws`. Test routers no longer pass through
`web::compat::into_axum_router`; `axum-test` was removed from the dev manifest
and lockfile. The unused production `web-compat` feature was replaced by `web`
to preserve the owned production API. Active CI and README source pins were
updated. The backend source/manifest scan found no remaining direct Axum or
axum-test API use. Shared library internals still use their private backend.

The four affected suites passed **53 tests** with **three existing ignores**
before and after migration. The final locked full backend `--tests` run passed
**200 tests** with **six existing ignores**. Its first run after feature cleanup
timed out in all three real-process lifecycle tests; those same tests passed on
untouched `master`, passed in isolation on the migration branch, and passed in
the final full rerun. `cargo fmt --all --check`, strict offline/default-feature
all-target Clippy (`-D warnings`) and locked offline production backend build
passed. The opt-in device test was compiled but not run; its test fixture allows
100 MiB responses and 120-second operations for real APKs and SDK tools.
Frontend, Android, Docker and all-feature embedded-frontend checks were not
rerun for this test transport migration.

LelloStore commit **`63bbc54`** was created on `migration/owned-test-harness`
in `/home/lelloman/lelloprojects/lellostore-harness-canary`. Original
`master` was rebased onto it; ancestry and identical trees were verified. The
owned worktree and branch were removed, leaving `master` clean at `63bbc54`.
The pre-existing `/tmp/cr183-release/lellostore` worktree was preserved. No push
or deployment was performed. See LelloStore's
`docs/STEP_11_TEST_HARNESS.md` for consumer fixture detail.

The 27 September audit recorded 11 services with no direct Axum exposure, five
active services with pending boundaries and one skipped. This verified adoption
removes LelloStore's test-only item. Current counts are **12 None, 4 active
pending, 1 skipped**; Fausto's separate production multipart/parser items,
Pezzottify, Crumbles and Androidoscopy remain as previously recorded.

## Pezzottify owned HTTP test harness rollout — 2026-09-28

**Adopted for upstream mock fixtures; two distinct boundaries remain.** The
active `dev` branch was clean at `df37837e`. The media tests serve image and
progressive audio responses; source-knowledge and work-knowledge tests serve
Wikidata/MusicBrainz search, fact and failure fixtures over loopback HTTP. Their
seven raw Axum mock listeners now use owned `web::Router`, handlers, extractors,
responses and `testing::TestServer::tcp`. The fixtures retain the same URLs,
status, JSON query assertions, streaming byte release and call-count behavior;
`TestServer` owns listener cleanup. Optional `test-harness` is enabled only for
dev builds. `simple-server.rev`, used by the existing checkout workflows, now
pins reviewed library source `d61c049aa49d89de6936de9e93a68bd18685f373`.
The lockfile update was limited to `reqwest`, `tower-http` and their necessary
transitive changes (including `cookie_store`, `hyper` and `hyper-util`).

Before migration the affected media, source-knowledge and work-knowledge suites
passed **18 tests** with one existing live-API ignore. After migration the same
suites passed **18**, one ignored. The final locked offline full library run
passed **1,120 tests**, zero failures and two existing ignores. Locked offline
production library/binary check, `cargo fmt --all --check` and the Git
diff whitespace check passed. Live API, Docker/browser E2E and frontend builds were not run
for these test-only fixtures. The production ingestion handler still imports
`web::compat::Multipart` for backend field/error behavior. The test-only prior
JSON error-renderer comparison in `server/api_error.rs` deliberately retains
raw Axum; neither boundary is claimed migrated.

Pezzottify commit **`3051b5f8`** was made on `migration/owned-test-harness` in
`/home/lelloman/lelloprojects/pezzottify-harness-canary`. Original `dev` was
rebased onto that tested branch; ancestry, identical tree and clean status were
verified. The owned worktree and branch were removed; unrelated pre-existing
worktrees were preserved. No push or deployment was performed. The current HTML
remaining-exposure cell now lists only production compatibility Multipart and
the legacy JSON error oracle; its former raw upstream-mock item is removed.
The 27 September audit table above remains a dated snapshot. Current totals
stay **12 None, 4 active pending, 1 skipped**.

## Pezzottify direct Axum cleanup — 2026-09-28

**Complete for consumer source and manifests.** Pezzottify's active `dev` branch
was clean at `3051b5f8`. Its ingestion upload handler actually used
`web::compat::Multipart`; `upload_file` consumes borrowed field metadata and
bytes, preserves invalid/missing filename and empty-data responses, and applies
the route's larger body limit. It now uses the existing owned
`web::multipart::Multipart`, whose fields and errors are simple-server types.
The unused `web-compat` feature was removed from the production dependency.
The test-only JSON error comparison now uses owned `web::Json` and `web::body`
while retaining status, headers, body-byte comparison and escaped-text checks.
The active README was updated. The reviewed shared library source remains
`d61c049aa49d89de6936de9e93a68bd18685f373` in `simple-server.rev`; no
shared-library extension or pin change was required. A full Rust source and
manifest scan found no remaining direct `axum`, `simple_server::axum`,
`web::compat`, `web-compat` or `axum-test` usage. Axum remains internal to
simple-server.

Before and after the change, the buffered JSON contract test, actual HTTP
upload authentication/field-error test, and actual HTTP large multipart/body
limit test each passed. The final locked offline full library suite passed
**1,120 tests** with **two existing ignores**; locked offline production
library/binary check, `cargo fmt --all --check` and whitespace checks passed.
The HTTP test builds still report a pre-existing unused `SocketAddr` import in
the common fixture. Other Docker/browser and live external-service E2E suites
were not rerun for this extractor and test-only API migration.

Consumer commit **`3d0ef0d8`** was made on `migration/owned-multipart-cleanup`
in isolated worktree `/home/lelloman/lelloprojects/pezzottify-multipart-cleanup`.
Original `dev` was rebased onto the tested branch; ancestry, identical tree and
clean status were verified. The temporary worktree and branch were removed,
leaving `dev` at `3d0ef0d8`. Unrelated worktrees were preserved. No push or
deployment was performed. The current HTML and summary-table Pezzottify
remaining-exposure cells are now **None**. Across the 17 tracked services, the
current count is **13 None, 3 active pending, 1 skipped**; the dated 27 September
audit table above remains unchanged.

## Fausto and Crumbles owned multipart cleanup — 2026-09-28

**No shared-library extension was needed.** The existing
`web::multipart::{Multipart, OwnedMultipart, MultipartError}` APIs expose
borrowed or owned fields, metadata, streaming chunks, `bytes`/`text`, parser
error text and status without consumer-visible backend types. Crumbles uses the
borrowed reader and maps the shared error's `status()` to its existing 413 or
bad-request application response. Fausto uses the owned reader, preserving its
metadata JSON, binary content and parser-error handling. The reviewed shared
sources were already pinned: Crumbles `simple-server.rev` at
`ca98a4159e1cb0dd7b9db2faa9a076d198b7973b` and Fausto active CI/README
at `d61c049aa49d89de6936de9e93a68bd18685f373`. The multipart module is
identical at those revisions. Neither pin needed changing.

**Fausto:** active `master` was clean at `83a5ce4`. The production node upload
now imports `web::multipart::OwnedMultipart`; the unused server `web-compat`
feature was removed. The remaining test-only raw Axum JSON-parser comparison
was replaced with an owned `web::Json` rejection comparison while retaining the
422 and validation-detail assertions. A new endpoint test creates a node with
binary content, reads the exact bytes back and verifies missing/malformed
metadata errors. Baseline API suite: **29 passed**; baseline JSON parser test:
**one passed**. The final locked offline workspace suite passed **665 tests**
with **ten existing ignores**; formatting and source/manifest scans passed.
The local migration commit **`1947a29`** was made in an isolated worktree,
rebased into original `master`, verified by ancestry and identical tree, then
the temporary worktree/branch were removed. Live external services and browser
E2E were not rerun.

**Crumbles:** active `master` started at `91d57f8` with unrelated tracked and
untracked Android design/client work in the original checkout. Production
attachment uploads now use `web::multipart::Multipart`; their error mapper
accepts shared `MultipartError` and preserves 413 versus malformed/interrupted
400 behavior. The unused workspace `web-compat` feature was replaced with
`web`. The application retains its configured total-wire and per-file limits,
chunked hashing, MIME validation, atomic persistence, audit and event rules.
The attachment suite passed **24 tests** before and after migration. The final
locked offline Rust workspace suite passed **1,449 tests** with **two existing
ignores**; strict workspace/all-target Clippy, formatting and source/manifest
scans passed. The unchanged ignored frontend `dist` was copied into the
isolated worktree for Rust embedding during checks. Web/browser, Android and
container suites were not rerun. The local migration commit **`a18ff96`** was
made on its isolated worktree branch. Original `master` was advanced from the
clean worktree while the dirty original checkout was temporarily detached;
ancestry and identical committed tree were verified. The original checkout was
restored to `master` with its Android changes and untracked files preserved.
Only the owned temporary worktree/branch were removed. Neither consumer was
pushed or deployed.

The current HTML observations and Markdown summary cells are now **None** for
Fausto and Crumbles. The dated 27 September audit table remains a snapshot.
At this Fausto/Crumbles checkpoint the totals were **15 None, 1 active pending
(Androidoscopy TLS), 1 skipped (Quentin Torrentino)**. Axum remains internal to
simple-server.

## Androidoscopy owned TLS cleanup — 2026-09-28

Androidoscopy `server` previously used `axum-server` directly for the legacy
app WSS listener, certificate loading and graceful-shutdown handle, including
its verified TLS registration test. Shared commit `e34c6d6` adds optional
`web::tls` with owned `TlsConfig` PEM loaders and an already-bound listener
server. It observes the same `Shutdown` as plain HTTP and initiates an unbounded
graceful drain; Androidoscopy's lifecycle retains its 30-second deadline and
application-owned WebSocket/task drain. Certificate generation, TLS fallback to
plain WS if certificate setup fails, the LAN pairing TLS protocol and trust
policy remain unchanged.

Baseline TLS registration passed (1 test). Shared checks passed: TLS feature
compile, strict library Clippy, two TLS contract tests (invalid PEM and
pre-requested shutdown). Androidoscopy's final locked offline server suite and
verified TLS WebSocket registration passed; the TLS test also bounds server
termination after requesting shutdown. Changed Rust files pass targeted
formatting; `git diff --check` passes. Server source, tests and manifest contain
no direct `axum`/`axum-server` references. The resolved dependency graph shows
`axum-server` only through `simple-server`. Existing dead-code warnings remain;
Android/SDK and separate E2E suites were not rerun.

Consumer commit `6428fab` is integrated into clean `master` from its isolated
worktree branch; the reviewed shared revision is `e34c6d6` in the active
`simple-server.rev` and README. Neither repository was pushed or deployed.
The current totals are **16 None, 0 active pending, 1 skipped (Quentin
Torrentino)**. The dated audit above remains a historical snapshot.

## Final public API audit and compatibility cleanup — 2026-09-28

The active 16 consumers have no direct Axum source/manifest use, but the full
abstraction criterion is not yet met. Quentin Torrentino remains explicitly
skipped and still imports `simple_server::axum` in its production server and
HTTP tests. Its main server also calls the backend-facing `http::serve`.
Removing either API now would break that checkout. The re-export and legacy
serving entry point therefore remain pending Quentin's migration or a separate
compatibility decision; this is not marked complete.

Independent shared API cleanup removes unused `web-compat` and its backend
router, multipart, WebSocket and tracing escape hatches. No active or skipped
consumer enables that feature. The old public tracing module is private behind
`web::tracing`, correlation's public `run` now accepts standard HTTP bodies and
its backend middleware adapter is removed, and `BodyLimit` exposes an owned
Tower service type. Library tests use direct backend imports only as internal
comparison oracles; the lifecycle example uses owned `web` routing and serving.
All-feature tests, strict all-target Clippy, formatting and minimal feature
checks pass on the isolated branch. After integration, locked offline production
checks pass for Fausto (`fausto-server`) and Crumbles (`crumbles`), exercising
the tracing, correlation and body-limit consumers. The original development
branches and working trees in those services were not changed. The
full final API removal still awaits the Quentin decision and a final audit of
the remaining legacy entry points. No consumer was changed in this cleanup.

## In-scope public Axum removal — 2026-09-28

The final read-only source/manifest audit of the 16 migrated repositories found
no direct `axum::`, `simple_server::axum`, `axum =` or `axum-server =` use in
Rust production code or tests. The `lello-auth-axum` crate name is its local
package name, not an Axum dependency. The `simple_server::axum` re-export and
public backend-facing `http::serve` have now been removed. The two legacy Axum
trait implementations on public extraction types were also removed; the owned
handler contract retains their extraction and rejection behavior. The transport
serving helper remains crate-private beneath `web::serve` and
`web::serve_with_connect_info`. The lifecycle example, serving tests and README
use owned APIs. Internal library comparison tests may still import Axum directly
as a backend oracle. The complete `scripts/check` matrix passed with offline
dependencies and loopback access: formatting, strict all-target/all-feature
Clippy, default/all-feature/minimal-feature tests and doctests, and rustdoc with
warnings denied. The rendered public docs contain `web::serve` and `http::bind`
but no Axum module, `http::serve` page or compatibility module. Owned extraction
tests cover the removed backend trait implementations' rejection behavior.

Quentin Torrentino remains out of scope at the user's request. Its current
checkout imports the removed re-export and calls the removed public serving
function, so it **will not build against this new shared revision**. No Quentin
files, branches or pins were changed. The in-scope count remains **16 None, 1
excluded**, and Step 07 can be considered next for those 16 services.

## Step 07a SQLite connection policy — 2026-09-28

`simple-server` commits `b831dc1` and `3219ae8` add the optional,
driver-independent `database-sqlite` feature. Applications explicitly choose
foreign keys, busy timeout, journal mode, synchronous mode and WAL
auto-checkpointing. The shared policy emits commands for each connection and
verifies effective values supplied by the driver; it does not bring in SQLx,
`rusqlite`, a pool, an ORM, migrations or a backup implementation. This avoids
the incompatible SQLite C-binding versions in the current SQLx and `rusqlite`
consumers. Fixed-width SQLite settings reject out-of-range values. The complete
shared `scripts/check` matrix passes with offline dependencies and loopback
access, including all-feature strict Clippy, feature-specific tests and rustdoc.

| Consumer | Active branch and commit | Production adoption | Verification and limits |
| --- | --- | --- | --- |
| Favzetto | `master` / `e5fdafd` | The existing one-connection SQLx SQLite pool runs shared foreign-key and WAL commands on each new connection. Readiness retains `SELECT 1` and now verifies the live settings. The app still owns its pool and migration ledger. | Baseline three database tests passed. Final four database tests passed, including WAL/foreign-key drift; all 135 library tests passed. The ignored frontend `web/dist` assets were copied unchanged for worktree compilation and removed with that worktree. Existing unrelated compiler warnings remain. |
| Pezzottify | `dev` / `715e31d4` | Existing `rusqlite` connections apply shared foreign-key, two-second busy-timeout and NORMAL synchronous policy. The backup registry still chooses WAL and disables auto-checkpoints through a separate explicit shared policy. Stores, migration/schema validation, checkpoint execution and the priority-aware executor remain application-owned. | Baseline two connection tests passed. Final connection tests 2/2, backup tests 4/4, full library tests 1,120 passed with two existing ignores, and production binaries checked. Existing `num-bigint-dig` future-compatibility warning remains. |

Both development branches were advanced onto their tested migration branches;
the temporary consumer branches and worktrees were removed. Pezzottify's
concurrent uncommitted Android files and `docs/authentication.md` remained
untouched in its original checkout. No push or deployment. The remaining 15
Step 07a rows were subsequently assessed in the record below.

## Step 07a applicability audit and 07b design — 2026-09-28

Read-only inspection of all 15 remaining local development branches (14
`master`, Simple Agents `main`) checked production connection constructors and
SQLite settings. No consumer source, branch or dependency was changed, and no
consumer builds or runtime checks were run. **Pending** means a real setting is
used but shared 07a is not adopted; **N/A** means the Rust policy has no current
production setting to replace. The two Done pilots and their tests are recorded
above. Quentin remains Pending and excluded at user request.

| Repository | 07a assessment and source evidence |
| --- | --- |
| Androidoscopy | N/A: no SQLite database in the server or Cargo manifests; `lan::Connection` is a network connection, not a database. |
| Crumbles | Pending: `crates/crumbles-core/src/db/pool.rs` sets WAL, foreign keys and a bounded busy timeout; `crumbles-integration/src/db.rs` has a separate SQLite pool, foreign keys and timeout. Both need component-specific verification. |
| Fausto | Pending: `core/src/storage/sqlite/mod.rs` sets foreign keys and WAL on opened `rusqlite` connections. |
| Lello Auth | Pending: `crates/lello-auth-core/src/db/mod.rs` configures foreign keys, WAL and a five-second busy timeout for SQLite; its PostgreSQL path is outside 07a. |
| LelloStore | N/A: `backend/src/db/mod.rs::init_pool` configures pool size/path but no selected SQLite PRAGMA; SQLx driver defaults are not an application policy to migrate. |
| Meteonesto | Pending: `weather-pipeline/src/database.rs` has distinct reader/writer policies with live PRAGMA checks; `publication.rs` also sets busy timeout, foreign keys and FULL synchronous mode. Its API/gateway do not use this SQLite path. |
| Observo | Pending: `observo-server/src/db.rs` sets busy timeout on connections and WAL during initialization; adoption must preserve that lifecycle distinction. |
| Paranza | N/A: `crates/paranza-store/src/lib.rs` opens SQLite and runs schema changes, but selects no 07a connection settings; its `PRAGMA table_info` is schema inspection. |
| Peerlo | N/A: `crates/peerlo-tracker/src/store.rs` and `crates/peerlo-metadata/src/store.rs` open SQLite with driver defaults and initialize schema; no explicit 07a policy found. |
| Pezzottflix | Pending: `pezzottflix-server/src/db/connection.rs` selects foreign keys, busy timeout, WAL/NORMAL or DELETE by configuration. The `raiplay-cli` SQLite databases also set WAL and need separate scope review. |
| Pezzottify Downloader | N/A for this Rust capability: `scripts/cron_downloader.py::open_db` uses Python `sqlite3` and sets a row factory/schema, but no SQLite PRAGMA; its Rust HTTP binaries do not open SQLite. The Python quota database remains application-owned. |
| Quentin Torrentino | Pending, excluded from migration at user request; no adoption claim or branch change. |
| SCT | Pending for `crates/sct-archive/src/catalog.rs`: its offline SQLite catalog selects foreign keys, DELETE journal, FULL synchronous and additional cache/temp-store settings. 07a can represent the first three only; the others must remain application-owned. `sct-core` server catalog is PostgreSQL and N/A for 07a. |
| Simple Agents | Pending: `simple-agents-service/src/db.rs`, `simple-agents-runner/src/lib.rs`, runtime and coding database modules select differing SQLite PRAGMAs. Track these components separately during adoption. |
| Simple AI | N/A: `backend/src/audit/sqlite.rs` opens a `rusqlite` audit database and creates/updates schema, but selects no 07a connection settings; the inference runner has no SQLite path. |

The [07b design](step-07b-database-migrations.md) specifies a driver-neutral,
read-only preflight planner over existing ledger observations. It does not yet
add code or mark any migration runner adopted. The survey now identifies SCT's
separate offline SQLite archive rather than treating the whole repository as
PostgreSQL-only. No push or deployment was performed.

## Step 07b shared migration preflight — 2026-09-28

The optional `database-migrations` feature exposes
`database::migrations::MigrationPlan::inspect`. Applications pass an ordered
manifest, their existing ledger observations and an optional separate
high-water mark. Numeric and lexicographically ordered text versions are
supported without a database driver. The report contains the matched and
pending entries plus the count of historical digest comparisons. Errors
distinguish dirty state, duplicate/out-of-order versions, gaps, unknown or
newer versions, renamed or changed migrations, missing historical digests and
high-water disagreement. No SQL is run, ledger created, schema shape inferred
or legacy state adopted. Existing runners retain execution and transaction
ownership. The 07a and 07b features work independently.

Eight contract tests cover fresh, partial and complete histories; a version-only
ledger; text versions; invalid manifest/ledger order; gaps and future state;
dirty, renamed and changed entries; identity and high-water mismatches.
The standalone no-default-feature 07b test passes. The full `bash scripts/check`
matrix passes, including all-feature strict Clippy, all-feature tests,
feature-isolated tests and strict rustdoc. Its first sandboxed run stopped at
loopback HTTP tests because socket binding was denied; the permitted rerun
passed. The existing `web_sse` feature-only unused-import warning remains.

At this shared implementation checkpoint the HTML and Markdown matrices were
synchronized at **0 Done, 15 Pending, 2 N/A** for 07b. Androidoscopy has no database; the downloader's SQLite cron
is Python-only, so neither has a Rust migration runner to adopt this helper.
Quentin remains excluded at user request. All other rows remain Pending until
their real migration paths and ledger compatibility are assessed; the new Cargo
feature alone is not adoption. The next step is a consumer canary that preserves
the current runner's ledger, execution and failure behavior. No consumer code,
branch or pin changed, and nothing was pushed or deployed.

## Step 07b Simple Agents canary — 2026-09-28

Simple Agents `main` commit `25b2ea44cf077661bad70b6aa7dbfb73e14ce255`
uses shared source `255881ae9926bbb9ce8f4c855d492ccfd1ffbd1d` in its
production service database migration path. The reviewed revision is pinned in
`simple-server.rev` and the workspace enables `database-migrations`. The
service builds its manifest with the same historical SHA-256 checksum algorithm,
reads its existing `service_migration` ledger, and uses the shared preflight
report to choose the pending suffix. It retains its strict unmanaged-schema
check, ledger table, `BEGIN IMMEDIATE` per-migration transaction, dirty marker,
SQL execution and `InvalidLedger(version)` error contract.

Baseline service migration tests passed 2/2. Final migration tests passed 4/4,
including older-schema upgrade, idempotence, ciphertext/ownership retention,
dirty/renamed/changed/gapped/future history rejection before later SQL, and
rollback plus retry after a failed migration. The locked Rust workspace suite
passed with serial test execution; workspace all-target check, strict Clippy,
formatting and diff checks passed. An initial parallel workspace run had one
timing-sensitive distributed test fail because slow maintenance starved permits;
that test passed alone and in the serial full rerun. Browser, Android,
worker-image and container qualifications were not repeated for this SQLite
change.

This is **Partial** repository adoption: the Runner transport database has a
`schema_version` singleton, not a ledger of historical names and checksums.
Its upgrades still use their existing transaction and version checks. Runtime
and coding schema are installed within that Runner transaction. The current
07b API requires names for recorded entries, so applying it to the Runner
would need a version-only contract or another explicit parity proof, not
fabricated historical names. No other Simple Agents component is marked
adopted. Its original `main` was rebased onto the tested worktree branch;
the temporary branch, worktree and dependency link were removed. The original
checkout remains clean. Nothing was pushed or deployed.

## Step 07b Meteonesto pipeline — 2026-09-28

Meteonesto `master` commit `bda1be88660d91195a5b0849ea4e0aa94ba740e8`
opts its production `weather-pipeline` writer into `database-migrations` at
reviewed shared source `0f92b9d15c11debe41c649a82a5e26c6b695c7bd`.
The application reads its existing `schema_migrations` ledger and SQLite
`user_version`, computes its historical SHA-256 checksums, and asks shared
`MigrationPlan::inspect` for the pending suffix. Its existing validation runs
first to preserve `NewerSchema`, `Integrity`, `UnknownMigration`,
`MigrationName` and `MigrationChecksum` errors, including their order and
details. The application still owns migration SQL, a single transaction for
the pending set, foreign-key checks, backups and rollback policy. Weather API
and gateway have no separate schema migration runner, so this is **Done in
the applicable pipeline scope**.

Baseline database integration tests passed 10/10. A new duplicate embedded
version test failed on the old implementation and passed once shared preflight
was wired in; it confirms rejection before schema SQL. Final database
integration tests passed 11/11 and four focused library database tests passed.
The tests cover fresh startup, older schema upgrade and retained rows,
checksum/name drift, newer schema, `user_version` disagreement, failed migration
rollback and reader isolation. The complete `weather-pipeline/scripts/check`
passed with formatting, strict all-feature Clippy, dependency policy,
all-target Rust tests, Python unit tests and a release build. Its first
sandboxed run stopped when `cargo deny` could not lock its advisory database;
the permitted rerun passed, with existing unmatched-source and duplicate-`syn`
warnings. Provider, container, Android, API/gateway and deployment checks were
not repeated for this database-local change.

Meteonesto's original `master` was rebased onto the tested migration branch;
ancestry and tree match, and the temporary worktree, branch and dependency
link were removed. The original checkout is clean. No push or deployment.

## Step 07b Crumbles integration runner — 2026-09-28

Crumbles `master` commit `d7f19411d829d2d0d32d4a6d061fc6f3ef11a8e1`
uses shared source `b522640c3dc967a5f6cd450d8fd39967388f094b` in
the production `crumbles-integration` runner database. Its `db::open` and
staged-upgrade validation both call the migrated path. The application reads
its existing `runner_migration` ledger, supplies numeric versions, names,
historical SHA-256 checksums and dirty state to `MigrationPlan::inspect`, and
uses the report's applied count to select the pending suffix. The existing
ledger format, unmanaged-schema guard, `InvalidLedger(version)` error,
per-migration SQLx transactions, dirty-row updates and rollback behavior remain
application-owned. The optional feature is enabled only for
`crumbles-integration`. The main Crumbles database's separate SQLx migration
policy is unchanged, so repository status is **Partial**.

Baseline: eight focused integration database tests passed. After adoption, the
full integration package passed 96 library tests, two binary tests, ten
integration tests and doc tests. A new duplicate-manifest test then passed with
the focused database suite (nine tests), confirming rejection before schema
SQL. Existing tests cover fresh open/restart, staged upgrade and rollback,
dirty, changed-name/checksum, missing and future ledger entries, unmanaged
schema and failed migration rollback. `cargo fmt --check`, strict all-target
package Clippy and `git diff --check` passed. The broader Crumbles workspace,
frontend and deployment checks were not repeated for this scoped database
change.

The change was committed in an isolated worktree from `master` at `4580425`.
The original `master` was rebased onto the tested branch and has the same tip
and tree; the temporary branch, worktree and dependency link were removed.
The original checkout's modified `ANDROID_CLIENT_PLAN.md` and five untracked
Android files/directories were preserved. No push or deployment.

## Step 07b Fausto SQLite storage — 2026-09-29

Fausto `master` commit `34326d2ea356f07f72ae8b3de59658b338709a82`
uses shared source `e815c1b8ccf6e4d64fb3f950ba0ca1b88e4edf5b` in its
production `fausto-core` SQLite store, reached by normal server startup. The
store first applies its existing legacy unversioned-database adoption rule,
then reads the `schema_version` version/name ledger and compares it with its
embedded manifest through `MigrationPlan::inspect`. It uses the report to
select the pending suffix. This ledger has no stored digests, so both inputs
use `None` and the integration does not claim checksum verification. Fausto
still owns foreign-key enforcement, the ledger table, migration SQL, and the
transactional or nontransactional boundary selected by each migration.
No other Fausto database migration runner was found, so 07b is **Done** for
its applicable SQLite store; 07a connection policy remains Pending.

Baseline: 431 core library tests passed. After adoption, 432 core tests passed,
including a new test that rejects renamed, missing and future ledger entries.
The 202 SQLite storage tests passed; existing tests also exercise fresh schema,
legacy unversioned adoption and idempotent restart. `cargo fmt --check`, locked
workspace all-target check and diff checks passed. Workspace check emitted two
existing server test-import warnings. Strict all-target core Clippy failed on
352 warnings across the crate; a library-only run with pedantic lints disabled
still found 12 warnings outside the changed migration code. The server, plugin,
frontend and deployment suites were not rerun for this storage-only change.

The change was committed in an isolated worktree from `master` at `1947a29`.
The original `master` was rebased onto that tested branch; ancestry and trees
match, the original checkout is clean, and the temporary worktree, branch and
dependency link were removed. No push or deployment.

## Step 07b Favzetto backend — 2026-09-29

Favzetto `master` commit `22cc24e659f2b0fcb870c1770b92256710fecb19`
uses reviewed simple-server source `da1229723b2378269ca6cf02d0cd568cc608ba07`
in production `Database::migrate`. Both configured startup migration and the
`migrate` command call this path. The backend discovers its sorted SQL files,
reads its existing `schema_migrations` ledger, and gives the complete filename
as both version and name to `MigrationPlan::inspect`. The ledger stores no
historical digests, so both inputs use no digest and this adoption claims no
checksum verification. The planner selects the pending suffix; Favzetto
retains its per-file SQLx transactions, per-file applied checks, ledger inserts
and post-migration legacy schema repairs. Missing history and unknown future
versions now fail closed before pending SQL. This is **Done** for Favzetto's
applicable backend database; its independent 07a policy remains Done.

Baseline: four focused database tests passed after copying the original
checkout's ignored `web/dist` into the isolated worktree for compile-time
embedding. Final focused database tests passed 6/6, including valid older
schema upgrade with retained rows and idempotence, and rejection of gapped or
future ledger entries before pending SQL. The final full backend run passed
137 library tests, 102 API tests, two lifecycle tests and two logging tests
with two unrelated API tests skipped. Those catalog-research runtime-bridge
tests failed identically on the untouched `master` checkout and migration
branch: they attempted transitions from a completed flow and received HTTP
400 rather than the expected 200. Warning-mode all-target Clippy, targeted
database-file formatting and diff checks passed. Repository-wide formatting
fails on unrelated files in both branches. Frontend and deployment checks
were not repeated.

The change was committed in an isolated worktree from `master` at `e5fdafd`.
The original `master` was rebased onto the tested branch; ancestry and trees
match, the original checkout is clean, and the temporary worktree, branch,
generated-asset copy and dependency link were removed. No push or deployment.

## Step 07b completion pass — 2026-09-29

Shared source `c1ff3d19d685cd267f5bc712b306f44ef4678d07` adds
`MigrationPlan::inspect_version_only` for stores with only a latest-version
marker. It reports a pending suffix, rejects unknown/future markers and always
reports zero historically verified digests. It does not create a ledger or
infer schema health. The focused no-default-feature suite passed 9 tests;
formatting and strict all-target minimal-feature Clippy passed. The shared
`master` branch was rebased onto the tested worktree branch; matching ancestry
and tree were verified, and the temporary branch/worktree were removed.

| Service / active branch | Result and actual production boundary | Verification / integrated commit |
| --- | --- | --- |
| LelloStore `master` | **Done.** Backend SQLx manifest and `_sqlx_migrations` rows pass through shared preflight before SQLx runs; recorded names, checksums, success and order are checked. | Baseline 8 database tests; final 94 library tests and strict all-target Clippy pass. A changed-checksum regression fails before SQLx. `0a5e2bf`. |
| Simple Agents `master` | **Done.** Existing service ledger keeps its full-history preflight. Runner transport now checks its singleton schema marker through `inspect_version_only` before its existing transactional upgrades; runtime/coding tables belong to that transaction. | Runner baseline/final 18 library and one state integration test pass; strict all-target Runner Clippy and formatting pass. `f99e0ce`, active pin/notes `dfbe48d`. |
| Lello Auth `master` | **Done.** SQLite and PostgreSQL initialization check their existing latest schema version before running application SQL. Neither version table records names or digests; no historical checksum claim is made. | Baseline open test and final 561 core tests pass; a future SQLite marker fails before upgrade. Normal all-target Clippy passes with three existing warnings in unrelated modules; PostgreSQL runtime tests need an isolated fixture and were not run. `fda44d0`. |
| Pezzottflix `master` | **Done.** Production server SQLx migrations inspect the existing ledger, then SQLx remains the runner. | Baseline four schema tests; final five schema tests and full library 547 passed, one ignored. A checksum-drift regression passes. Changed file formatted; normal library Clippy passes with existing warnings, while repository-wide formatting has existing drift. `0895b0a`, active pin/notes `00d13b3`. |
| SCT `master` | **Done for PostgreSQL catalog.** Embedded SQLx manifest and PostgreSQL ledger pass through preflight. The offline SQLite archive has a fixed format marker, not a migration ledger. | Nine core library tests pass, one ignored. New checksum-drift and existing migration/rollback tests pass against an isolated PostgreSQL 17 container; container removed. Formatting and strict all-target core Clippy pass. `b16ffe3`, active pin/notes `cdb1fff`. |
| Crumbles `master` | **Done.** Main core SQLx ledger joins the previously adopted integration-runner ledger. Core preflight runs after application-owned legacy baseline adoption and ledger-window checks; its existing checksum error contract is preserved. | Baseline ledger failure test passes; final 12 core migration tests and 679 core library tests pass, with strict all-target core Clippy and formatting. A renamed recorded entry fails without ledger writes. `83b0b2d`. |
| Pezzottify `dev` | **Done for five versioned stores.** User, server, catalog, enrichment and download queue use version-only preflight before their existing upgrades. Schema validation, catalog legacy-column classification, SQL execution and backup policy stay local. Ingestion's column-driven changes and search-index rebuild state have no ordered migration ledger and are outside 07b's preflight scope. | Baseline 1,120 passed, two ignored; final 1,121 passed, two ignored. A future catalog marker fails without writing. Formatting and strict production lib/bin Clippy pass; all-target strict Clippy finds existing test warnings. `44fa2235`. |

Four remaining service assessments are **N/A for the current 07b
contract**, with no consumer code changes: Observo `master` `7217da1` runs one
idempotent `SCHEMA` batch without a migration ledger; Paranza `master`
`4739150` runs `CREATE TABLE IF NOT EXISTS` and checks column presence before
two `ALTER TABLE` repairs without a version ledger; Peerlo `master` `5ac6afd`
creates tracker/metadata tables when absent without ordered migration history;
SimpleAI `master` `f2baca1` runs idempotent table creation and column additions
without a ledger. Adding a ledger or retroactively naming those changes would
be a separate behavior change, not adoption of this read-only planner. No
runtime result is claimed for these source-only assessments. Androidoscopy
and the Python-only downloader remain N/A; Quentin Torrentino remains Pending
because it is excluded by request.

All consumer changes above were committed in isolated worktree branches,
their active `master` or `dev` branches were rebased onto those branches, and
ancestry/identical tips were verified before the temporary branches and
worktrees were removed. Existing Crumbles and LelloStore Android edits, Lello
Auth research files, and SCT qualification files were preserved. Active
`simple-server.rev` pins and build instructions were updated where present.
No push or deployment was performed. Final matrix: **10 Done, 0 Partial,
1 Pending, 6 N/A**.

## Step 07a completion pass — 2026-09-29

The optional, driver-independent SQLite connection policy at reviewed
`simple-server` revision `99444c0eed1b8860ec2e5705e48ac799d7633c7f`
is now used by every service that selected an applicable production connection
setting. Each consumer continues to own its driver, pool, database files,
schema, transactions and backup behavior. The [earlier applicability audit](#step-07a-applicability-audit-and-07b-design--2026-09-28)
records the pre-adoption state; this pass resolves its eight applicable Pending
rows. Favzetto and Pezzottify were already Done.

| Service / active branch | Production adoption and preserved behavior | Verification / integrated commit |
| --- | --- | --- |
| Crumbles `master` | Core SQLx pool uses WAL for files, foreign keys and its configured timeout; the integration daemon's main pool and read-only staging/admission connections use their existing foreign-key and timeout choices. In-memory core keeps its effective memory journal. | 679 core tests and nine focused integration database tests pass; formatting and strict all-target Clippy pass for affected packages. `cc20137`. |
| Fausto `master` | Core file SQLite uses foreign keys and WAL; in-memory SQLite uses foreign keys without requesting WAL. | 433 core library tests and formatting pass. Strict all-target/core Clippy still reports pre-existing warnings/errors. `0c1b1c6`. |
| Lello Auth `master` | Each new r2d2 SQLite connection gets foreign keys and a five-second timeout; file databases also request WAL, while shared memory retains its memory journal. PostgreSQL is outside 07a. | 562 core library tests and changed-file formatting pass; ordinary library Clippy completes with three existing warnings. `7428b44`. |
| Meteonesto `master` | Weather-pipeline writer, reader and publication connections adopt their selected foreign-key, busy-timeout, WAL, synchronous and auto-checkpoint settings; local auto-vacuum, trusted-schema and journal-size choices remain local. | 51 library and 11 database integration tests, formatting and strict all-target Clippy pass. `6ea0968`. |
| Observo `master` | Each new r2d2 connection receives its five-second timeout; file initialization requests WAL, while memory retains its memory journal. | 98 server tests and changed-file formatting pass; strict Clippy still reports unrelated existing warnings. `a2b1906`. |
| Pezzottflix `master` | Server pool adopts foreign keys, five-second timeout and configured WAL/NORMAL or DELETE mode; standalone RaiPlay channels/content CLI stores adopt WAL. In-memory keeps its journal. | Server library 548 passed, one ignored; CLI 19 tests and strict CLI Clippy pass. Changed files format; ordinary server library Clippy passes with existing warnings. `59df421`. |
| SCT `master` | Offline archive index and catalog use foreign keys, DELETE journal and FULL synchronous through the policy; cache-size and temporary-store PRAGMAs remain local. PostgreSQL server catalog is outside 07a. | Archive catalog, reader and restore tests pass; formatting and strict all-target archive Clippy pass. `22240bd`. |
| Simple Agents `master` | Service pool uses foreign keys, WAL, FULL synchronous and five-second timeout; Runner transport uses WAL, FULL and five-second timeout without adding foreign keys. Both apply and verify settings on each new connection. Test-only stores and read-only paths without selected 07a settings are outside scope. | 61 service library, seven foundation, 19 Runner library and one Runner state tests pass; formatting and strict all-target Clippy pass. `0c122e6`. |

The six N/A assessments remain unchanged: Androidoscopy has no SQLite store;
LelloStore, Paranza, Peerlo and Simple AI select only driver defaults or perform
schema work outside 07a; Pezzottify Downloader uses Python SQLite. Quentin
Torrentino remains Pending because it is excluded by request. Final matrix:
**10 Done, 0 Partial, 1 Pending, 6 N/A**.

Each migration was committed in an isolated worktree branch, then its active
`master` branch was rebased onto that branch. Integrated tips and trees were
verified, and temporary worktrees and branches were removed. Existing unrelated
consumer edits and pre-existing worktrees were preserved. Active
`simple-server.rev` pins and build references were updated where present. No
push or deployment was performed.

## Step 07e shared SQLite schema core — 2026-09-29

Implemented in `40c5214` from clean `master` base `085b569`, on isolated
`codex/07e-schema` worktree branch. The optional `database-sqlite-schema` feature
provides `database::sqlite::schema`: borrowed/owned snapshots, safely quoted
identifiers, trusted application expressions, deterministic creation plans,
driver-neutral observations, explicit comparison policies and structured
reports. No runtime dependency is added. The normal dependency graph for this
feature contains only `simple-server`.

The implemented scope is ordinary tables, columns, ordered/composite primary
and unique keys, composite foreign keys and named/partial indexes. Unsupported
or unavailable metadata cannot silently produce a match. Required-subset reports
separately identify permitted extras; exact reports reject them. Applications
retain SQL execution, transactions, ledger/version markers, domain queries and
legacy classification. See the [API and adapter contract](step-07e-sqlite-schema.md)
for coverage limits, including the ordinary-table capability assertion.

Verification: the existing 12 connection-policy/migration tests passed before
changes. All 18 new schema tests pass against bundled SQLite through the existing
rusqlite 0.33 development dependency, including metadata differences, composite
constraints, partial-index contents, attached database isolation, quoted names,
unsupported properties, metadata read failure propagation, and file-backed
rollback/retry/reopen with data and version preservation. Borrowed definitions
and runtime observations are tested together. No SQLx consumer or production
canary is claimed by these fixtures.

Repository formatting, strict all-target/all-feature Clippy, default and
all-feature tests, the existing feature-matrix checks, no-default-features check,
and strict all-feature rustdoc passed. The sandbox initially prevented existing
HTTP tests from binding local sockets; the permitted rerun completed the tests.
The final rustdoc check found a new unresolved link, which was corrected and
rechecked successfully. The standalone schema example runs; documentation links,
Markdown column counts and rendered HTML table dimensions/statuses were checked.

The new 07e column has **17 Pending**: Pezzottify awaits its canary, fifteen
services await capability-specific applicability review, and Quentin Torrentino
remains excluded. Existing 07a/07b adoption is unchanged. Backup coordination
(07c) and synchronous execution (07d) remain planned. The dedicated branch is
integrated into `master` and its worktree/branch removed after verification;
no push or deployment is part of this increment.


## Step 07e Pezzottify schema canary — 2026-09-29

**Done for the versioned schema helper.** Pezzottify `dev` advanced from
`c53e06cb` to `acef712fdcca785042b7be1b8fb22eea465dca11`. The reviewed shared
revision and active `simple-server.rev` pin is
`7bc92f5f889dba2fb4c688e12920c3fe248af3a5`, integrated into simple-server `master`.

The production descriptor helper now delegates table/index creation to shared
plans and schema comparison to shared reports. Creation covers the user, server,
catalog, enrichment and download-queue versioned stores. Existing user, server
and download-queue validation paths use the shared validator; catalog and
enrichment startup behavior is unchanged. Local descriptors/macros, rusqlite I/O,
SQL migration execution, transactions, marker offset 99999, legacy catalog
classification, raw ingestion/search/auxiliary schemas and backup/checkpoint
behavior remain application-owned. This is not adoption of 07c/07d.

The canary required explicit mixed-depth validation controls in simple-server:
exact regular columns and case-sensitive names, first primary-key column,
name-only indexes, unordered unique named-column sets, per-column foreign-key
ON DELETE checks and an explicit table-property exclusion. Defaults retain the
application's single-layer parenthesis normalization. The report records
unverified definitions/predicates, expressions, ON UPDATE/grouping, hidden
columns and other DDL properties instead of implying full-schema validation.
Missing selected metadata fails closed; SQLite metadata errors propagate.

Verification:

- Baseline Pezzottify full library suite: **1,125 passed, two existing ignores**.
  Final: **1,137 passed, two existing ignores**. Loopback tests pass with the
  required sandbox permission.
- **12 new canary tests**, including differential creation/validation against a
  frozen pre-change oracle for **38 historical snapshots** (16 user, 8 server,
  10 catalog, 1 enrichment, 3 download). Independent actual index metadata and
  partial-predicate comparisons also pass.
- File-backed production startup, upgrade/data retention, restart, mid-upgrade
  rollback and successful retry; schema drift fails before backup registration.
  Legacy acceptance/rejection and metadata authorization errors are covered.
- Consumer formatting, database-boundary checks and strict production lib/bin
  Clippy pass. Shared **22 schema tests** and full `bash scripts/check` pass
  (all-feature lint/tests, feature matrix, no-default check and strict rustdoc).
- No consumer all-target Clippy claim (existing test-only lint debt), Docker,
  browser or Android qualification. Existing num-bigint-dig future-compatibility
  warning remains. See Pezzottify `docs/step-07e-sqlite-schema.md` for commands.

Both original development branches were rebased onto their dedicated branches;
ancestry and identical tested trees were verified. Owned worktrees/branches and
scratch logs were removed; pre-existing worktrees and recovery branches were
preserved. Nothing pushed or deployed. Both trackers show **1 Done, 16 Pending**
for 07e: fifteen need applicability review, and Quentin Torrentino stays excluded.
## Public crates.io package migration — 2026-09-29

**Done for public package delivery in all 18 active consumer development branches
and Homelab build orchestration.** Published `lelloman-simple-server` 0.1.0 from
`c95506164669c37a12bc06339a3b977a4b8d6a1b` under `MIT OR Apache-2.0`. The
`simple-server` dependency alias retains existing Rust imports. All 25 consumer
lockfiles identify crates.io and checksum
`1f3187c81c94701cd041df7ab17ce968b73c967db77c551170e3c24960b6c77a`.
See [publication evidence](publishing.md#verified-cratesio-release-010-29-september-2026).

The publication includes concurrent SQLite schema changes through `7345a52`.
The full release check script passes formatting, strict all-target/all-feature
Clippy, default/all-feature tests and feature matrix (**569 test executions**),
no-default-feature compilation and strict rustdoc. Clean-checkout publish dry
run and real publication passed. An independent fresh Cargo home, without
registry credentials, downloaded, compiled and ran the released package; its
archive checksum matches the consumer lockfiles.

All production users were identified from manifests and shared API imports.
Existing feature selections/default-feature behavior are retained, including
Pezzottify's concurrent SQLite schema feature. Native, CI, Docker, Compose and
Android build paths no longer require a sibling library checkout or private
registry access. The old checkout scripts/revision files were removed. The
shared implementation is the reviewed release; this delivery migration makes no
new capability-adoption claims. The private Fucina release remains available.

| Repository | Development branch / integrated commit | Verification |
| --- | --- | --- |
| androidoscopy | `master` / `8cc9f4e7b9dc` | Server and pairing JNI crate: all-target checks. |
| crumbles | `master` / `01a1ae04c0bb` | Workspace and native-dispatch qualification: all-target checks. |
| fausto | `master` / `624a90223a25` | Workspace all-target check. |
| favzetto | `master` / `07553fcddff9` | Backend all-target check. |
| lello-auth | `master` / `dc2da7259087` | Workspace/examples all-target check. |
| lellostore | `master` / `0d361fa9a969` | Backend all-target check, repeated after concurrent Paravoid changes. |
| meteonesto | `master` / `5837d312eb03` | API, gateway and pipeline all-target checks with required Rust 1.97.1. |
| observo | `master` / `53a78a443838` | Server and content-extractor all-target checks. |
| paranza | `master` / `309ed300abc1` | Workspace all-target check. |
| peerlo | `master` / `7c482a44a9f6` | Workspace all-target check. |
| pezzottflix | `master` / `f4d81b1a524d` | Workspace and standalone RaiPlay CLI all-target checks. |
| pezzottify | `dev` / `b47ec31b03b2` | All-target check repeated after current dev changes; concurrent package migration retained. |
| pezzottify-downloader | `master` / `9bd75f9a24bc` | All-target check; concurrent equivalent package migration retained. |
| quentin-torrentino | `master` / `1f76d881b199` | All-target check; before/after test failures match; both signal/drain cases pass (details below). |
| sct | `master` / `77732bae83d9` | Workspace all-target check; qualification source export updated. |
| simple-agents | `master` / `ecae1fa36083` | Workspace and managed-handoff qualification all-target checks; unrelated working edits preserved. |
| simple-ai | `master` / `c199e6fee940` | Production workspace libraries/binaries pass after concurrent semantic-service changes; test fixture limitation below. |
| torrentino | `master` / `81fdaa78d71d` | Replacement service all-target check and 43 tests pass before/after, including real-binary shutdown. |
| homelab | `master` / `83b007f3696a` | Four hermetic release-workflow tests, including immutable lockfile export/build-failure cleanup; shell syntax passes. |

All 17 changed Dockerfiles pass `docker build --check`. YAML parses, shell/Python
syntax and diff whitespace checks pass. Existing embedded frontend assets were
reused for Rust checks; no new frontend build or full Android qualification is
claimed. Full container builds and complete application suites were not rerun
for every service. The separately recorded concurrent Pezzottify/downloader
migration includes its own Docker and broader test evidence.

Two legacy HTTP implementations needed explicit compatibility changes because
the old `simple_server::axum` re-export and public backend serve helper no longer
exist. Quentin Torrentino and the separate Torrentino replacement service now
import **the same Axum 0.8.9** directly for their existing HTTP code, while using
the public shared package for lifecycle, logging and other existing helpers.
Their HTTP serving retains graceful lifecycle cancellation and the
pre-requested-shutdown guard. This is not owned HTTP migration: Quentin's Axum
centralization/routing remains Pending, and Torrentino's raw HTTP usage is
recorded here outside the original 17-service capability matrix.

- Torrentino: **43 tests pass before and after**, including real HTTP/WebSocket
  contracts and binary signal shutdown; all targets compile.
- Quentin: all targets compile. The baseline and final workspace test command
  both reach **521 passed, one failed** (`test_post_process_dispatches_to_video`).
  The separately run server tests match **174 passed, one failed**
  (`test_musicbrainz_search`), including 144/145 real HTTP E2E cases. These
  baseline failures remain; tests after an early suite failure are not claimed.
  Existing real-process SIGINT/SIGTERM checks pass with open WebSockets, listener
  closure and durable `ServiceStopped` audit events.
- SimpleAI: production libraries/binaries pass; all-target test compilation is
  blocked by the existing untracked `scripts/configs/rtx.toml`
  include. No private runtime configuration was copied to satisfy that test.
- Meteonesto: the initial default Rust 1.96 check was rejected by its declared
  1.97.1 minimum; all three checks pass with installed Rust 1.97.1.
- Homelab: the old `test_deploy.py` fixtures fail on the unchanged baseline.
  The current four-case release-workflow suite passes, including the new
  application-lockfile export, no sibling source, build-failure-before-push and
  temporary-context cleanup checks. No deployment command was run.

Implementation and verification used isolated branches/worktrees. Concurrent
package migrations in Pezzottify, its downloader and SimpleAI were retained,
including their stricter locked Docker builds and unrelated application changes.
LelloStore's new Paravoid commit and Homelab's concurrent build adjustment were
preserved. Every consumer development branch was rebased onto its migration
branch, ancestry and tested-tree integration were checked, and all 19 temporary
consumer worktrees/branches were removed. Uncommitted/untracked original files
were verified preserved; Simple Agents' overlapping Cargo.lock and README edits
were merged without stashing its active work. Original dirty application changes
are outside the clean committed-branch verification scope. Pezzottify's separate
Paravoid worktree and SCT's validation scratch checkout were left untouched.
No Git push, deployment or branch-protection change was performed.


## Step 07e consumer rollout checkpoint — 2026-09-30

**Stopped at user request.** Matrix: **3 Done, 1 Partial, 13 Pending**.
Pezzottify's earlier canary is unchanged. Remaining applicability decisions have
not been finalized; unsupported requirements are not labeled N/A. Torrentino stays
skipped. 07c/07d are unchanged. No further service is started at this checkpoint.

All three migrations use published, exact `lelloman-simple-server = 0.1.0` with
`database-sqlite-schema`; registry checksum
`1f3187c81c94701cd041df7ab17ce968b73c967db77c551170e3c24960b6c77a`.
No path override or unpublished API is required.

| Consumer | Integrated master commit | Scope and verification |
| --- | --- | --- |
| SimpleAI | `ec8612b5b93e2ad595d62470098c4e0319c7f6bd`, from `7ca18c2` | Eight audit bootstrap tables and three indexes use shared creation plans at existing startup positions; ALTER, idempotence and transaction ownership remain local. Baseline 64 focused audit tests passed; final full backend library **299 passed**. Four new differential/file-backed tests cover actual schema/index metadata, defaults/keys, legacy/repeated startup, data/markers, failed-bootstrap ordering and DQS restoration/existing-index acceptance. Changed-file formatting passes. Production no-deps Clippy completes with 11 existing warnings outside the adapter; strict dependency Clippy stops on four untouched simple-ai-common warnings. |
| Paranza | `50e621c405b85b20e628807be5d5f329067b3ab9`, from `309ed30` | Ten store bootstrap tables use shared creation plans; IF NOT EXISTS, table order and activation-column repairs remain local. Baseline **17**, final **19** store tests passed. Two new tests compare all table/default/key/unique-index metadata and verify production legacy-file startup/restart, retained data/markers and activation repairs. Strict store all-target Clippy, package formatting and diff checks pass. Lockfile only adds the already-locked shared package to the store. |
| Peerlo | `7af21ac67d078a812e4c2472ae44e3902cd737d2`, from `7c482a4` | **Partial:** tracker infohashes/candidate_peers and six indexes use shared plans under the existing blocking lock. MetadataStore's AUTOINCREMENT files table and FTS5/triggers remain an unsupported coverage gap. Baseline tracker **80**, final **84** passed; unchanged metadata **130 unit + 16 integration** passed, one existing ignored doctest. Four new tests cover metadata/defaults/composite keys, idempotence/markers, real legacy-file reopen, failure order and missing-index-column rejection with DQS restoration. Strict tracker all-target Clippy and package formatting pass. |

Generated quoted index columns revealed a real SQLite DQS fallback difference:
a missing column could become a string literal instead of the old bare-column
failure. Both index-executing adapters temporarily disable DQS_DDL for generated
indexes and restore its previous value on success/error. Existing-index acceptance
and original partial-bootstrap failure order are regression-tested. This adds no
new schema validation contract or persistent connection setting.

Each consumer has `docs/step-07e-sqlite-schema.md` with commands and limits.
No full daemon/fleet/runner, DHT, GPU/model, Docker/browser/Android or production
qualification is claimed. All commits were made in dedicated worktrees; original
master branches were rebased onto them and ancestry/identical tested trees
verified. Owned worktrees, temporary branches and scratch files were removed at
this checkpoint; unrelated work and concurrent Meteonesto operations commits
were preserved. Nothing pushed, deployed or published.

Remaining: finalize applicability for every untouched row, distinguishing
application migration scripts from reusable creation/validation helpers. Preliminary
scans found AUTOINCREMENT/FTS in Observo, CHECK in SCT's archive catalog, and
CHECK/STRICT/triggers in Simple Agents. Do not drop these requirements or label
incompatibility N/A. Peerlo stays Partial until its metadata scope is resolved.
The scope question about existing helpers versus rewriting all application
migration SQL was unanswered when the user stopped work; resolve it on resume.
All unactioned rows retain Pending.

## Step 07e extended creation and Peerlo release gate — 2026-10-03

Shared source `bda33540e410bc759a80e8627e132923b8996859` implements additive
AUTOINCREMENT, FTS5 and trigger descriptions with explicit creation mode.
Existing creation APIs remain compatible. Execution, synchronization SQL bodies,
backfill, rebuild, transactions, PRAGMAs and markers stay consumer-owned. Ordinary
schema observations do not establish validation of these extended objects.

Library master includes the implementation. Full `scripts/check` passes, including
9 extended SQLite tests and 22 existing schema tests, strict Clippy and rustdoc.
The 0.1.1 package builds and verifies; publication dry-run passes without upload.
Archive SHA256: `f7fd8567b285a8e1cb8315626df78c8d56b444907e8a9a5ac9e83f5db6e47f25`.
The archive records the shared source commit above. Version 0.1.1 is not published.

Peerlo canary `6268085` is committed on isolated `codex/07e-metadata`. Its metadata
bootstrap uses both shared creation phases, preserving legacy failure order,
search triggers, AUTOINCREMENT, existing objects and driver defaults. All five
shared dependency aliases target exact 0.1.1; the proposed registry checksum
matches the prepared archive. No path override is committed. Tests used a temporary
local patch to the exact shared source: **236 passed**, one existing ignored
doctest. Six new legacy comparisons cover schema, CRUD/search, reopen, rollback,
sequence progression, missing backfill, startup failures and retry. Workspace
check and formatting pass. Strict metadata all-target Clippy retains existing
redundant_closure and unnecessary_map_or findings; normal Clippy completes.

Peerlo master remains clean at `7af21ac67d078a812e4c2472ae44e3902cd737d2`.
No registry-based verification or consumer integration is claimed yet. Publication
requires user approval; afterwards download verification, locked tests against
crates.io, master rebase and worktree cleanup remain. Release and canary worktrees
are deliberately retained for this gate. Matrix remains **3 Done, 1 Partial,
13 Pending**. No push, deployment or production database operation occurred.

## Step 07e Peerlo published canary completion — 2026-10-03

With explicit user approval, `lelloman-simple-server` **0.1.1** was published
from clean source `bda33540e410bc759a80e8627e132923b8996859`. The independent
static.crates.io archive download records that same source revision and SHA256
`f7fd8567b285a8e1cb8315626df78c8d56b444907e8a9a5ac9e83f5db6e47f25`.
Cargo downloaded the published package for Peerlo; no local override is present.

Peerlo `master` was rebased from `7af21ac` onto the canary branch and is now
`1ce8481` (implementation `6268085`). The tested source tree and branch ancestry
were verified. Both tracker and metadata production bootstrap use shared creation
plans, covering ordinary tables/indexes, AUTOINCREMENT, external-content FTS5
and the existing synchronization triggers. This is creation adoption, not a new
query layer or extended-object validator. Application-owned driver behavior,
transactions, search SQL, backfill and markers remain as before.

Registry-only `cargo test --locked -p peerlo-metadata -p peerlo-tracker`: **236
passed**, one existing ignored doctest. `cargo check --locked --workspace` passes
with six existing CLI warnings; formatting and diff checks pass. No new lockfile
changes occurred during these checks. Existing strict metadata Clippy debt and
the absence of full daemon/DHT, Docker or production qualification remain as
recorded above. Shared full checks/package verification passed before publication.

Owned release and consumer worktrees/branches were removed after integration.
Both original repositories are clean. Matrix: **4 Done, 0 Partial, 13 Pending**.
Other consumer rollout has not resumed; Torrentino remains excluded. No Git push
or deployment occurred. This completion supersedes the release gate above.

## Step 07e Observo, Favzetto and Fausto parallel batch — 2026-10-04

Three agents owned disjoint repositories; root owned both central trackers.
All consumers use published exact `lelloman-simple-server =0.1.1`, reviewed
source `bda33540e410bc759a80e8627e132923b8996859`, archive SHA256
`f7fd8567b285a8e1cb8315626df78c8d56b444907e8a9a5ac9e83f5db6e47f25`.
No local patch or new publication was needed. Creation adoption is optional and
scoped; authored migration SQL, drivers, query behavior and transaction ownership
remain application-owned. No new structural-validation policy is claimed.

| Consumer | Integration and adoption | Verification and limits |
| --- | --- | --- |
| Observo | Clean `master` `53a78a44383885c2950a07c7a8f964c6dd9f6edb` → `9cf709bdccee44731ec5d806e678066d8377fb50`. **Done (creation)**: production bootstrap uses extended plans for all 18 ordinary tables, indexes and internal FTS5. Startup order, failure boundaries, AUTOINCREMENT, constraints, pool/WAL and application mirror updates preserved. Server and sibling extractor pins aligned. | Baseline DB test passes; final **103 server tests pass**, including five differential SQLite tests. Real-binary E2E covers auth, metrics, CRUD, filtering, restart, cron admission and shutdown. Extractor **16 pass / 3 failures**, all reproduced on original master. Clippy retains the same denied approximate-constant error and 36 warnings. Changed-file formatting and diff checks pass; unrelated repository formatting debt preserved. |
| Favzetto | Clean `master` `07553fcddff927478698a52d30ba21b407d60e27` → `6bcbc8fc1d33d4a696772d7616e1b42ccb4a0abc`. **Done (ledger creation; scoped)**: production `schema_migrations` creation uses shared plan, preserving nullable TEXT primary-key/default metadata and existing-table no-op. Authored SQL migrations and legacy CHECK/ALTER/rebuild repairs unchanged. | Baseline **6 DB tests**, final **7** with real SQLite differential coverage. Final **138 library + 102 API + 2 lifecycle + 2 logging tests pass**. Two bridge API failures (`catalog_research_runtime_bridge_approves_runtime_draft`, `catalog_research_runtime_bridge_rejects_runtime_draft`) reproduce on unchanged base: terminal completed-state transitions return 400 versus expected 200. Normal all-target Clippy completes with existing warnings, none in changed DB module. Changed-file formatting/diff pass; full-repo formatting debt preserved. |
| Fausto | Clean `master` `624a90223a25712c374cea1283a27f5eecc7b565` → `9f1d6a1749a7560a57b5e3484de80918a95be105`. **Partial**: production `schema_version` ledger uses shared ordinary creation; existing-table early return and unversioned adoption remain. Optional core embedding insertion/search/migration still create dynamic `vec0` virtual tables unsupported by current planner. Historical three SQL migrations intentionally remain local, which is not itself a gap. | Baseline **433 unit tests**; final **435 unit + 4 integration pass**, four integration and one doctest retain existing ignores. Two new SQLite regressions compare metadata/constraints/rowid behavior and preserve malformed/custom ledgers, rollback and retry. Workspace all-target check, formatting and optional sqlite-vec production compilation pass. Normal Clippy completes with existing debt (352 lib-test warnings, 299 duplicate); no new adapter/test findings. Optional sqlite-vec tests are blocked on both baseline/final by existing String-versus-Option model_version fixture. |

Each consumer records details in `docs/step-07e-sqlite-schema.md`. All three
original branches were rebased **onto** their isolated migration branches;
ancestry and identical tested trees were verified before deleting owned
worktrees/branches. Original repositories are clean; owned scratch is cleaned.
No push, deployment or production database operation occurred.

Matrix now **6 Done, 1 Partial, 10 Pending** (Torrentino remains excluded).
Fausto's remaining gap suggests a later generic virtual-table creation description
with explicit trusted module arguments and application-owned extension setup;
FTS5 must not be substituted for vec0. Other services still need individual
applicability review rather than automatic descriptor adoption.

## Step 07e Androidoscopy, Crumbles and lello-auth batch — 2026-10-04

Three agents assessed disjoint repositories; root updated both central trackers.
Reviewed optional shared capability: published `lelloman-simple-server =0.1.1`,
source `bda33540e410bc759a80e8627e132923b8996859`, archive SHA256
`f7fd8567b285a8e1cb8315626df78c8d56b444907e8a9a5ac9e83f5db6e47f25`.
Only Crumbles needs the new API and updates its dependency. No feature/version
change is imposed on N/A consumers.

| Consumer | Applicability and integration | Verification and limits |
| --- | --- | --- |
| Androidoscopy | **N/A for Rust schema adoption**. Rust session state is memory and credentials are files/JSON; no Rust SQLite driver, bootstrap or shape-comparison adapter. The Android SDK viewer uses platform SQLiteDatabase to inspect arbitrary host-app databases; the Kotlin demo creates local tables. No Rust/Kotlin bridge is provided by this module. Assessment-only `master` `14afe1b21059433e0d5dc3bc0dcf5f6e8c2ac142` → `fdb16f71036cf7aeaca160b807b65bc418c41dbb`. | Locked offline metadata passed for server (302 packages) and pairing helper (9), neither graph containing SQLite/SQL drivers; server dependency tree and production source inspection confirm scope. E2E has no committed lockfile, so locked metadata cannot run; its manifest was inspected and no lockfile generated. Runtime/Android/dashboard suites not rerun for docs-only assessment. Existing dependency pins remain unchanged. |
| Crumbles | **Partial**. Core legacy-v17 adoption now creates `_sqlx_migrations` using shared ordinary creation, preserving column metadata/defaults/key behavior and execution ownership. Authored domain history and legacy fingerprint remain local. Integration runtime ledger still requires unsupported STRICT and CHECK(dirty IN(0,1)); its guarantees are preserved. Clean `master` `a5a8ebdc9ddb582689c36cb5bfb261706bb4c28a` → `599cbd2752ffa8d114013fa236b41b6c6d90501b`; actual registry exact 0.1.1 and checksum, no unrelated lockfile upgrades. | Baseline **12 core migration + 9 integration DB tests**. Final **14 core migration + 9 integration DB tests**, including two SQLite old/new metadata/constraints/default/nullable-key and rollback/marker regressions. Full core library **681 tests pass**. Strict all-target Clippy for core and integration, formatting and diff checks pass. No new structural-validation policy, full-system E2E, deployment or production database operation claimed. |
| lello-auth | **N/A for current optional schema capability**, not N/A for SQLite itself. All SQLite creation, including schema_version ledger, belongs to authored versioned MIGRATIONS/SCHEMA_V1. Marker reader only reads; there is no independent unversioned bootstrap or existing descriptor/shape-validation adapter. PostgreSQL independent ledger is outside SQLite scope. Assessment-only `master` `dc2da72590878646e9e98b935ce8dd5edddde667` → `228edd17bfb83d6ef8a70ae9696b08828d7bdc07`; migrations, dependencies and policy unchanged. | **8 focused database tests pass**, covering fresh startup, future marker rejection, connection policy, transactions and pool access; source inspection and diff checks pass. Full workspace, browser/E2E and PostgreSQL suites not rerun for documentation-only change. Three original untracked identity-provider research files are preserved. |

Each consumer has `docs/step-07e-sqlite-schema.md` with evidence and boundaries.
All agents used dedicated worktree branches, committed there, rebased original
`master` **onto** the migration/assessment branch, and verified ancestry and
identical tested trees before removing owned worktrees/branches and scratch.
Unrelated untracked files and pre-existing worktree records were preserved.
No push, publication or deployment occurred.

Matrix now **6 Done, 2 Partial, 2 N/A, 7 Pending**, including excluded Torrentino.
Pending services: LelloStore, Meteonesto, Pezzottflix, Pezzottify Downloader,
SCT and Simple Agents; Torrentino remains skipped. Concrete shared gaps remain
Fausto's dynamic vec0 and Crumbles integration STRICT/CHECK ledger creation.

## Step 07e STRICT/CHECK and generic virtual-table core — 2026-10-04

Shared implementation `358a226a29d66e6f8a73df4573799c8b0970c8cd` is committed
and integrated into `master` from an isolated worktree branch.
`create_extended_plan_with_options` adds `CreationOptions` without changing existing
public snapshot literals or the existing creation APIs' SQL. Table options describe
STRICT and optional named column/table CHECK constraints. Generic virtual tables
have escaped identifiers and explicit trusted module arguments, supporting
module-defined grammars such as vec0 dimensions. The shared library loads no
extensions, executes no SQL, owns no transaction and adds no runtime dependency.
Existing structural observations do not validate these new properties.

Full `bash scripts/check` passed: format, strict all-target/all-feature Clippy,
default/all-feature tests, feature matrix, no-default build and strict rustdoc.
The schema matrix now includes **17 extended creation tests** (eight added) plus
**22 existing schema tests**. Real SQLite covers strict type rejection/coercion,
ANY, key metadata, named/column/table CHECKs including NULL semantics, rollback,
Crumbles-style legacy ledger comparisons and file restart, attached-schema FTS5
and RTree creation/query/idempotence, unknown modules and known name collisions.

Standalone repository consumer fixture `tests/fixtures/sqlite_vec_creation`
loads sqlite-vec **0.1.6** outside the library and passes dynamic 2/3-dimensional
vec0 creation, nearest-neighbor query, existing-table no-op and wrong-dimension
rejection. It has its own lockfile and runs in `scripts/check`; independent format
and strict Clippy pass. Unsafe extension FFI stays in this external fixture; the
library retains its unsafe-code prohibition. Cargo excludes this nested consumer
crate from the public archive; shared regression tests are packaged.

Prepared release **0.1.2**: package verification and crates.io publication dry-run
passed with no upload. Archive has 126 files and no build/environment artifacts;
embedded source commit matches the implementation above. SHA256:
`d31aa705a4b51e0dfea8fdc334de20f8cf6048b049dedb970ef63227a7661d46`.
Publication requires user approval. The release worktree/branch are intentionally
retained at the reviewed source commit while this gate is pending.

Crumbles and Fausto remain **Partial** until their actual production callers use
a published new API and pass consumer verification. No consumer repository,
historical SQL migration, production database, push or deployment was changed
in this core increment. Matrix remains **6 Done, 2 Partial, 2 N/A, 7 Pending**.
See [the API contract](step-07e-sqlite-schema.md#strict-check-and-generic-virtual-tables-published-in-012).

## Step 07e Fausto and Crumbles completion — 2026-10-04

User authorized continuing publication and both consumer integrations.
`lelloman-simple-server` **0.1.2** was published from clean reviewed source
`358a226a29d66e6f8a73df4573799c8b0970c8cd`. Independent crates.io archive
download matches its embedded source commit and SHA256
`d31aa705a4b51e0dfea8fdc334de20f8cf6048b049dedb970ef63227a7661d46`.
Both consumers use exact published 0.1.2 without a path override; lockfile
changes are limited to shared package version/checksum. Full shared checks,
package verification, archive inspection and publication dry-run passed earlier.

| Consumer | Integrated production adoption | Registry-backed verification |
| --- | --- | --- |
| Fausto | **Done (ledger + optional runtime vec0 creation)**. `master` `9f1d6a1749a7560a57b5e3484de80918a95be105` → `b2ccfd6`. Production ensure_vector_table uses shared generic creation with original naming, text primary key and dynamic float dimensions. Existing-table no-op, module registration, locks, execution/errors and insertion/search/deletion stay local. | Baseline default **435 unit pass**; optional baseline compilation reproduced the previously recorded String-versus-Option model_version fixture error. The one-line fixture fix now enables optional tests. Final **438 sqlite-vec unit pass**, including legacy/shared metadata, 2/3 dimensions, nearest-neighbor search, duplicate/dimension errors, rollback, changed-dimension no-op and repeated legacy-file reopen. Default **435 unit + 4 integration pass**, existing four integration/one doctest ignores remain. Workspace all-target check/format/diff pass; two existing server warnings. Normal optional all-target Clippy completes with existing debt; strict cleanliness is not claimed. |
| Crumbles | **Done (core + integration runtime ledger creation)**. `master` `599cbd2752ffa8d114013fa236b41b6c6d90501b` → `de7556a`. Integration migrate executes shared STRICT/CHECK ledger creation after unmanaged-schema check, before ledger reads/preflight. Original key/nullability/types/defaults, dirty CHECK and existing-object no-op remain. Core creation, authored migrations/fingerprints/checksums, staged upgrades and transactions unchanged. | Baseline **9 integration DB tests pass**. Final full integration **99 pass**, with two new SQLite differential tests for exact metadata/default/key/STRICT enforcement, dirty/NULL/duplicate errors, malformed/existing ledger preservation, rollback, retained marker and retry. Existing DB tests cover file restart, staged upgrade/restore, dirty/future/checksum and failed-migration rollback. Full core **681 pass**. Strict core/integration all-target Clippy, workspace formatting and diff checks pass. |

Each consumer records evidence in `docs/step-07e-sqlite-schema.md`. Work was
committed in isolated worktree branches; original `master` branches were rebased
**onto** those branches. Ancestry and identical tested trees were verified before
owned worktrees/branches were removed. The reviewed shared release worktree and
owned transient release/test files were also cleaned after publication.
All three original checkouts are clean. No Git push, deployment or production
database operation occurred. Full consumer server/browser/Docker/Android E2E
qualification was not run. Creation adoption does not claim new extended-object
structural validation or ownership of authored SQL migration history.

Matrix: **8 Done, 0 Partial, 2 N/A, 7 Pending**. Remaining in-scope services:
LelloStore, Meteonesto, Pezzottflix, Pezzottify Downloader, SCT and Simple Agents.
Torrentino remains excluded. This record supersedes both Partial gaps and the
0.1.2 publication gate above.

## Step 07e Meteonesto, Pezzottflix and Downloader batch — 2026-10-04

Three agents owned disjoint repositories; root updated both central trackers.
Reviewed shared capability is published exact `lelloman-simple-server 0.1.2`,
source `358a226a29d66e6f8a73df4573799c8b0970c8cd`, archive SHA256
`d31aa705a4b51e0dfea8fdc334de20f8cf6048b049dedb970ef63227a7661d46`.
Only Pezzottflix's standalone CLI uses this new API; N/A consumers retain their
existing dependency pins. No shared library modification or publication occurred.

| Consumer | Applicability and integration | Verification and limits |
| --- | --- | --- |
| Meteonesto | **N/A** for optional current creation/shape-comparison API. Weather pipeline creates all production tables/indexes and its ledger inside six immutable checksum-bearing SQL migrations; no independent bootstrap or current structured shape comparator. quick_check checks integrity, while existing 07b handles dual ledger/user_version markers. API/gateway have no SQLite driver/bootstrap. `master` `aa9c4992c046c3516ab01f8e93c2826b381be5f5` → `471e9d06808e976a476339075ea2dafcc7737d68`, assessment only. | **11 actual SQLite database integration + 4 database unit tests pass**, pipeline formatting/diff checks pass. SQL/checksum/transactions/policies/dependencies unchanged. Full components/provider/CLI/HTTP/Python/release/API/gateway/deployment suites not rerun for docs-only audit. |
| Pezzottflix | **Done (CLI creation; scoped)**. Standalone raiplay-cli production bootstrap uses shared plans for three ordinary tables, two indexes and internal FTS5, preserving channels-first/content-second phases, autocommit, defaults, manual mirror upsert and no-backfill behavior. Server uses SQLx migrator/versioned SQL only; that authored history remains local. `master` `f4d81b1a524db2373dad7f5d78bfd7d56e3fb48e` → `71a13092a09b533525b985f175241502e2a8b538`. | Baseline **4 CLI DB + 5 server DB tests pass**. Final full CLI **22 pass**, including three SQLite differential tests for metadata/defaults/index terms/FTS shadows, counters, restart/search, unindexed path key, existing/custom shape no-op, no backfill and missing-column/FTS-shadow partial-failure inventories/markers/autocommit. Primary SQLite error messages compare; driver SQL source text/offsets naturally differ. Strict all-target CLI Clippy, CLI formatting/diff pass. CLI registry lock pins exact 0.1.2/checksum with no unrelated updates; separate server workspace and graph remain unchanged at 0.1.0. Full server/browser E2E not run. |
| Pezzottify Downloader | **N/A for current Rust schema API**. Python cron owns sqlite3 SCHEMA_SQL/open_db for runs/download_attempts/events AUTOINCREMENT tables. Rust uses no SQLite driver/bootstrap/comparator. Existing quota stdio bridge exchanges clock/count/budget only; it has no schema generation/execution interface. No new cross-language bridge or fake feature adoption. `master` `9bd75f9a24bcd8d98bd0680d1c0ffd184a9f3669` → `22ffc5c343cbbf1bbffa51f0db978d72893c3504`, docs only. | Linux-filtered locked/offline metadata resolves **351 packages** without SQLite/SQL drivers. Unfiltered offline attempt lacked uncached macOS coreaudio dependency. Scoped Python baseline/final **80 pass / 23 deselected**, including isolated actual SQLite tests. Full baseline attempt **91 pass / 12 fail**, all requiring an absent built Rust quota executable (also absent original checkout); these are environment requirements, not claimed app regressions. Rust runtime/strict lint/authenticated downloads/deployment not run; no runtime/manifests changed. |

Consumer `docs/step-07e-sqlite-schema.md` files record detailed evidence.
Each assessment/migration was committed in an isolated worktree branch; original
`master` was rebased **onto** it, and ancestry/identical tested trees verified
before deleting owned worktree/branch/scratch. Original checkouts are clean.
No push, deployment, production database or credentials were used.

Matrix: **9 Done, 0 Partial, 4 N/A, 4 Pending**. Pending in-scope services are
LelloStore, SCT and Simple Agents; Torrentino remains excluded. Optional schema
adoption does not require replacing application-authored SQL migration history
or moving Python/Android storage into Rust. No new structural-validation policy
is claimed by this creation rollout.

## Step 07e Simple Agents and SCT batch — 2026-10-04

Two agents owned disjoint repositories; root updated both central trackers.
User explicitly deferred **LelloStore** because its backend is actively changing;
no agent/task/worktree was created for it and no LelloStore files were touched.
Reviewed shared API is published exact `lelloman-simple-server =0.1.2`, source
`358a226a29d66e6f8a73df4573799c8b0970c8cd`, registry archive SHA256
`d31aa705a4b51e0dfea8fdc334de20f8cf6048b049dedb970ef63227a7661d46`.
Consumer graphs resolve the actual registry package without local patches;
lockfile changes are limited to shared version/checksum. No library change or
publication was needed.

| Consumer | Integrated production scope | Verification and limits |
| --- | --- | --- |
| Simple Agents | **Done (independent service + Runner creation)**. `master` `24a420105a8de1dbd277c35b995e7444a13096c8` → `adee7a797b5f330cc2c10ec2f5c2e1535bfd1229`. Shared plans create STRICT/CHECK service_migration and transport_meta/transport_message, preserving default/column/key/UNIQUE semantics, BEGIN IMMEDIATE ownership, guard ordering, marker and binding checks. Authored service and runtime/coding/retention upgrade SQL remains unchanged. Workspace, Runner and delivery aliases align to 0.1.2. | Baseline **4 service DB + 23 Runner unit + 1 Runner integration pass**. Four added actual-SQLite comparisons cover metadata/keys/STRICT/defaults/CHECK/type/null error codes, idempotence, custom existing objects and rollback. Initial broader suite hit `maintenance_longer_than_permit_ttl_does_not_interrupt_execution` permit-starvation timeout; it passed isolated in 13.8 seconds. Subsequent full service/Runner run **179 pass**, skipping only that named case. This is not an unqualified wholly green broad run. Strict all-target no-deps Clippy, changed-file formatting and diff checks pass. Browser/Docker/deployment qualification not run; scheduler code unchanged. |
| SCT | **Done (offline archive creation)**. `master` `77732bae83d9b6a9d63761ece6b7a071970427f9` → `d6b81bb63661985c3ce98cd47910534d25d445c9`. ArchiveIndex::create and new-file ArchiveCatalog::open describe 12 ordinary tables/three indexes and catalog CHECK(format=1). Catalog header → marker insertion → remaining schema order, existing transactions, FKs/keys/nullability, locking/file/storage/driver policy preserved. Reopen checks retain original depth; PostgreSQL history is outside SQLite scope. | Baseline full archive **12 tests pass**; final **15 pass**, including three regressions against frozen old SQL for metadata/constraint behavior, marker/DDL rollback and legacy reopen. Strict archive all-target Clippy, full workspace compile, workspace format and committed-diff whitespace checks pass. PostgreSQL core/server integration, browser/client qualification/deployment suites not rerun. |

Consumer `docs/step-07e-sqlite-schema.md` files contain evidence and boundaries.
Both agents committed in isolated worktree branches, rebased original `master`
**onto** those branches, verified ancestry/identical tested trees, then removed
owned worktrees/branches/logs. Simple Agents retains its pre-existing prunable
worktree/recovery refs; its original checkout is clean. SCT retains exactly its
pre-existing untracked `.validation-work/` and `docs/step-04c-cors.md`.
No push, deployment, production databases or unrelated files were changed.

Matrix now **11 Done, 0 Partial, 4 N/A, 2 Pending**. LelloStore remains Pending
**deferred at user request**; Torrentino remains Pending/excluded. All other
services have verified scoped adoption or an evidenced N/A assessment. This
rollout does not claim full domain schema validation, move cross-language
storage into Rust, or replace authored SQL migration history.

## All-project backend dependency audit — 2026-10-04

Read-only consumer audit of `/home/lelloman/lelloprojects`: **64 top-level Git
repositories**, **5,632 tracked or nonignored source/manifest files** in the
focused Rust/Python/JS/TS/Go inventory. A broad hidden/unignored filesystem pass
also inspected 464 manifest paths (including generated copies), Kotlin/Java
server imports/build files, C/C++ server-library markers, and standalone project
directories. Generated targets/node_modules, downloaded toolchains, source caches
and SCT's preserved `.validation-work/simple-server` copy were excluded from
active findings. Branches/revisions below identify observed local working trees,
including current uncommitted/new nonignored source; no deployment state is claimed.
Some permission-denied capture/runtime-storage directories under km-g6-control
and watch-rns-rs were inaccessible. No permissions were changed; no locked build,
dependency-removal experiment or runtime tests were run. Detection uses known
framework names/source patterns and is not proof of absence of every possible
backend implementation. LelloStore was inspected read-only, never changed.

### Direct Axum

Active direct Axum dependencies/imports were found in three consumer repositories:

| Repository | Evidence | Scope |
| --- | --- | --- |
| Talia | `talia/engine/Cargo.toml:13`, `engine/src/main.rs:6`, plus `spikes/transport/server/src/main.rs:2`. | Engine routing/handlers/serving and test fixtures, plus an experimental transport server. Talia was never included in the 17-service tracker. |
| torrentino | `torrentino/crates/service/Cargo.toml:12`, `src/main.rs:80`, `src/api.rs:7`. | Separate repository from quentin-torrentino; production Axum router/handlers/rejections/WebSockets/serve and fixtures. Some simple-server helpers are used, but HTTP abstraction is incomplete. |
| quentin-torrentino | `quentin-torrentino/crates/server/Cargo.toml:34`, `src/main.rs:454`, `src/api/catalog.rs:5`. | Previously excluded service retains direct Axum routing/extractors/serving, multipart/SSE/WebSocket and test usage. Audit does not resume its migration. |

No direct Axum dependency or active source import/re-export was found in the
**16 non-excluded services** in the existing matrix, including read-only
LelloStore. Old comments/log target names do not constitute adoption gaps.
Axum inside simple-server is the intended implementation. SCT's untracked
validation copy of the library is an artifact, not a new consumer dependency.

### Backend libraries still used by migrated Rust services

| Theme | Current evidence | Interpretation / possible follow-up |
| --- | --- | --- |
| Disk static files / SPA fallback | Pezzottify `pezzottify-server/src/server/server.rs:36` and `route_builder.rs:451`; Pezzottflix `pezzottflix-server/src/server.rs:10`/`:440`; SCT `crates/sct-server/src/lib.rs:14`/`:34` directly use tower-http ServeDir/ServeFile. | Shared static-file/fallback abstraction is a useful multi-service increment. Fausto also has its own rust-embed-based Tower Service (`server/src/static_files.rs:18`); an embedded-assets adapter could serve that distinct policy. |
| Response compression | Pezzottflix `pezzottflix-server/src/server.rs:491` installs tower-http CompressionLayer. | Separate optional shared compression API; do not silently alter negotiation/content-type/streaming policy. |
| Mutable cookies / cookie middleware | lello-auth `crates/lello-auth-axum/src/middleware/session.rs:14`/`:44`, UI/authenticator handlers, server and examples use tower-cookies Cookie/Cookies/CookieManagerLayer. | Existing shared credential extraction does not replace its full mutable cookie-jar/output-cookie middleware contract. Extend shared cookies before removing this dependency. |
| Request-ID compatibility types | Fausto `server/src/api/correlation.rs:6`, `error.rs:9`, `federation/handler.rs:12`; Downloader `src/puppeteer/correlation.rs:6`. | Shared correlation is adopted, but application typed extensions still use tower-http RequestId; Downloader also uses its UUID generator. This is a compatibility boundary, not absence of shared correlation. |
| Unix HTTP serving / proxy | Downloader `src/downloader/http_server.rs:105` uses Hyper HTTP/1 serve_connection/TokioIo in actual serve_unix; `src/downloader/mod.rs:33` calls it. `src/puppeteer/proxy.rs:7`/`:27` uses Hyper/hyperlocal streaming Unix client. Hyper mock-server code further down proxy.rs is test-only. | Shared Unix-listener/connection serving could hide server-side Hyper. Streaming Unix HTTP/WebSocket client proxying is a distinct optional client capability, not normal TCP server adoption. |
| Configured generic HTTP rate limit | Pezzottflix `pezzottflix-server/src/server.rs:471` installs `middleware::rate_limit::RateLimiter` when rate_limit.enabled; `src/middleware/rate_limit.rs:3` directly uses Governor quota/clock/limiter. | **Actual active shared-policy gap.** Historical Step 10 doc says this middleware has no production reader, which is no longer true. Login failure windows, daily quota and TMDB pacing remain shared. Step 10 matrix corrected to Partial pending migration of this now-active path; current counters are 11 Done, 1 Partial, 5 N/A. |
| Tower Service/Layer and test utilities | Fausto custom static-file/rate-limit services, Downloader serving adapter, and many router oneshot tests use Tower traits/extensions directly. | Normal service composition/test infrastructure, not direct Axum. Hiding these would require a separately defined trait/test-adapter contract; not all such imports are production leftovers. |

Candidate **stale direct declarations**, not confirmed removal-safe: Androidoscopy
server tower-http (fs), LelloStore backend tower-http (fs), lello-auth-axum
tower-http (cors/trace), Pezzottify-server Hyper. Active reviewed production source
contains no corresponding direct use; logging target strings and negative tracing
assertions do not count. Validate each package/feature graph before removing.
LelloStore cleanup stays deferred. Legacy tower-http/governor references in
comparative tests may intentionally remain dev dependencies.

### Other projects and languages

- **Homelab:** production Python aiohttp idle-manager API/wake relay, Flask
  telegram-webhook, Go net/http access-gateway, and stdlib HTTP sync/observer
  services. These are outside the Rust migration matrix.
- **Casuccia:** Python stdlib ThreadingHTTPServer capture receiver/queue
  (`server/app.py:10`).
- **Observo worker:** Node built-in http management API
  (`observo-worker/src/management-api.js:9`, called by `src/index.js:132`).
  Its Rust server migration does not cover this Node process.
- **Simple AI:** Python stdlib HTTP semantic/extraction/classification/audio/XTTS/
  Chatterbox provider processes remain under scripts; separate aiohttp fake runner
  and HTTP/OIDC test fixtures are intentional test infrastructure.
- **Pezzottflix Android:** Ktor/Netty WebSocket remote-control server
  (`android/remotecontrol/.../server/RemoteWebSocketServer.kt:10`–`:18`).
  This on-device Kotlin server cannot adopt a Rust feature alone.
- **My Home Assistant:** aiohttp-based platform extension endpoints; those use
  Home Assistant's hosting contract rather than a standalone Rust HTTP server.
- **Librespot:** Hyper-based Spotify HTTP clients and discovery HTTP server
  (`discovery/src/server.rs:318`), an upstream library/protocol implementation.
- **Utility/reference servers:** rns-rs VPS history dashboard, Rustentia embedding
  demo, Talia Python dashboard, Paravoid compatibility/provisioning/reference
  servers, and standalone simple-hearth-monitor tools. Test-only Flask/aiohttp/
  stdlib HTTP servers also remain in service E2E fixtures. These should not be
  mechanically folded into the Rust server module.

No active Actix-web, Warp, Rocket, Poem, Salvo, Tonic, Express, Fastify, Koa,
Hono, FastAPI, Django or Spring server declaration/use was found in the reviewed
application inventories. Flask/aiohttp/Ktor and language-standard HTTP servers
above are positive findings. Broad generated vendor matches were not classified
as project adoption. Client libraries (reqwest, tokio-tungstenite), database
drivers/pools (SQLx, rusqlite, r2d2, sqlite-vec), and tracing/filter configuration
remain intentional ownership or separate roadmap work; presence alone is not
a migration regression.

The earlier "all services" statement applies to **in-scope Rust services and
applicable implemented modules**, not every project under lelloprojects, every
auxiliary Python/Node/Kotlin process, or eliminating all backend dependencies.
The broad audit also detects the newly active Pezzottflix Governor scope above.

### Repository inventory

“None detected” means no direct Axum or named server-framework match in the
reviewed source inventory; it does not certify absence of all backend libraries.

| Project | Observed branch / revision | Finding |
| --- | --- | --- |
| accordomi | `master` / `80bbc4d5` | None detected in reviewed inventory. |
| amperino | `master` / `999d540f` | None detected in reviewed inventory. |
| android-identicons | `master` / `66402864` | None detected in reviewed inventory. |
| android-keys | `main` / `df0d438f` | None detected in reviewed inventory. |
| android-simple-ephem | `master` / `f0d6d2b8` | None detected in reviewed inventory. |
| android-simplebpmdetector | `master` / `6b6cfd81` | None detected in reviewed inventory. |
| androidoscopy | `master` / `fdb16f71` | No Axum; tower-http declaration candidate; WebSocket clients/test infrastructure. |
| casuccia | `main` / `075a4ca6` | Python stdlib HTTP capture receiver. |
| chrome-utils | `master` / `fdc54495` | None detected in reviewed inventory. |
| creticulum | `master` / `6ccca85b` | None detected in reviewed inventory. |
| crumbles | `master` / `de7556ae` | No Axum; Tower tests/SQLx/client libraries; cached Android sources excluded. |
| dotfiles | `master` / `b03200e3` | None detected in reviewed inventory. |
| fausto | `master` / `b2ccfd60` | Tower RequestId compatibility/custom services; database/vec extension local. |
| favzetto | `master` / `6bcbc8fc` | No Axum; Tower tests/SQLx/client libraries. |
| halloo | `master` / `(unborn)` | No committed HEAD / no reviewed source. |
| homelab | `master` / `77123cc3` | Python aiohttp/Flask/stdlib servers and Go net/http gateway. |
| km-g6-control | `main` / `d6637e00` | None detected in reviewed inventory. |
| lello-auth | `master` / `228edd17` | tower-cookies production API; tower-http declaration candidate; Python fixture. |
| lellodesign | `master` / `3fc683b6` | None detected in reviewed inventory. |
| lelloman-com | `master` / `e4a20981` | None detected in reviewed inventory. |
| lellostore | `master` / `34fe9e2a` | No Axum; tower-http declaration candidate; read-only/deferred. |
| librespot | `dev` / `939dc5ee` | Hyper Spotify clients/discovery server; upstream library. |
| lxmf-rs | `dev` / `f8f55201` | None detected in reviewed inventory. |
| lxst-rs | `master` / `8ee20127` | None detected in reviewed inventory. |
| maruzzella | `master` / `dd4572f9` | None detected in reviewed inventory. |
| mat | `master` / `e3c2f965` | None detected in reviewed inventory. |
| meteonesto | `master` / `471e9d06` | No Axum; Python fixture and SQLite driver intentional. |
| mimmo | `master` / `af69f53a` | None detected in reviewed inventory. |
| my-home-assistant | `master` / `b8ed4e7d` | aiohttp Home Assistant extension endpoints. |
| napulicchio | `main` / `e3338f61` | None detected in reviewed inventory. |
| nomadnet-rs | `master` / `af680bfa` | None detected in reviewed inventory. |
| nvidia-gpu-data | `master` / `872292b8` | None detected in reviewed inventory. |
| observo | `master` / `9cf709bd` | Node HTTP worker API; Rust has no Axum. |
| paranza | `master` / `50e621c4` | No Axum; Tower tests/SQLite/client libraries. |
| paravoid-android | `master` / `55f64af7` | Python stdlib provisioning/compatibility/reference servers. |
| peerlo | `master` / `1ce84815` | No Axum; Governor legacy tests/client/database libraries. |
| pezzottflix | `master` / `71a13092` | Tower static/compression, active Governor gap, Android Ktor; CLI schema shared. |
| pezzottify | `dev` / `e3e6fba0` | Tower HTTP static files; Python mocks; client/database libraries. |
| pezzottify-downloader | `master` / `22ffc5c3` | Hyper Unix serving/client proxy; Tower RequestId; Python SQLite local. |
| pezzottv | `master` / `21a89218` | None detected in reviewed inventory. |
| projectino | `master` / `ecd796fd` | None detected in reviewed inventory. |
| pv-estimator | `dev` / `1d0d9ea7` | None detected in reviewed inventory. |
| quentin-torrentino | `master` / `1f76d881` | Direct Axum; excluded migration. |
| reticulum-pin | `(detached)` / `3f95b472` | None detected in reviewed inventory. |
| rns-rs | `dev` / `afe71800` | Python stdlib VPS dashboard; SQLite driver local. |
| rns-topo | `master` / `1f3e9fcc` | None detected in reviewed inventory. |
| ronomepo | `main` / `aab408cb` | None detected in reviewed inventory. |
| rustentia | `master` / `1f854ba9` | Python stdlib embedding demo; SQLite driver local. |
| rusty-sweeper | `master` / `ac4ea52a` | None detected in reviewed inventory. |
| scannerino | `main` / `cb418763` | None detected in reviewed inventory. |
| sct | `master` / `d6b81bb6` | Tower static files; PostgreSQL/SQLite drivers intentional; validation artifact excluded. |
| sim-rns | `master` / `1e73fd6a` | None detected in reviewed inventory. |
| simple-agents | `master` / `adee7a79` | No Axum; Tower tests/WebSocket clients/SQLx intentional. |
| simple-ai | `master` / `851ce0cd` | Python stdlib provider servers/aiohttp test runner; Rust has no Axum. |
| simple-android-assistant | `master` / `87a49956` | None detected in reviewed inventory. |
| simple-server | `master` / `c0d408bb` | Axum/Tower/Hyper internal implementation; expected. |
| sniffy | `master` / `d502d867` | None detected in reviewed inventory. |
| talia | `master` / `e37ac714` | Direct Axum engine and transport spike; Python dashboard. |
| torrentino | `master` / `81fdaa78` | Direct Axum service; separate from quentin-torrentino. |
| tufino | `master` / `fe53c5a9` | None detected in reviewed inventory. |
| vggt-capturer | `main` / `a1c17908` | None detected in reviewed inventory. |
| watch-rns-rs | `master` / `8bcf5003` | No direct application framework; cached upstream sources excluded. |
| wgtransport | `master` / `cdf73f99` | None detected in reviewed inventory. |
| wpe-the-wood-price-extractor | `main` / `b32003b2` | None detected in reviewed inventory. |

Standalone non-repository source directories were covered by the broad scan;
notably simple-hearth-monitor has a stdlib HTTP tool. Hidden control directories,
build/source caches, credentials and permission-denied runtime data were not
audited as application projects. This audit changed only shared documentation
and truthful tracker scope, with no consumer implementation/branch/dependency
changes or tests, no push and no deployment.


## Torrentino tracker inclusion — 2026-10-04

The matrix now includes **torrentino**, the replacement service, and has 18 rows.
**quentin-torrentino remains excluded by user request**; its historic adoption
entries do not authorize further migration. Talia remains outside the matrix.

Read-only inspection of Torrentino's clean active `master` at `81fdaa78d71d`
verified actual production calls to shared `logging::try_init`, `Signals`,
`Lifecycle::service` and `Lifecycle::run` in `crates/service/src/main.rs`.
Those two modules are Done; previous public-package migration evidence records
43 passing tests and a real-binary shutdown check. No tests were rerun for this
documentation update. The dependency remains public simple-server 0.1.0.

HTTP centralization and routing remain Pending: `main.rs` directly calls
`axum::serve`; `api.rs` uses Axum routing, middleware, extractors, body limits
(65,536 bytes for API routes; 4,096 for sessions) and WebSockets. Custom bearer
and session authentication needs migration. `store.rs` configures SQLx WAL,
foreign keys and a five-second busy timeout, rejects `user_version > 4`, and
executes `schema.sql`; SQLite policy, preflight and creation remain Pending.
Other modules are Pending until applicability is assessed; this does not mean
unused features should be introduced. No consumer code changed.

The all-project audit above remains a dated snapshot of the earlier 17-row
matrix. Its statement that 16 non-excluded services have no direct Axum applies
to those previously migrated services, not to newly included Torrentino.
Current Step 10 totals: 11 Done, 1 Partial, 5 N/A, 1 Pending. Current Step 07e
totals: 11 Done, 4 N/A, 3 Pending (Torrentino, deferred LelloStore and excluded
Quentin Torrentino).

## Remaining backend exposure cells — 2026-10-04

The main matrix now carries the backend findings from the all-project audit in
each affected service's last cell, rather than reporting None when only direct
Axum was absent. The column is renamed **Remaining backend exposure** in both
trackers. Pezzottflix explicitly lists static-file/fallback services, compression
and its active Governor HTTP limiter; Step 10 remains Partial.

Static files, mutable cookies, request-ID compatibility and Unix HTTP/proxy
boundaries are recorded for the other affected services. Suspected stale direct
dependencies are labelled verification candidates, not confirmed active usage.
Auxiliary Node/Python/Kotlin servers are explicitly labelled outside Rust scope;
ordinary driver/client dependencies and intentional test utilities are not
automatically classified as migration work. Completed module statuses remain
unchanged. These cells use the 4 October source audit evidence above; no consumer
code changed and no runtime tests were rerun for this documentation correction.

## Static files library checkpoint — 2026-10-04

Implemented at `dbc7f68` on simple-server master, prepared version 0.1.3.
The optional `static-files` feature exposes owned `web::static_files::StaticDir`
and `StaticFile`; no backend file-serving types appear in the public API.
See [the contract](static-files.md) for streaming, MIME, GET/HEAD, ranges,
modification-time conditionals, directory indexes/redirects, explicit SPA/404
fallback, precompressed siblings, symlink ownership and error handling.
No cache-header or dynamic compression middleware is added implicitly.

`bash scripts/check` passes: 625 test executions, strict all-feature/all-target
Clippy, feature isolation, no-default-feature compilation and rustdoc with
warnings denied. Static-file tests include 42 differential requests against the
previous Pezzottify configuration and a real TCP/shutdown test. Registry package
contents were inspected and `cargo publish --dry-run --locked --registry crates-io`
passes from clean committed source. Publication is pending explicit approval.

Pezzottify was tested in an isolated worktree from clean committed dev
`21641eec`; unrelated work appeared in its original checkout and is preserved.
Baseline auth and permissions: 44 passed. The new production-frontend HTTP
contract test passes against the old backend. The canary replaces ServeDir/
ServeFile, removes direct Hyper/Tower HTTP declarations and moves Tower into
dev dependencies. Public-release consumption and integration are still pending,
so the last-cell leftovers have not yet been removed.

### Pezzottify static-file canary ready; publication pending

Prepared consumer commits `e83df8e9` (implementation) and `d9a13b20` (verification)
are on `migration/shared-static-files`, refreshed onto concurrent dev commits
`bf9912ae` and `dc803924`. Production Rust checks were rerun on the combined CI-fix
source; the latter commit changes only an unrelated Python E2E test. Original dev
remains at `dc803924`: do not interpret preparation as completed adoption.

Verified with an uncommitted command-line Cargo source patch:

- Full fast unit run: 1,170 passed, two ignored, one failure in the existing
  scheduler test's 200 ms execution assumption. The isolated retry passes.
- All 34 integration suites: 342 passed, 32 existing ignores. External deletion
  of build artifacts interrupted execution after 11 suites/145 passes; the
  remaining 23 suites were rebuilt outside the projects directory and passed
  197 tests on the combined CI-fix source. No missing-binary result is counted
  as a passing test.
- Frontend/auth/permission checks on the combined source: 45 passed.
- Formatting, strict production Clippy and locked all-target/all-feature checking
  pass. Existing fixture warnings and num-bigint-dig future-compatibility warnings
  remain; strict all-target Clippy is not claimed.
- Metadata verifies no direct Axum, Hyper or Tower HTTP declaration; Tower is
  dev-only. Static frontend configuration and API middleware ordering remain
  unchanged. No user databases, deployment or remote branches were changed.

The public 0.1.3 candidate is source `dbc7f682f9656c1b29c6a52dfc011a94de27087a`,
archive SHA256 `0c3c1347b1c85484b5d4ba18cd6d886688b8a1dc2d94d84bc596a6abec128ddb`.
A fresh packaging run reproduces the dry-run archive hash. Publication approval
was requested because the new API is absent from existing registry releases.
The consumer's candidate lockfile still reflects the local test override; registry
resolution/checks, rebasing dev onto the migration branch, and consumer worktree/
branch removal remain pending. The remaining-backend cell deliberately retains
current dev's static-file/Hyper cleanup items until integration.

Shared-library implementation and tracker updates are integrated into master.
The library worktree/branch and owned temporary build artifacts are cleaned up;
the committed consumer worktree/branch is retained for the pending release.

## Static files publication and Pezzottify integration — 2026-10-04

Published public `lelloman-simple-server` 0.1.3 with explicit user authorization
from clean tested source `dbc7f682f9656c1b29c6a52dfc011a94de27087a`. Cargo confirmed
registry availability. Independent download verifies SHA256
`0c3c1347b1c85484b5d4ba18cd6d886688b8a1dc2d94d84bc596a6abec128ddb`, the embedded source
commit and static-file source. Full library checks already passed 625 test
executions, strict Clippy, feature isolation and rustdoc.

Pezzottify's migration branch was refreshed onto current dev `9f644b79`, retaining
its rewritten history, async-trait lockfile update and Android CI changes. Final
consumer commit `aee770c9` locks the actual crates.io package/checksum, with no
source override. Registry verification: 45 frontend/auth/permission tests pass;
formatting, strict production Clippy and locked all-target/all-feature checking
pass. The earlier source canary passed all 34 integration suites (342 passed,
32 existing ignores); its broad unit run passed 1,170 with one scheduler timing
failure that passed on isolated retry, plus two ignores. Those earlier results
are not represented as a fresh complete registry suite.

Dev was rebased onto the migration branch and ancestry/tree equality verified.
Unrelated active Push work was preserved. Its overlapping route_builder.rs local
edits were retained while applying only the two static-file API substitutions;
all other recorded local file hashes were unchanged. No user work was included
in the migration commits. The clean migration checkout was tested; ongoing
uncommitted Push work has a separate verification scope: a final `cargo check
--locked` also passes on the restored original checkout, including those local
edits. The 45 registry tests cover the committed migration checkout.

Current production routes now use owned StaticDir with the same directory-index
and SPA fallback semantics. Direct Hyper and Tower HTTP dependencies are removed;
Tower is dev-only. The remaining-backend cell is now None for this Rust scope.
Library release and consumer migration worktrees/branches were removed after
verification, and owned temporary builds/logs were cleaned up. No Git push or
service deployment was performed. This section supersedes the earlier publication
and consumer-integration pending notes.

## Androidoscopy backend dependency cleanup — 2026-10-04

Active clean master started at `fdb16f7`; cleanup commit `171cf75` was implemented
in an isolated worktree and master rebased onto it. Ancestry/tree equality was
verified and the owned migration worktree/branch removed. No other branches, user
data, remote history or deployment were changed. See consumer
`docs/backend-dependency-cleanup.md` for detailed scope.

Removed unused direct Tower HTTP 0.5/fs declaration and its lockfile entries,
including unused http-range-header. Dashboard files are rust-embed assets handled
with owned simple-server HTTP types, not disk directory/file serving. Therefore
static-files is N/A here: no new feature or public-version bump is introduced.
Existing public simple-server 0.1.0 and pairing JNI scope remain unchanged; the
reviewed optional disk API is public 0.1.3 source `dbc7f68`. Transitive Tower HTTP
0.6 through reqwest and intentional client/asset/TLS libraries are not direct
backend migration leftovers. Logging target strings preserve configuration
compatibility and are not dependency references.

Verification: baseline/final full server suites match at 79 passes each; locked
all-target check and binary build pass; all six controller/legacy WS/legacy TLS
SIGINT/SIGTERM shutdown cases pass with live sockets. Standalone E2E passes five
unit plus seven full-stack tests. Its ignored generated lockfile is not committed.
Strict production Clippy stops on the identical pre-existing derivable Config
Default implementation; formatting output exactly matches baseline failures in
untouched Rust files. These are reported baseline limits, not green checks.

Both tracker remaining-backend cells now read None for Androidoscopy's Rust
scope. Earlier audit's stale declaration candidate is resolved. Existing module
statuses are unchanged. Central trackers are committed separately on
simple-server master; owned temporary builds/logs are cleaned up.

## Fausto backend dependency cleanup — 2026-10-04

Active clean `master` started at `b2ccfd601d0d`; implementation was tested and
committed in the dedicated `migration/backend-cleanup` worktree, then original
`master` was rebased onto it at `006004d`. Ancestry and identical trees were
verified. Owned worktree/branch, frontend dependencies/fixtures, build cache and
logs were removed after integration; no push or deployment was performed.

Production correlation and federation/API errors now use shared opaque
`correlation::HeaderRequestId`. UUID generation, repeated/empty/long/non-text
headers, response overrides and conflicting typed-extension precedence retain
Fausto's policy. Forty before/after differential cases compare the original
Tower HTTP layers with the shared implementation. Tower HTTP is dev-only for
that oracle, with only its request-id feature enabled; its production declaration
and unused filesystem/propagate-header features are removed. The direct normal
dependency tree confirms this, and the source audit finds no production Axum,
Hyper or Tower HTTP imports.

The handwritten Tower embedded-file service was replaced by production
`Router::fallback(serve_embedded)`, using shared owned HTTP types. Application
rust-embed packaging, MIME detection, root/index, SPA/missing-file fallbacks,
POST behavior and feature-disabled 404 body remain. The handler strips HEAD
bodies in-process; network HEAD was already bodyless, and final loopback checks
confirm status, MIME, exact content length and empty body. Disk static-files is
N/A for these embedded assets; the unchanged public `=0.1.2` already provides
all required APIs. There is no new shared-library release.

Generic Tower Layer/Service composition of Fausto's per-IP adapter around the
shared token bucket remains an intentional boundary. Outbound reqwest and
embedded asset packaging remain application-owned. Transitive Axum/framework
dependencies are expected; the final table cell tracks outstanding migration
work, not all transitive dependencies. There is no remaining identified backend
migration gap for Fausto in this scope.

Verification:

- Unmodified server baseline: **218 passed**, one existing ignored doctest;
  all-target Clippy completes with warnings, formatting passes.
- Before/after embedded fallback contracts cover GET/HEAD/POST, root, SPA/query
  paths, missing assets, disabled UI and exact CSS/binary bytes. Production-router
  loopback checks include request-ID propagation and graceful shutdown.
- Final default server suite: **220 passed**, one ignored doctest. With web-ui
  and Swagger: **221 passed**, one ignored doctest, including six live process
  lifecycle cases and the WebSocket transport suite.
- All-target Clippy with web-ui/Swagger completes with warnings (not a strict
  warnings-as-errors pass); no-default-feature compilation, formatting and diff
  checks pass. Integrated master matches the tested migration tree.
- The unchanged frontend still fails TypeScript checks and standalone Vite build
  (`import.meta` in LoginView), as documented in the original migration. Optional
  Rust/HTTP checks use explicit temporary HTML/CSS/binary fixtures, not a working
  production UI build. Docker E2E was not rerun.

Fausto's active README pin was corrected from stale `=0.1.0` to actual `=0.1.2`.
Consumer details: `fausto/docs/simple-server-migration.md#backend-cleanup-2026-10-04`.

## Lello-auth backend dependency cleanup — 2026-10-04

Active `master` started at `228edd17bfb8`. A dedicated
`migration/backend-cleanup` worktree held the declaration-only cleanup, tested
and committed at `2e42c18`. Original master was rebased onto that branch;
ancestry and identical tested trees were verified. The three original untracked
identity-provider research files were preserved and their hashes verified.
Owned worktree/branch, build caches, logs and scripts were removed after
integration. No push, deployment or shared-library release was performed.

The `lello-auth-axum` production manifest's unused Tower HTTP cors/trace
declaration is removed. The complete Rust source audit finds no Tower HTTP
imports or middleware installation. CORS remains Caddy-owned; Rust tracing/CORS
applicability does not change. Direct Tower usage is exclusively test ServiceExt,
so its declaration moves to dev-dependencies. The lockfile removes two dependency
edges, with no package version changes. The direct normal dependency tree
confirms neither Tower nor Tower HTTP is declared by the HTTP integration crate.
Transitive Tower/HTTP/Axum dependencies still exist, as expected.

**Cookie migration remains pending.** The shared integration crate, production
server, embedded example and external-OIDC example still use Tower Cookies
Cookie/Cookies, CookieManagerLayer and SameSite. Shared public
`lelloman-simple-server =0.1.0` supplies an extraction compatibility adapter, not
an owned mutable jar/middleware contract. Its auth cookie extractor reads
credentials and cannot replace output deltas or unauthenticated browser CSRF
and device-link flows. No cookie import is renamed to falsely imply adoption.

The needed shared contract covers incoming parsing, clone/shared mutation,
add/remove deltas appended as distinct Set-Cookie headers, cookie attributes and
missing-layer rejection. Lello-auth must keep its session/CSRF/OIDC/device-link
policy, including __Host- host-only cookies, Secure/HttpOnly, Path=/, SameSite
Lax/Strict, lifetimes and deletion rules. Details are recorded in
`lello-auth/docs/BACKEND_DEPENDENCY_CLEANUP.md`. The table's last cell now lists
only this pending cookie boundary, removing the completed Tower HTTP item.

Verification:

- Baseline and final `cargo test --locked --workspace --all-targets` each:
  **969 passed, 14 ignored** external database cases. This includes real-loopback
  admin/login/logout, device-link, OAuth, OIDC lifecycle, CSRF/password, profile,
  reauthentication and TOTP suites, plus core database and token contracts.
- CI's Rust **1.88.0** strict all-target workspace Clippy passes before and after,
  with its existing `large_enum_variant`/`too_many_arguments` allowances.
- The newer local compiler's strict Clippy fails before and after with the same
  two pre-existing `cmp_owned` findings at server main.rs:887 and :903; these are
  not migration failures. Formatting, diff checks and the Python CI contract
  pass before and after.
- No application source or cookie behavior changes. No new tests are added for
  this unused-declaration cleanup. External PostgreSQL ignores, excluded
  standalone examples/auth-helper, browser release gates and Docker E2E were
  not rerun. The existing public =0.1.0 dependency remains unchanged.

## Owned mutable HTTP cookies — 2026-10-04

Simple-server now implements optional `cookies`, exposing owned
`web::cookies::{Cookies, CookieManagerLayer, CookieManager}` and its response
future. The feature enables web and the standard cookie data library, not the
external Tower Cookies dependency. The owned jar/middleware implementation does
not wrap or re-export Tower Cookies types. Standard cookie values/builders,
SameSite/Expiration and time types are exposed from the cookie data library.
Application session, CSRF, cookie attributes and authentication policy remain
application-owned. See [the contract](cookies.md).

The layer installs a fresh lazy request jar, shares mutations across clones and
head-only handler/middleware extraction, and appends deltas to existing
Set-Cookie headers after a successful downstream response. It preserves repeated
header parsing/percent decoding, duplicate-name precedence, mutation/removal
semantics, invalid-header filtering and missing-layer 500 text. Read-only requests
emit no cookies. Request/response bodies, streaming, metadata, readiness, inner
service errors and cancellation are preserved. Detached jars, delta snapshots
and manual append support custom composition. The old tower-cookies extraction
adapter remains available unchanged for unmigrated consumers.

Verification:

- Unmodified baseline: **366 all-feature tests/doctests passed**, strict
  all-target/all-feature Clippy and formatting passed.
- New suite: **11 tests pass**, including **48 differential cases** against
  Tower Cookies: six incoming-header cases times eight mutation policies.
  Coverage includes absent/duplicate/repeated/encoded/non-text/malformed input,
  replacement/removal/cancellation, explicit expiry and attributes, invalid
  output filtering, existing Set-Cookie preservation and response metadata.
- Real loopback login/read/logout checks verify separate cookie fields,
  __Host- attributes, percent decoding, read-only output and explicit deletion.
  Additional tests cover shared middleware/multiple extractor mutations,
  per-request isolation, concurrent jar clones, downstream extractor rejection,
  detached composition, service readiness/errors, untouched response streams
  and pending-future cancellation.
- Complete `scripts/check`: **657 passed test executions**, no ignores/failures;
  strict all-target/all-feature Clippy, formatting, no-default-feature builds,
  existing feature matrix and warnings-as-errors rustdoc pass. Added minimal
  cookies-only and cookies/compatibility/HTTP-harness test configurations.
- Minimal cookies-only normal dependency tree contains **no tower-cookies**.
  No dependency versions or lockfile changed.

The implementation was developed in a dedicated `feature/owned-cookies`
worktree from clean master `2401f2c`, committed and integrated by rebasing original
master onto that feature branch. Ancestry and the tested tree were verified;
owned worktree/branch, build caches and logs were removed after integration.
No push, deployment or package publication was performed.

At the implementation checkpoint, consumer adoption and a new crate release
were pending: published 0.1.3 does not contain this capability. The subsequent
0.1.4 publication and verified Lello-auth canary below supersede that checkpoint.
No service was marked Done merely because the library feature existed.

## Mutable cookie release 0.1.4 and Lello-auth canary — 2026-10-04

Public `lelloman-simple-server` **0.1.4** was published with user authorization
from clean tested source `40c41291d9b0f90705c93f8528eead16a9eb2477`. Package inspection checked 130 files,
including cookie implementation/tests and excluding build/credential files.
Package verification and publication dry run passed before upload; Cargo
confirmed registry availability. Archive SHA256 is
`5526aea10c87982acc311e23401e6134f624f03d10470c982059163c325b595a`. Independent public download matches both the archive checksum
and embedded Git commit. The root and standalone SQLite fixture lockfiles both
record the new version. Complete library validation passes **657 test executions**,
strict Clippy, feature isolation, formatting and warnings-as-errors rustdoc.
The archive/source revision is recorded in [the publishing record](publishing.md).

Lello-auth's active master started at `2e42c18`. A dedicated
`migration/owned-cookies` worktree held the implementation, tested and committed
at `e0faab8`; original master was rebased onto it. Production server,
shared session/authenticator/UI handlers, `session_layer()`, HTTP test fixture
and embedded/external-OIDC examples now instantiate/extract shared owned
`web::cookies` types. All direct package pins (workspace, core and examples) use
public **=0.1.4**. No path/Git override or sibling library is required. Workspace
and three example lockfiles each resolve one shared package whose checksum
matches the independently verified archive, and no Tower Cookies package.
The webhook example had no cookie use; its unused compatibility feature is
removed without adding the new feature.

Session, CSRF, authenticator/device-link and OIDC policy, names/attributes,
expiry/deletion and middleware placement remain application-owned and unchanged.
Both new real-HTTP cookie tests pass before and after migration: host-only
__Host-/Secure/HttpOnly/Path=/, session Lax seven-day and CSRF Strict one-hour
lifetimes, read-only requests without deltas, logout header/database revocation,
protected-profile redirect and forged-CSRF replacement without authentication.
An initial new-test expectation was corrected against the baseline observed
`/login?redirect_to=/profile` redirect before migration, preserving that URL.

Verification:

- Unmodified baseline workspace tests/doctests: **991 passed, 22 ignored**;
  the two added HTTP cookie tests then pass against the old layer.
- Final workspace tests/doctests: **993 passed, 22 ignored**, including full
  real-loopback browser/authentication/OAuth/OIDC/device-link/TOTP suites and
  server process lifecycle/database contracts. The 22 existing ignores are
  external PostgreSQL cases and example doctests.
- Baseline/final strict workspace all-target Clippy passes on CI Rust **1.88.0**
  with its two existing allowances. Formatting and Python CI contract pass.
- All three excluded standalone HTTP examples compile before and after.
  Existing example/local-compiler warnings remain; not claimed strict-clean.
- Source audit finds no direct Axum, Tower HTTP or Tower Cookies imports in
  production, tests or examples. Historical lello-auth-axum package naming,
  generic Tower test ServiceExt, standard cookie value types, outbound reqwest,
  proxy CORS and transitive framework implementation are intentional boundaries.
- External PostgreSQL fixtures, unrelated auth-helper tests, Docker/browser
  release gates and deployment were not run. Consumer details:
  `lello-auth/docs/BACKEND_DEPENDENCY_CLEANUP.md#owned-cookie-adoption--2026-10-04`.

Integration ancestry and tested trees were verified for both local master
branches. Three original untracked identity-provider research files were
preserved with matching hashes. Only the owned release/migration worktrees,
branches, test/build/package artifacts, logs and temporary scripts were removed;
pre-existing unrelated worktrees were untouched. Package publication does not
include a Git push or deployment. The last table cell for Lello-auth now says
None because its identified cookie compatibility boundary is migrated, rather
than because a Cargo feature was merely selected.

## LelloStore SQLite applicability and backend cleanup — 2026-10-04

The user resumed the previously deferred LelloStore work. Active clean `master`
started at `061a7cc0c7864b334c24cd05c813ef3fbbf164c2`; migration commit
`343353c9ef3d1c69646255a658957094f2ed8e43` is integrated into `master`.

07e is **N/A**: all production tables and indexes are created by the authored
SQLx versioned migration files. `backend/src/db/mod.rs::run_migrations` already
uses shared migration preflight before running SQLx. There is no independent
unversioned bootstrap or structural comparison policy to migrate. SQLx owns its
internal migration ledger. The separate 07b adoption remains Done.

Removed the unused direct Tower HTTP fs dependency and its 0.5 lockfile package.
Moved direct Tower 0.4 to dev-dependencies for the HTTP tracing test. No direct
Axum/Hyper/Tower HTTP source APIs remain. Embedded frontend serving and APK
range/stream responses already use owned shared HTTP types. Remaining exposure
is **None**; driver/client dependencies and the shared library's internal backend
packages remain intentional. Public simple-server stays pinned at 0.1.0, checksum
`1f3187c81c94701cd041df7ab17ce968b73c967db77c551170e3c24960b6c77a`; shared library
reviewed at `64795a6`. No new API or release was needed.

Baseline and final full all-feature backend tests: **227 passed, zero failed,
nine existing ignores**. Initial final run failed the existing fake-aapt2 50 ms
timeout assertion; it passed in isolation and the subsequent full run. Locked
all-feature check, strict all-target/all-feature Clippy, formatting and diff
checks pass. Existing frontend/dist assets were copied into the isolated
worktree; frontend rebuild, Android interoperability and environment-dependent
ignored tests were not run. No tests added for dependency-only cleanup.

Consumer evidence: `lellostore/docs/step-07e-sqlite-schema.md`. Original master
was rebased onto the dedicated migration branch; tested/integrated trees and
ancestry verified, temporary worktree and branch removed. Original checkout is
clean; its pre-existing detached release worktree is preserved. No pushes or
deployments. Current 07e matrix totals: **11 Done, 5 N/A, 2 Pending** (Torrentino
and excluded Quentin Torrentino); earlier totals remain dated checkpoints.

## Observo remaining-backend scope audit — 2026-10-04

Clean active `master` `9cf709b` was inspected across the server, content extractor,
link scorer and Node worker. Rust source/manifests have no direct Axum, Hyper,
Tower HTTP or Tower Cookies usage; Tower remains test-only. SQLite bootstrap
creation is already adopted. No application implementation change was needed.

The former remaining cell described the separate Node worker management API.
It is an intentional runtime boundary, not pending Rust adoption. The Node HTTP
listener exposes profile status/export/import and operates on live Puppeteer
state, task draining, busy admission and browser restart. Migrating it would
require a separately scoped worker architecture change; no claim is made that
this Node API now uses simple-server. Its code and behavior remain unchanged.
The pending Rust exposure cell is now **None** in both trackers.

Consumer evidence: `observo/docs/backend-migration-scope.md`, committed as
`0ba9b3f5bc4115d7ffdcb685e9abd14bcba4cbc7`. All **31 existing Node worker tests
passed**, zero skips/failures; these do not cover browser-backed management
HTTP operations. No Rust suite/browser E2E rerun for this documentation-only
audit. Existing 07e results remain historical. Shared library inspected at
`3c8c10f`; consumer dependencies remain unchanged at public 0.1.1.

Master was rebased onto the isolated audit branch, identical trees and ancestry
verified, and the owned worktree/branch removed. No pushes or deployments.

## Pezzottflix static serving and configured HTTP limiter — 2026-10-04

Active clean `master` started at `71a1309`; consumer migration
`ccbdc876ffd8640feaf7d635461eb3aaba835c6d` is integrated into master. Shared
public 0.1.4 source `40c41291d9b0f90705c93f8528eead16a9eb2477`, checksum
`5526aea10c87982acc311e23401e6134f624f03d10470c982059163c325b595a` is now
pinned in the server workspace and documented in README. The standalone
RaiPlay CLI stays unchanged at 0.1.2.

Production frontend/SPA fallback uses owned StaticDir with directory indexes
and explicit index.html fallback. Missing assets, relative root resolution,
GET/HEAD, methods, ranges and conditional requests retain their existing policy.
The factory called by production is compared against Tower HTTP 0.5: **120
requests match full headers, status and response bytes**, including traversal
and redirects. Missing fallback index returns 404.

Configured /v1 HTTP admission uses shared Budget and ExtraIdleCredit, preserving
Governor 0.6 refill/idle behavior, per-IP/global/disabled modes, peer fallback,
map/locking, cleanup after attempted requests, zero-setting assertions, plain
429 body and Retry-After floor rounding. Accepted extreme rates above one
billion preserve their old zero-debt behavior. **15,000 deterministic decisions
match Governor, including exact retry durations**. Existing production-router
and cleanup/refill tests remain green. Governor and Tower are now test-only;
unused direct nonzero_ext is removed. Step 10 becomes **Done**.

Baseline full workspace: **603 passed, zero failed, three existing ignores**.
Final full workspace: **605 passed, zero failed, three existing ignores**.
Locked workspace check and warning-capped all-target Clippy pass. Baseline and
final strict Clippy have identical diagnostic counts/messages (34 library, 39
library-test errors); repository formatting differences also persist. Changed
rate-limiter formatting and diff checks pass; unrelated formatting is preserved.
No frontend/Android/browser/container E2E or standalone CLI rerun. Evidence
is in `pezzottflix/docs/backend-cleanup.md` and the updated Step 10 document.

The remaining cell now lists only **gzip CompressionLayer**. It stays unchanged
in its original position around the combined router; simple-server has no
response compression API yet. Runtime Tower HTTP enables only compression-gzip;
fs is enabled for the old static-service test oracle. Android Ktor/Netty serving
is a separate runtime boundary, recorded here rather than as pending Rust work.
This cleanup does not claim that all backend dependencies have been removed.

Master rebased onto the isolated migration branch; ancestry and identical tested
trees verified. Owned worktree/branch removed, unrelated recovery branch
preserved. No pushes/deployments. Both central trackers updated consistently.

## Owned gzip response compression prepared — 2026-10-04

Optional `compression` provides owned web compression layer, service and future
returning the shared Body. Gzip negotiation/quality/toggle, u64 size thresholds,
custom response-metadata predicates, default MIME exclusions, encoded/range
guards and streaming/readiness/error/cancellation behavior are supported.
No middleware is installed implicitly. See [the contract](compression.md).

Full `scripts/check` passes **687 test executions**, zero failures/ignores, strict
all-target/all-feature Clippy, isolated features, fixture, formatting, rustdoc.
Ten new tests include 1,280 Tower HTTP 0.5 differential cases, gzip bytes and
header contracts, policies, service/body errors, cancellation and TCP/HEAD.
The newer backend deduplicates Vary Accept-Encoding tokens; token semantics
match the legacy implementation. gRPC-prefix exclusion is explicitly retained.
Version 0.1.5 is prepared for publication; it is not yet a published/adopted
release. Pezzottflix migration is being verified separately before integration.

## Pezzottflix owned compression canary prepared — 2026-10-04

Shared library source `4a0f06ea61719ca5e9219f006988824c09d75bf6` is committed and
integrated into simple-server master. Version 0.1.5 publication dry run/package
verification pass; inspected archive contains 132 files, SHA256
`3df80ad94d2f2accc506f153787c81b6c11c13a0da670212c1e233d54ddcbab7`, and
embeds that Git source. The release has **not been published**; user approval
has been requested for the permanent public publication.

Pezzottflix candidate `e891348` on isolated `codex/pezzottflix-compression`
replaces the gzip production layer with the shared owned layer at its existing
position. Direct Tower HTTP is now dev-only. Three canaries include 420 old/new
policy cases, real TCP requests through the actual production router, and
compressed frontend/SPA/range fixtures. Full candidate suite: **608 passed,
zero failed, three existing ignores**; baseline 605 passed/three ignores.
Warning-capped all-target Clippy passes; strict lint baseline findings remain
(34 library/39 library-test errors). No browser/Android/container/CLI rerun.

Candidate verification currently uses the exact local release-source worktree;
that path is intentionally **not integrated** into Pezzottflix master, which
remains `ccbdc87`. After approval, publish the clean committed 0.1.5 package,
independently verify its public archive/source/checksum, replace the temporary
consumer path with a registry pin, regenerate/verify Cargo.lock, rerun checks,
commit and rebase master onto the final migration, then remove retained owned
worktrees/branches/builds. The remaining cell records this pending integration;
it does not claim a completed consumer adoption. Evidence lives in the
candidate's `docs/response-compression.md`. No Git pushes or deployments.

## Published gzip compression and Pezzottflix completion — 2026-10-04

The user explicitly authorized publication. Public `lelloman-simple-server`
**0.1.5** is published from clean tested commit
`4a0f06ea61719ca5e9219f006988824c09d75bf6`; registry availability confirmed.
Independent static.crates.io archive download verifies SHA256
`3df80ad94d2f2accc506f153787c81b6c11c13a0da670212c1e233d54ddcbab7` and
the embedded Git source. Inspected package has 132 files; package verification
and publication dry run passed. Shared full validation passed **687 test
executions**, zero failures/ignores, strict Clippy, feature isolation, fixture,
formatting and warnings-as-errors rustdoc. See [release evidence](publishing.md)
and [the compression contract](compression.md).

Pezzottflix active clean master `ccbdc87` was rebased onto the tested migration
branch, including local canary `e891348` and final public-registry commit
`5c83181b35de8de8fe22b5238bc17d5c95e2eae7`. Production gzip now uses owned
CompressionLayer in the original position around API/health/frontend routes.
Direct Tower HTTP and Governor are dev-only differential oracles; no direct
Axum, Tower HTTP or Governor normal dependency remains. Other driver/client
packages and Android Ktor/Netty serving remain intentional runtime boundaries.
The remaining Rust backend exposure cell is **None** in both trackers.

Temporary consumer path removed. Exact registry 0.1.5 and its source/checksum
verified in Cargo.lock; README build pin updated. The standalone RaiPlay CLI
and its lockfile remain unchanged at 0.1.2. **Final full workspace against the
public crate: 608 passed, zero failed, three existing ignores**, matching the
local canary. Baseline: 605 passed/three ignores. Three added tests include
420 legacy-policy comparisons, real production-router TCP gzip/identity/refusal/
HEAD/health and TCP SPA/missing-asset/range checks. Shared tests separately
cover 1,280 legacy cases, streaming/readiness/body/service errors/cancellation.
Vary token semantics are preserved; duplicate Accept-Encoding tokens may be
deduplicated by the newer backend. Explicit gRPC prefix exclusions retain the
old predicate. Application policy/placement remains unchanged.

Published-dependency locked workspace check and warning-capped all-target
Clippy pass. Strict lint findings from baseline/local candidate persist (34
library and 39 library-test errors); strict Clippy was not repeated after
switching to identical public source. Unrelated formatting differences remain;
new canary module and diff checks pass. No frontend/browser/Android/container
or standalone CLI rerun. Consumer evidence is in
`pezzottflix/docs/response-compression.md` and updated `docs/backend-cleanup.md`.

Base branch ancestry and identical tested/integrated trees verified. Both owned
migration worktrees/branches and release/test temporary builds, logs and archive
were removed; original checkouts are clean and pre-existing recovery branch
preserved. Only the authorized crate publication changed remote state; no Git
push or deployment. Earlier prepared/pending records remain dated checkpoints.

## Downloader and Torrentino cleanup — 2026-10-04

This is the earlier preparation checkpoint. The later downloader publication and
completion below supersede its pending-release/integration status.

Scope: remaining direct HTTP backend dependencies, preserving application
protocols. Torrentino is the current replacement repository, not excluded
quentin-torrentino. Original branches were clean active `master`: Torrentino
`81fdaa78`, downloader `22ffc5c3`, shared library `ab61041`. Separate sibling
worktrees and migration branches were used throughout.

### Torrentino integrated

Master is `9850c1659ab060caf575f70d02580236b9ff0815`. The independent
`crates/service` package now consumes public `lelloman-simple-server =0.1.5`
(source `4a0f06ea61719ca5e9219f006988824c09d75bf6`, archive checksum
`3df80ad94d2f2accc506f153787c81b6c11c13a0da670212c1e233d54ddcbab7`).
Direct Axum and HTTP-body-util dependencies/imports were removed. Production
routes, handlers/extractors/rejections, responses, serving, body limits and
WebSocket upgrades/messages use owned APIs; mock HTTP fixtures do too.

Shared `auth::AsyncAccess` evaluates existing credential policy while keeping
plain application `Principal` extensions. Bearer header precedence and
case-insensitive scheme parsing, strict duplicate-cookie rejection, hashed
SQLite sessions, binding/expiry, CSRF, admin/resource/owner authorization remain
unchanged. The shared header helper preserves authenticated no-store responses.
Limits remain 64 KiB protected API and 4 KiB public session creation. Embedded
assets/security headers and database/worker behavior remain application-owned.

Baseline: 43 tests passed. Final: **44 tests**, formatting, diff checks and
**strict all-target Clippy** pass. New HTTP contracts check credential precedence,
authenticated fallback, cache headers and oversized body rejections. Existing
contracts cover owners, sessions, CSRF, WebSocket replay/expiry and binary shutdown.
**Real Chromium E2E passes** integrations, sessions, previews, clarification,
confirmation, format consent, delivery, movie identification and logout.
Master was rebased onto the migration branch; ancestry and identical tested tree
were verified. Temporary Torrentino worktree/branch were removed. Legacy root
workspace clients/tools are unchanged.

Request correlation, HTTP tracing, CORS and health are N/A after source review:
there is no request-ID protocol, HTTP trace middleware, cross-origin access grant
or served probe. SQLite policy/preflight/schema, fixed worker scheduling and
worker/budget applicability remain explicitly Pending; HTTP cleanup does not
claim their migration. Remaining HTTP backend exposure is **None**.

### Shared Unix HTTP release prepared

Shared `master` includes tested commit
`58e9b75` (version 0.1.6). Opt-in Unix serving accepts an application-bound
listener and drains active HTTP requests using explicit shared shutdown.
The application retains socket permissions/removal, signals/deadline and upgraded
WebSocket session policy. The owned pooled Unix HTTP/1 client streams bodies,
returns owned responses/errors and leaves forwarding/deadline/retry policy to
the caller. Non-UTF-8 paths and unsupported request targets fail explicitly.
`HeaderRequestId::uuid_v4()` provides UUID format without a consumer backend
request-ID dependency. See [Unix HTTP contract](unix-http.md).

Complete `scripts/check`: **706 test executions**, zero failures/ignores,
strict all-target/all-feature Clippy, isolated features, SQLite fixture,
formatting and warnings-as-errors rustdoc pass. Seven new transport/UUID contracts
cover repeated headers, request/response streaming, upstream body errors,
graceful draining, pre-requested shutdown, error classification, socket ownership
and Unix WebSocket text/binary/close. The clean committed publication dry run
verifies **135 package files**, including the Unix contract. **0.1.6 has not been
published; authorization was requested for the permanent public crate version.**

### Downloader canary pending public dependency switch

Candidate commit `d0321c8604e5a788d74314468481e0006da06000` in owned branch
`codex/downloader-backend-cleanup` uses the reviewed shared worktree candidate.
Original downloader master remains `22ffc5c3` until registry adoption/integration.
Child Unix serving now uses shared `web::unix::serve`; application socket 0700
permissions, stale-file removal, lifecycle 30-second grace and final cleanup are
preserved. HTTP proxy and three-second health check use shared `UnixClient`;
Host filtering, method/path/query/header forwarding, 502 errors and streaming
backpressure/failure behavior are retained. UUID request IDs use the shared
helper with a distinct application extension wrapper, preserving raw/repeated
headers, extension overrides, selected-ID context and response propagation.
Librespot HTTP primitives use the owned facade and its upstream Spotify client.
Tower HTTP/Hyper/Hyperlocal helpers move to development-only legacy oracles/mock
servers. The outbound Unix WebSocket protocol client remains tokio-tungstenite.
Python SQLite ownership remains N/A for Rust database modules.

An initial unchanged baseline run hit the credentials-file timing assertion;
the isolated case and full baseline rerun pass. Final candidate: **164 tests pass**,
including raw-header/extension legacy comparisons, streaming-before-EOF/body
failure, real-process lifecycle and TCP-to-Unix WebSocket forwarding. No service
deployment, remote push or user data changes occurred. Shared release/downloader
worktrees are intentionally retained for publication and final registry testing;
the downloader remaining cell lists what still exists on its integrated master.

## Downloader SCT and SimpleAI completion — 2026-10-04

Completed the downloader, SCT and SimpleAI cleanup before the requested broader
Torrentino pass. Torrentino was not changed during this batch. All three active
branches were clean `master`, selected from local status/history/tracking, not
remote HEAD. Baselines, isolated sibling worktrees, tested commits, rebasing the
original master onto each migration branch, identical-tree/ancestry verification
and removal of owned worktrees/branches followed the agreed workflow. No Git
push, service deployment or branch-protection changes were performed.

### Published shared support

`lelloman-simple-server =0.1.6` is published from clean tested commit
`58e9b7504176c82b538dca0cd3041468fb4a1ccd`, following the user's go-ahead to finish
the prepared downloader. Public archive SHA256 is
`4f37d395af4d6ca8495ad190b8cecc5ab078ddfc92b8e9b3a41bc9eee80d3fce`.
Cargo confirms publication/availability. An independent public download matches
checksum, embedded source commit and every library source file. The complete
shared check passes **706 test executions**, strict Clippy, isolated features,
SQLite fixture, formatting and warnings-as-errors rustdoc; package dry run and
publication verification pass for 135 files. See [Unix HTTP](unix-http.md) and
[publication evidence](publishing.md#verified-cratesio-release-016-4-october-2026).

### Pezzottify-downloader

- Original master `22ffc5c3` → integrated master
  `06943ad42a2acea03660fa69d45e076c5577dcc7`; the earlier canary commit is retained
  in history, followed by the final public dependency switch. Manifest/lockfile
  use public 0.1.6 with the verified checksum, without a path override.
- Production child serving calls shared `web::unix::serve`; Puppeteer HTTP
  proxy/health calls shared streaming `UnixClient`. Socket ownership/0700,
  stale-file cleanup, signals, 30-second grace/final removal, three-second health
  deadline, Host filtering, path/query/header forwarding, 502 failures and lazy
  streaming/error behavior are preserved. WebSocket upstream connection remains
  an outbound protocol client.
- UUID selection calls shared `HeaderRequestId::uuid_v4()`; the application ID
  extension wrapper preserves raw/repeated caller headers and extension overrides
  independently of shared selected-ID scope/response propagation. Librespot's
  HTTP primitives use the owned facade with its upstream Spotify client.
- **164 tests** and locked all-target check pass against the published package,
  including real process lifecycle, TCP-to-Unix WebSockets, streaming-before-EOF,
  upstream body failure and legacy request-ID oracle comparisons. Capped Clippy
  passes. Strict Clippy reproduces the same 12 findings on unchanged original
  master; diagnostic sets match. Original broad formatting debt remains.
- No direct normal Tower HTTP/Tower/Hyper/Hyper-util/Hyperlocal/HTTP-body-util
  dependencies remain. Development-only legacy oracles/mock servers are retained.
  Python SQLite remains outside Rust database applicability. Live Spotify login
  or a fresh Docker release build was not exercised. Remaining Rust server
  backend exposure: **None**.

### SCT

- Original master `d6b81bb` → integrated master
  `9e118e6c73db6f80b15f85fc525a1d254096a05b`. Workspace/core consume public 0.1.6;
  only the server adds the static-files feature. Direct Tower HTTP dependency
  entries are removed.
- Production fallback calls shared `StaticDir`. Directory indexes, GET/HEAD,
  MIME/body, ranges, modification conditionals, redirects/query, absent-file
  404 and method 405 remain unchanged. No implicit SPA fallback, compression or
  caching policy was added. Reserved API paths still return structured unsupported
  errors, not static content; auth/correlation/health/storage policy is preserved.
- Baseline workspace: **58 tests, 92 infrastructure ignores**. New real-TCP
  production-router contract passes with old ServeDir and new StaticDir. Final:
  **59 tests, the same 92 ignores**; strict all-target/all-feature Clippy,
  workspace build, formatting and diff checks pass.
- Initial baseline linking with debug information disabled hit an undefined
  SQLite/Tokio monomorphization symbol. Successful baseline/final host checks use
  isolated targets, two jobs, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`
  and `--config 'profile.dev.package.sqlx-sqlite.codegen-units=1'`. No repository
  build policy or database behavior was changed. PostgreSQL/S3/destructive
  qualification and frontend suites were not rerun for this fallback migration.
  Remaining Rust server backend exposure: **None**.

### SimpleAI

- Original master `851ce0c` → integrated master
  `9d29333df8ad498aec1ab83d101bf47184fa6aca`. Already adopted owned production
  routing/extractors/responses/serving, WebSocket/socket/message types, SSE and
  existing shared middleware. No direct production Axum/Tower HTTP/Hyper server
  dependencies or compatibility imports were found. Rust runtime source is
  unchanged; the public package is upgraded from 0.1.0 to verified 0.1.6 and
  current documentation is corrected. No unused new features are installed.
- Baseline/final workspace suites both pass **482 tests with one existing ignored
  Wake-on-LAN doctest**. Locked build, capped all-target Clippy and diff checks
  pass. Strict Clippy reproduces the same four findings in unchanged common
  source on original master; existing broad formatting debt remains. Real
  HTTP/WebSocket/SSE and fake-engine process tests are in the suite. The unchanged
  ignored RTX config fixture required by compile-time tests was copied only into
  the owned worktree and removed with it; nothing was written to original config.
- Audio, classification, extraction, Chatterbox and XTTS providers use Python
  standard-library `ThreadingHTTPServer`. These are separate model processes
  spawned/managed by runner engines, not direct Rust Axum exposure. Replacing
  their transport would require a distinct cross-language bridge/provider redesign.
  Their model/protocol code is unchanged. Outbound Reqwest/tokio-tungstenite
  clients are likewise outside owned server API migration. Live GPU/provider
  deployment qualification was not run. Remaining Rust server backend exposure:
  **None**; the separate Python scope boundary is recorded here, not as unfinished
  migration in the remaining cell.

### Integration and cleanup

All three masters contain their reviewed commits and match the tested trees;
original checkouts are clean. Owned consumer worktrees/migration branches and
shared release worktree/branch are removed. Both trackers now show only actual
remaining work in their last cells. Task-owned `/tmp` build directories, logs,
public verification archive, isolated fixtures/sockets and the central tracker
worktree are removed after integration. No unrelated artifacts are deleted.

## Torrentino remaining modules complete — 2026-10-04

Active `master` moved from `9850c1659ab060caf575f70d02580236b9ff0815` to
`11593ea11a25939ea28805f72fe4d3d0454a377a`. The isolated migration uses public
`lelloman-simple-server =0.1.6`, source `58e9b7504176c82b538dca0cd3041468fb4a1ccd`,
checksum `4f37d395af4d6ca8495ad190b8cecc5ab078ddfc92b8e9b3a41bc9eee80d3fce`.
No new library publication was required.

- 06a tracks accepted intake, identification and acquisition cycles. Lifecycle
  still owns worker futures; cancellation and panic release work guards.
- 06b uses shared schedule descriptions for immediate 250 ms intake, retaining
  Tokio Skip overdue-tick behavior, and shared polling for immediate serial
  identification/acquisition with a one-second delay after each completion.
- 06c uses ExecutionBudget for existing identification/acquisition deadlines and
  shared retry classification/caps. Durable attempt charging, retry checkpoints,
  transactions and cancellation authority remain application-owned.
- Step 10 evaluates existing durable intake, identification, acquisition and
  reply counters plus the 32-snapshot owner quota through shared limit checks.
  It adds no HTTP throttling or counter reset behavior.
- 07a configures and verifies WAL, foreign keys and the five-second busy timeout
  on every SQLx connection. 07b preserves the numeric user_version guard,
  accepting historical nonpositive markers and refusing future versions.
  Version-only preflight verifies no historical digests or schema health.
- 07e replaces production schema.sql bootstrap with structured creation of eleven
  tables and three explicit indexes. The old SQL remains a test oracle; PRAGMA
  metadata and actual defaults, constraints and cascades match. Existing tables
  are not rebuilt. Bootstrap and marker updates retain their transaction.
- 03b shares UUID generation for existing error-envelope IDs only. No new request
  header or task-local propagation is installed. HTTP tracing, CORS and health
  remain N/A after assessment. No direct Axum/backend dependency remains in the
  replacement Rust service. Legacy root CLI/external SQLx client code remains
  outside this service scope; quentin-torrentino is excluded and untouched.

Baseline: 44 Rust tests passed. Final: **56 passed, zero failed or ignored**;
locked build, formatting, diff checks and strict all-target Clippy passed.
Real Chromium E2E passed integrations, sessions, previews, clarification,
confirmation, format consent, delivery, movie identification and logout,
including service restart/replay. Added tests cover schema equivalence,
connection replacement, historical/future markers, rollback, quota boundaries,
paused-clock worker cadence/deadlines and cancellation/panic guard cleanup.

Master was rebased onto the committed migration; ancestry and identical tested
and integrated trees were verified. Owned worktrees, branches and temporary
build artifacts were removed. Both trackers record all applicable active steps
complete; schema totals including the excluded row are **12 Done, 5 N/A,
1 Pending (Quentin)**. Nothing was pushed or deployed.

## Step 07c/07d shared implementation — 2026-10-04

Implemented in an isolated branch/worktree from clean active master `e69edc1`.
Local package version is **0.1.7, unpublished**. Neither new feature adds a
native SQLite binding to normal dependencies or requires HTTP. No consumer was
changed; all 18 entries in each new column remain Pending (17 assessment/canary,
one excluded Quentin Torrentino). Existing Done/N/A statuses are preserved.

07c `database-sqlite-backup` exposes preparation-only checkpoint registry reports
and guarded per-file backup coordination through synchronous and Send async
adapter traits. Checkpoint/raw-copy and engine-consistent-copy strategies remain
distinct. Reports retain phase completion, busy/page counts, native errors,
verification results, publication state and failed cleanup/retained staging.
No-replace publication is an explicit adapter contract, and publication success
is not undone or hidden by cleanup failure. Canonical identity, fencing, filesystem
permissions, native copy/verification and retention remain application-owned.
Abruptly abandoning an async coordinator may leave staging; own and await it
through shutdown rather than assuming cancellation rolled back native work.
No restore or cross-file atomic snapshot is claimed. See the
[backup contract](step-07c-sqlite-backup.md).

07d `database-blocking` supplies a fixed dedicated-thread executor with explicit
weighted priorities, per-priority queue bounds, application-defined lane limits,
queue/runtime deadlines and shared sync/async result observation. Queue timeout
and pre-dispatch cancellation prevent execution; caller runtime timeout retains
real worker/lane occupancy until native work finishes. Panic isolation includes
abandoned result/cancelled capture destructors. Close/drain has separate modes
for draining accepted jobs or cancelling queued jobs. No operation is retried or
interrupted implicitly. See the [executor contract](step-07d-database-blocking.md).

Verification: pre-edit database/schema baseline **51 passed**. Final all-feature
suite **422 tests/doctests passed, zero failed or ignored**, including **11 new
backup tests and 15 new executor tests**. Strict all-target/all-feature Clippy,
formatting, diff checks, rustdoc, standalone feature builds and the synchronous
executor example pass. Real SQLite fixtures verify WAL reader/checkpoint busy
results, consistent copying with committed WAL data, phase failures including
partial-copy cleanup, integrity/schema rejection, destination collision/source
protection and retained staging after failed cleanup. Controlled execution tests
verify fairness, saturated-lane bypass, admission bounds, cancellation, panic
recovery, runtime deadlines, shutdown and reentrant destructors. A real timed-out
SQLite transaction remains active then commits once; no rollback/retry is inferred.

The roadmap and both matrices now distinguish implemented shared APIs from
pending consumer adoption. Next are backup canaries against Pezzottify's registry
and Crumbles' staged-copy path, then a Pezzottify executor canary after exact
production-policy review. Master was rebased onto the committed implementation,
ancestry and tested/integrated trees verified, and the owned worktree/branch and
temporary logs removed. Nothing was published, pushed or deployed.

## Step 07c/07d publication — 2026-10-04

Explicitly authorized by the user after the library implementation. Published
`lelloman-simple-server 0.1.7` from clean committed source `d59f067532ca7858adcc07e78ab272ff27ba055e`;
archive SHA256 `81ad9eef5bc3388727f06c97be1fa029c0baf39a6ba8efc021d2e9a033db74d0`.
The preceding unpublished checkpoint is superseded for release availability,
not consumer adoption: both new columns remain Pending assessment/canary;
Quentin Torrentino remains excluded.

Full release `scripts/check` passes **758 test executions**, strict all-feature
Clippy, isolated feature suites, SQLite fixture, formatting and rustdoc with
warnings denied. Fixed the fixture's stale 0.1.6 lock entry before release and
added new feature-only suites to the check script. Inspected 142 package files;
clean publication dry run and upload pass. Cargo confirms registry availability.
Independent archive verification matches all 68 source files and the embedded
Git commit. A fresh Cargo home, without registry credentials or path overrides,
downloads, compiles and runs checkpoint preparation and bounded execution. Its
lockfile matches crates.io source, version and public checksum.

Release preparation and evidence are committed in an isolated worktree; master
was rebased onto that branch, ancestry/tested-tree integration verified, and the
owned worktree, branch and temporary consumer/logs removed. Consumer manifests
were not changed. Nothing was pushed or deployed. See the
[release record](publishing.md#verified-cratesio-release-017-4-october-2026).
