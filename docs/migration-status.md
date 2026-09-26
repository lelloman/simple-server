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

**Combined auth (08/09):** complete locally — 16 Done, 1 N/A, 0 Pending.

**Rate limiting (10):** complete locally — 12 Done, 0 Partial, 5 N/A, 0 Pending.
All 17 services have been assessed. The five final consumers now use shared
policies for their remaining quotas and pacing; application storage and transaction
ownership are preserved. Test limitations are recorded in the completion evidence.

**Routing / HTTP core (11):** **8 Done, 0 Partial, 9 Pending**. Pezzottify, Androidoscopy, Crumbles, Fausto, lello-auth, lellostore, Favzetto and Observo now
use shared APIs for all production route groups, ordinary handlers, built-in
extractors, response adapters and router assembly. Done excludes explicitly
tracked protocol compatibility boundaries. Pending records unverified adoption,
not a claim that the shared API already supports every service's needs.
See the [contract](web-core.md) and [completion evidence](#pezzottify-routing-completion--2026-09-25).

**Latest integration (2026-09-26):** Observo `master` at `7217da1`, using shared
`5ba4658`, completes routing, forms/HTML/redirects, peer-aware serving and plugin
proxy adoption. No direct Axum API or compatibility adapter remains in its Rust
sources/manifests. All **97 Rust tests** and both real-binary HTTP suites pass
before and after. Shared: **255 tests/doctests**, strict Clippy and minimal web
build pass. Consumer Clippy retains an existing denied numeric-constant lint;
its capped-lint run completes. See the
[Observo verification record](#observo-routing-completion--2026-09-26).

**Next execution order:** complete and verify Axum removal from every consumer →
revisit Step 07 database helpers. Step numbers are retained; Step 07 is deferred,
not required for the Axum abstraction. Database setup and migrations remain local
to services. The Axum-removal milestone includes production code, tests and removal
of the transitional re-export; Axum can remain internal to simple-server.
See the [roadmap and completion criteria](design.md).

The final observations column records the [Axum exposure audit](#axum-exposure-audit--2026-09-24).
It is independent of module adoption status.

Step 03a rollout verified: 2026-09-20. Earlier adoption evidence retains its original dates.

| Project | Server components | 1. Axum centralization | 2. Lifecycle / main() | 03a. Logging | 03b. Correlation | 03c. HTTP tracing | 04a. Body limits | 04b. Response headers | 04c. CORS | 05. Health/readiness | 06a. Task ownership | 06b. Scheduling | 06c. Execution policies | 08/09. Auth | 10. Rate limiting | 11. Routing / HTTP core | Remaining Axum exposure |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local)** | **Done (local canary)** | **Done (local; scoped canary)** | N/A (assessed) | N/A (no served probe) | **Done (local; scoped canary)** | **Done (local; primitives)** | **Done (local; primitives)** | **Done (local; sessions + route permissions)** | **Done (HTTP, MCP, durable quotas + outbound pacing)** | **Done (all production route groups)** | Multipart fields/errors; SSE search producers; MCP and sync WebSocket protocol types; backend tracing observer; independent HTTP mocks and error-renderer differential test. |
| favzetto | `backend` | **Done** | **Done (local; scoped)** | **Done (local pilot)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | **Done (local canary)** | **Done (local; request work)** | **Done (local; bounded batches)** | **Done (local; retry scope)** | **Done (local canary)** | **Done (local; global + endpoint budgets)** | **Done (local)** | WebSocket socket/message types and upgrade compatibility remain. Production routing, extractors, responses, middleware, serving, multipart readers/fields/errors and HTTP test fixtures use shared APIs. |
| androidoscopy | `server`; Android SDK pairing | **Done** | **Done (local; scoped)** | **Done (local; legacy logger)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (no served probe) | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; controller + LAN access)** | **Done (local; device JNI pairing gate)** | **Done (all production route groups)** | Controller and legacy WebSocket protocol types/upgrade compatibility; axum-server TLS configuration, serving and shutdown handle, including TLS test fixture. Production routing, dashboard responses and ordinary HTTP tests now use shared APIs. |
| crumbles | `crumbles`, `crumbles-integration` | **Done** | **Done (scoped)** | **Done (local canary)** | **Done (local pilot; main HTTP server)** | **Done (local canary; main HTTP server)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped canary)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; durable primitives)** | **Done (local; retry primitives)** | **Done (local; main + integration)** | **Done (local; HTTP + MCP + durable dispatcher)** | **Done (both servers)** | Multipart fields/errors; realtime WebSocket socket/message types; explicit tracing compatibility adapter. Both servers, MCP/health service mounting, auth/CSRF, metrics, static responses and HTTP fixtures now use shared APIs. |
| fausto | `server`; associated plugin API and plugins | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | **Done (local)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; dynamic cron)** | N/A (assessed) | **Done (local; HTTP + WebSocket + admin/write)** | **Done (local; five API tiers)** | **Done (local)** | Owned multipart fields/errors; WebSocket socket/message types; tracing compatibility; axum-test transport and parser oracles. Server, plugin API v2, both plugins, Swagger, validation and streamed responses use shared routing APIs. Dynamic plugins require rebuilding. |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (Rust; Caddy owns CORS) | **Done (local; scoped)** | **Done (local; webhook scope)** | **Done (local; capacity)** | **Done (local; retry scope)** | **Done (local; sessions + resource/admin access)** | **Done (endpoint budgets + persisted device polling)** | **Done (local)** | No direct Axum interfaces remain in production Rust, examples or tests. Server, embedding API and all three HTTP examples use shared routing, forms, HTML/redirects, cookie extraction and peer-aware serving. The lello-auth-axum crate name is retained; Axum is internal to simple-server. |
| lellostore | `backend` | **Done** | **Done (local)** | **Done (local)** | N/A (assessed) | **Done (local)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; OIDC + admin)** | N/A (no inbound admission policy) | **Done (local)** | WebSocket socket/message types; tracing compatibility adapter; axum-test transport and multipart/WebSocket helpers. Routing, auth extraction, streamed downloads, static files and OIDC mocks use shared APIs. Multipart readers, fields and errors are now fully shared types. |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; pipeline/gateway)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | **Done (local; weighted claims)** | **Done (local; budgets/retry)** | **Done (local; all three services)** | **Done (local; gateway budgets)** | Pending | API, gateway and pipeline control routers; proxy headers/client identity; byte responses and static fallbacks; custom middleware and mock upstreams. |
| observo | `observo-server`; standalone extractor logging | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | N/A (no served probe) | **Done (local; scoped)** | **Done (local; primitives)** | N/A (assessed) | **Done (local; IP/key/JWT access)** | N/A (no request quota) | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Routing, forms/HTML/redirects, auth middleware, peer-aware serving, plugin request/body proxying and HTTP tests use shared APIs; Axum remains internal to simple-server. |
| paranza | `apps/paranza-server` | **Done** | **Done (local; scoped)** | N/A (no logger) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; runner + PCM access)** | N/A (no implemented request quota) | Pending | Private router; state/path/query/raw-body extraction; response conversion and test helpers. Smallest core HTTP pilot; no observed WebSocket, SSE or multipart use. |
| peerlo | `peerlo-api` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | N/A (assessed) | N/A (assessed) | **Done (local; retry primitives)** | **Done (local; bearer + Torznab keys)** | **Done (local; API + crawler + durable DHT)** | Pending | Public REST routers; auth, rate-limit and metrics middleware; custom error responses and tracing tests. Suitable second pilot for middleware composition. |
| pezzottflix | `pezzottflix-server` | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local)** | **Done (local; main HTTP)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; socket scope)** | **Done (local; durable queue cadence)** | N/A (assessed) | **Done (local; sessions + permissions + WebSocket)** | **Done (login, durable daily quota + TMDB pacing)** | Pending | Stateful subrouters; auth/language/pagination extractors; video streaming; WebSocket sync; client-address extraction; static/compression integration and HTTP tests. |
| pezzottify-downloader | Puppeteer API, downloader HTTP server and Python cron | **Done** | **Done (local; scoped)** | **Done (local)** | **Done (local; Puppeteer)** | **Done (local; both HTTP routers)** | N/A (assessed) | N/A (assessed) | **Done (local; both routers)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; priority admission)** | N/A (assessed) | N/A (no application caller gate) | **Done (local; Python/SQLite quota bridge)** | Pending | Parent and child HTTP routers; Unix-socket serving; streamed audio/proxy responses; WebSocket proxying; correlation/tracing and admission middleware; proxy tests. |
| quentin-torrentino | `crates/server` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; stage capacity)** | N/A (assessed) | **Done (local; API access)** | **Done (local; MusicBrainz pacing)** | Pending | Public router composition; multipart torrent uploads; SSE chat; dashboard WebSockets; static-file services; custom middleware and HTTP test fixtures. |
| sct | `sct-server` | **Done** | **Done (scoped)** | N/A (no logger) | **Done (local; storage-backed HTTP)** | **Done (local; storage-backed HTTP)** | **Done (local; storage-backed HTTP)** | **Done (local; scoped)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; worker scope)** | **Done (local; durable worker cadence)** | **Done (local; retry scope)** | **Done (local; tokens/sessions + transactional access)** | N/A (durable state caps, no request quota) | Pending | Custom JSON/query rejection mapping; auth extractors and matched-route metadata; archive/checkpoint/transfer streams; range/HEAD behavior; HTTP tests and example. |
| simple-agents | `simple-agents-service`; runner-shell logging; associated coding test servers | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | N/A (assessed) | **Done (local pilot; service routes)** | **Done (local; scoped canary)** | N/A (assessed) | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; transactional capacity)** | **Done (local; retry scope)** | **Done (local; caller + transactional access)** | N/A (task/storage admission, no request quota) | Pending | Public modular router; per-route response mapping; session SSE; broker WebSockets; streamed releases; service tests and coding-crate GitHub mock servers. |
| simple-ai | `backend`, `inference-runner` | **Done** | **Done (local; scoped)** | **Done (local)** | N/A (assessed) | **Done (local; backend)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; scoped canary)** | **Done (local; scoped)** | **Done (local; scoped)** | **Done (local; batch readiness)** | N/A (assessed) | **Done (local; backend user/admin)** | **Done (local; backend budgets)** | Pending | Backend and inference-runner routers; streamed chat/speech; multipart OCR/audio; custom errors; SSE; admin/gateway WebSockets; peer-aware tests. |

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
| observo | master `ef3bff39` | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Routing, forms/HTML/redirects, auth middleware, peer-aware serving, plugin request/body proxying and HTTP tests use shared APIs; Axum remains internal to simple-server. |
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
| pezzottify | `dev` / `a25b0c3a` | `pezzottify-server/src/server/route_builder.rs` installs authentication, CSRF, rate limits, tracing and cache policy, but no CORS layer or allow-origin response policy. |
| favzetto | **Done (local)** | WebSocket socket/message types and upgrade compatibility remain. Production routing, extractors, responses, middleware, serving, multipart readers/fields/errors and HTTP test fixtures use shared APIs. |
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
| observo | Protected-route tree only: wildcard origins/methods/request/exposed headers, no credentials; public routes stay outside CORS, auth/body-limit stay inside. | Baseline route tests: 47 passed. Final server suite **92 passed**; new policy test distinguishes preflight from ordinary exposed headers. All-target Clippy completes with warnings capped. | **Done (local)** | No remaining direct Axum API usage in Rust sources/manifests. Routing, forms/HTML/redirects, auth middleware, peer-aware serving, plugin request/body proxying and HTTP tests use shared APIs; Axum remains internal to simple-server. |
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
