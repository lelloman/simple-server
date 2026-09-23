# Service migration status

Open the [HTML migration matrix](migration-status.html) for the visual table,
with all services as rows and implemented and planned steps as columns.

This table tracks adoption of implemented `simple-server` capabilities. Each row
is a product; the component column identifies its Rust servers. Add a new step
column when the corresponding shared capability is implemented, then update
each product independently as it adopts that capability.

**Status:** Done = migrated and verified; Pending = not migrated;
Partial = only some listed components migrated; N/A = deliberately not needed.
Optional modules do not have to be adopted by every product.

Step 03a rollout verified: 2026-09-20. Earlier adoption evidence retains its original dates.

| Project | Server components | 1. Axum centralization | 2. Lifecycle / main() | 03a. Logging | 03b. Correlation | 03c. HTTP tracing | 04a. Body limits | 04b. Response headers | 04c. CORS | 05. Health/readiness | 06a. Task ownership | 06b. Scheduling | 06c. Execution policies |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local)** | **Done (local canary)** | **Done (local; scoped canary)** | N/A (assessed) | N/A (no served probe) | **Done (local; scoped canary)** | **Done (local; primitives)** | **Done (local; primitives)** |
| favzetto | `backend` | **Done** | **Done (local; scoped)** | **Done (local pilot)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | **Done (local canary)** | **Done (local; request work)** | Pending (durable workflow) | **Done (local; retry scope)** |
| androidoscopy | `server` | **Done** | **Done (local; scoped)** | **Done (local; legacy logger)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (no served probe) | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) |
| crumbles | `crumbles`, `crumbles-integration` | **Done** | **Done (scoped)** | **Done (local canary)** | **Done (local pilot; main HTTP server)** | **Done (local canary; main HTTP server)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped canary)** | **Done (local; scoped)** | **Done (local; scoped)** | Pending (durable scheduler) | Pending (durable policies) |
| fausto | `server`; associated plugin API and plugins | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | **Done (local)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; dynamic cron)** | N/A (assessed) |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (Rust; Caddy owns CORS) | **Done (local; scoped)** | **Done (local; webhook scope)** | **Done (local; capacity)** | **Done (local; retry scope)** |
| lellostore | `backend` | **Done** | **Done (local)** | **Done (local)** | N/A (assessed) | **Done (local)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; pipeline/gateway)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | Pending (durable scheduler) | **Partial (runtime budgets)** |
| observo | `observo-server`; standalone extractor logging | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | N/A (no served probe) | **Done (local; scoped)** | **Done (local; primitives)** | N/A (assessed) |
| paranza | `apps/paranza-server` | **Done** | **Done (local; scoped)** | N/A (no logger) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | N/A (assessed) |
| peerlo | `peerlo-api` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; retry primitives)** |
| pezzottflix | `pezzottflix-server` | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | **Done (local; main HTTP)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; socket scope)** | Pending (durable queue) | N/A (assessed) |
| pezzottify-downloader | Puppeteer API and downloader HTTP server | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; Puppeteer)** | **Done (local; both HTTP routers)** | N/A (assessed) | N/A (assessed) | **Done (local; both routers)** | **Done (local; scoped)** | **Done (local; scoped)** | Pending (priority scheduler) | N/A (assessed) |
| quentin-torrentino | `crates/server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; stage capacity)** | N/A (assessed) |
| sct | `sct-server` | **Done** | **Done (scoped)** | N/A (no logger) | **Done (local; storage-backed HTTP)** | **Done (local; storage-backed HTTP)** | **Done (local; storage-backed HTTP)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; worker scope)** | Pending (durable scheduling) | **Done (local; retry scope)** |
| simple-agents | `simple-agents-service`; runner-shell logging; associated coding test servers | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local pilot; service routes)** | **Done (local; scoped canary)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | Pending (fleet reservations) | **Done (local; retry scope)** |
| simple-ai | `backend`, `inference-runner` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local; backend)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped canary)** | **Done (local; scoped)** | **Done (local; scoped)** | Pending (model-aware batching gap) | N/A (assessed) |

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

[Step 03: observability](step-03-observability.md) contains three independently
adoptable modules: **03a logging setup**, **03b request correlation**, and
**03c HTTP tracing**. 03a is implemented; local adoption is recorded per service
above. 03b rollout is complete with six adopted products and eleven N/A;
03c is locally adopted by nine products, with eight N/A after assessment.
The [03a contract](step-03a-logging.md) records the API and pilot compatibility. Completion of one module does not imply completion of Step 03.

The accepted [Step 04 source assessment](step-04-http-policy-assessment.md) splits
HTTP policies into optional 04a body limits, 04b response headers and 04c CORS.
04a is implemented; the pilot/canary verification is recorded below.
[04b response headers](step-04b-response-headers.md) is rolled out locally: ten adopted, seven N/A; see the verification record below.
[04c CORS](step-04c-cors.md) and [05 health](step-05-health.md) are implemented;
their completed local rollout evidence is recorded below.
[06 background tasks](step-06-background-tasks.md) now has three implemented
library modules; consumer rollout remains pending.

Remaining shared capabilities are database helpers, authentication, authorization,
and rate limiting. The HTML matrix shows those columns as planned.
The end goal is to **completely abstract Axum away** from consumer code and the
public API, then remove the transitional Axum re-export and temporary escape
hatches. Completing the module columns alone does not establish that outcome;
a final consumer/API audit must verify the
[design completion criteria](design.md#end-goal-completely-abstract-axum-away).
This final abstraction milestone remains planned.

The initial scope is the 17 Axum-based products inventoried in this workspace.
Custom servers in `rns-rs` and `lxmf-rs`, and embedded servers in `librespot` and
`wgtransport`, are not currently scheduled for this migration. They can be added
if adoption of an independent `simple-server` module becomes useful.

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
| paranza | master `2bd528f7` | apps/paranza-server/src/main.rs::api_router defines management routes and state without a body-limit layer. |
| peerlo | master `907c9ad2` | peerlo-api create_router/start_server installs metrics, tracing, auth, rate limiting and optional CORS without an extractor limit. |
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
| observo | master `ef3bff39` | observo-server routes/main and routes/plugins.rs: dynamic plugin response forwarding and MIME construction, with no local cache/security-header policy. Upstream header forwarding is not header-policy adoption. |N/A (no served probe) |
| paranza | master `2bd528f7` | apps/paranza-server/src/main.rs: API and PCM protocol/header conversion, no response cache/security-header policy. |
| peerlo | master `907c9ad2` | crates/peerlo-api/src/lib.rs and routes.rs: auth, rate limiting, tracing and CORS, without response cache/security-header policy. |
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
| pezzottify | `dev` / `a25b0c3a` | `pezzottify-server/src/server/route_builder.rs` installs authentication, CSRF, rate limits, tracing and cache policy, but no CORS layer or allow-origin response policy. |N/A (no served probe) |
| favzetto | `master` / `47c8fe67` | Backend production router has body/header/auth policy but no CORS middleware or Access-Control-Allow headers. |**Done (local canary)** |
| androidoscopy | `master` / `f4461a81` | `server/src/main.rs` HTTP/WS router setup and legacy server have no CORS response policy. |
| lello-auth | `master` / `b1827fdd` | CORS managed by Caddy; not migrated into Rust. `homelab/caddy/Caddyfile` permits selected application origins with credentials and OPTIONS 204. Server, integration crate and examples install no Rust CORS layer. Moving ownership needs coordinated proxy/application changes; live deployment not probed. |
| meteonesto | `master` / `6d0eb458` | Weather API, gateway and pipeline control routers implement their own HTTP policies, with no application CORS layer. Infrastructure/auth-edge documentation is not application adoption. |
| paranza | `master` / `2bd528f7` | `apps/paranza-server/src/main.rs` router and configuration contain no CORS policy. |
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
| observo | Protected-route tree only: wildcard origins/methods/request/exposed headers, no credentials; public routes stay outside CORS, auth/body-limit stay inside. | Baseline route tests: 47 passed. Final server suite **92 passed**; new policy test distinguishes preflight from ordinary exposed headers. All-target Clippy completes with warnings capped. | `86d28f401fc4fa5236b30dc26c310e8ce3262684` |N/A (no served probe) |
| peerlo | Conditional production server plus both CORS router helpers: wildcard origins/methods/request headers, no exposed headers/credentials; original auth/rate-limit order retained. | Three baseline CORS tests pass with socket permission (initial sandbox run denied two local binds). Final peerlo-api suite **167 passed**, including real loopback startup and combined rate-limit paths. All-target Clippy completes with warnings capped. | `067c2a71354d73ee803235869c49afc296c75557` |
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
| meteonesto | `master` / `6d0eb45` | `6d032afedc8ee948be47dd6aeeec916a2d9a382c` | API 2 health tests, gateway 1 health-boundary test, pipeline liveness 1 and readiness recovery/shutdown 1 passed. Component checks passed with Rust 1.97.1; pipeline lock updated offline. Metrics, OIDC/upstream timeout and correlation preserved. |
| sct | `master` / `b7d8230` | `b34840dbf7dbf1939426565acedd068916bf20b4` | Baseline HTTP test compiled but sandbox denied bind; permitted final HTTP test 1/1 and server check passed. Database timeout, writer readiness, 204/503 and text liveness preserved. Changed-file formatting passed. |
| simple-agents | `main` / `0cf88e2` | `3a106d9efac3852146ff10d74bda924830a7281c` | Focused real-router database readiness/liveness regression 1/1 before and after; formatting passed. Replayed concurrent UI commit; tested Cargo/crates tree unchanged by replay. |
| simple-ai | `master` / `30ed3ea` | `0523c03277930f2e8bc64ff715779d16e600541c` | Baseline runner health 2; final backend 1 and runner 4. Fake engines verify complete mixed/unhealthy reports, every-engine polling, empty/OCR behavior; GET/HEAD/POST verified. Existing ignored rtx.toml copied unchanged after missing-fixture compile failure; final rerun passed. |
| peerlo | `master` / `067c2a7` | `e51e4b7444f17073d509d04ca88f531d204a5f5e` | Baseline health 4; final 165 unit + 2 tracing tests. Always-200 degraded/healthy response, uptime, routing, auth, metrics, rate limits and CORS retained. Formatting/diff passed. |
| paranza | `master` / `2bd528f` | `1c559ee7bde3b510177f93dc979bfe02335d9842` | Baseline health test passed; final full server suite 54 passed, including new real-router success and poisoned-mutex 500 test. Single detailed snapshot and original error mapping retained. Incidental formatting excluded. |
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
Unsupported requirements remain Pending/Partial rather than being called complete
or N/A. Per-service evidence and compatibility gaps are recorded below.

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

**Supported scoped rollout complete locally; compatibility gaps remain.** All
17 services are assessed. The Pezzottify canary and 15 subsequent consumer changes
are committed on their development branches. Each development branch was rebased
onto its migration branch; ancestry and tested trees were verified, and all Step 06
consumer worktrees/branches removed. Paranza required no code change. Concurrent
LelloStore commits and unrelated Simple AI, LelloAuth and SCT work are preserved.
No pushes or deployments were performed.

| Module | Done (scoped) | Partial | Pending compatibility | N/A (assessed) |
| --- | ---: | ---: | ---: | ---: |
| 06a ownership | 14 | 0 | 0 | 3 |
| 06b scheduling/capacity | 5 | 0 | 8 | 4 |
| 06c policies | 6 | 1 | 1 | 9 |

Remaining scheduling work covers durable claims/reservations/queues (Pezzottflix/Favzetto/Crumbles/Meteonesto/SCT/Simple Agents), model-aware
batching (Simple AI), and downloader priority/prefetch rules. Crumbles' durable
execution authority and signed jitter remain Pending; Meteonesto adopts runtime
budgets but retains incompatible configurable retry multipliers, hence Partial.
Baseline test/lint failures and checks not rerun are explicit in each record.


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
