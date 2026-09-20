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

Step 03a pilot verified: 2026-09-20. Earlier adoption evidence retains its original dates.

| Project | Server components | 1. Axum centralization | 2. Lifecycle / main() | 03a. Logging |
| --- | --- | --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** | **Done (local; scoped)** | Pending |
| favzetto | `backend` | **Done** | **Done (local; scoped)** | **Done (local pilot)** |
| androidoscopy | `server` | **Done** | **Done (local; scoped)** | Pending |
| crumbles | `crumbles`, `crumbles-integration` | **Done** | **Done (scoped)** | Pending |
| fausto | `server`; associated plugin API and plugins | **Done** | **Done (local; scoped)** | Pending |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | **Done** | **Done (local; scoped)** | Pending |
| lellostore | `backend` | **Done** | **Done (local)** | Pending |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | **Done** | **Done (local; scoped)** | Pending |
| observo | `observo-server` | **Done** | **Done (local; scoped)** | Pending |
| paranza | `apps/paranza-server` | **Done** | **Done (local; scoped)** | Pending |
| peerlo | `peerlo-api` | **Done** | **Done (local; scoped)** | Pending |
| pezzottflix | `pezzottflix-server` | **Done** | **Done (local; scoped)** | Pending |
| pezzottify-downloader | Puppeteer API and downloader HTTP server | **Done** | **Done (local; scoped)** | Pending |
| quentin-torrentino | `crates/server` | **Done** | **Done (local; scoped)** | Pending |
| sct | `sct-server` | **Done** | **Done (scoped)** | Pending |
| simple-agents | `simple-agents-service`; associated coding test servers | **Done** | **Done (local; scoped)** | Pending |
| simple-ai | `backend`, `inference-runner` | **Done** | **Done (local; scoped)** | Pending |

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
deployment was performed. Modules 03b and 03c remain planned.

## Planned steps

[Step 03: observability](step-03-observability.md) contains three independently
adoptable modules: **03a logging setup**, **03b request correlation**, and
**03c HTTP tracing**. 03a is implemented and locally adopted by Favzetto; 03b and 03c remain planned.
The [03a contract](step-03a-logging.md) records the API and pilot compatibility. Completion of one module does not imply completion of Step 03.

Further capabilities include HTTP support, health, background tasks, database helpers,
authentication, authorization, and rate limiting. The HTML matrix already shows
these columns as planned; their exact grouping and order remain adjustable.
Hiding the transitional Axum API is incremental work across those steps.

The initial scope is the 17 Axum-based products inventoried in this workspace.
Custom servers in `rns-rs` and `lxmf-rs`, and embedded servers in `librespot` and
`wgtransport`, are not currently scheduled for this migration. They can be added
if adoption of an independent `simple-server` module becomes useful.
