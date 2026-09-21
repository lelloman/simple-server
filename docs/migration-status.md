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

| Project | Server components | 1. Axum centralization | 2. Lifecycle / main() | 03a. Logging | 03b. Correlation | 03c. HTTP tracing |
| --- | --- | --- | --- | --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |
| favzetto | `backend` | **Done** | **Done (local; scoped)** | **Done (local pilot)** | N/A (assessed) | Pending |
| androidoscopy | `server` | **Done** | **Done (local; scoped)** | **Done (local; legacy logger)** | N/A (assessed) | Pending |
| crumbles | `crumbles`, `crumbles-integration` | **Done** | **Done (scoped)** | **Done (local canary)** | **Done (local pilot; main HTTP server)** | Pending |
| fausto | `server`; associated plugin API and plugins | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | Pending |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |
| lellostore | `backend` | **Done** | **Done (local)** | **Done (local)** | N/A (assessed) | Pending |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; pipeline/gateway)** | Pending |
| observo | `observo-server`; standalone extractor logging | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |
| paranza | `apps/paranza-server` | **Done** | **Done (local; scoped)** | N/A (no logger) | N/A (assessed) | Pending |
| peerlo | `peerlo-api` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |
| pezzottflix | `pezzottflix-server` | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | Pending |
| pezzottify-downloader | Puppeteer API and downloader HTTP server | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; Puppeteer)** | Pending |
| quentin-torrentino | `crates/server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |
| sct | `sct-server` | **Done** | **Done (scoped)** | N/A (no logger) | **Done (local; storage-backed HTTP)** | Pending |
| simple-agents | `simple-agents-service`; runner-shell logging; associated coding test servers | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |
| simple-ai | `backend`, `inference-runner` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | Pending |

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
is unchanged and 03c is implemented; its Crumbles canary is in progress.

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
verification scope and existing limitations are recorded above; 03c is implemented; its Crumbles canary is in progress.

## Planned steps

[Step 03: observability](step-03-observability.md) contains three independently
adoptable modules: **03a logging setup**, **03b request correlation**, and
**03c HTTP tracing**. 03a is implemented; local adoption is recorded per service
above. 03b rollout is complete with six adopted products and eleven N/A;
03c is implemented; its Crumbles canary is in progress.
The [03a contract](step-03a-logging.md) records the API and pilot compatibility. Completion of one module does not imply completion of Step 03.

Further capabilities include HTTP support, health, background tasks, database helpers,
authentication, authorization, and rate limiting. The HTML matrix already shows
these columns as planned; their exact grouping and order remain adjustable.
Hiding the transitional Axum API is incremental work across those steps.

The initial scope is the 17 Axum-based products inventoried in this workspace.
Custom servers in `rns-rs` and `lxmf-rs`, and embedded servers in `librespot` and
`wgtransport`, are not currently scheduled for this migration. They can be added
if adoption of an independent `simple-server` module becomes useful.

## Step 03c: HTTP tracing

The opt-in `http-tracing` module is implemented; see the [contract](step-03c-http-tracing.md). Consumer adoption remains Pending until verified and integrated. Crumbles is the first canary: its main server installs `TraceLayer::new_for_http()` and an application correlation span. Baseline server tests pass: 529 passed, two existing ignores.
