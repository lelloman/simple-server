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

Last verified against local checkouts: 2026-09-19.

| Project | Server components | 1. Axum centralization | 2. Lifecycle / main() |
| --- | --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** | Pending approval |
| favzetto | `backend` | **Done** | **Done (local; scoped)** |
| androidoscopy | `server` | **Done** | Pending |
| crumbles | `crumbles`, `crumbles-integration` | **Done (migration branch)** | Pending |
| fausto | `server`; associated plugin API and plugins | **Done** | Pending |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | **Done** | **Done (local; scoped)** |
| lellostore | `backend` | **Done** | **Done (local)** |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | **Done** | Pending |
| observo | `observo-server` | **Done** | Pending |
| paranza | `apps/paranza-server` | **Done** | Pending |
| peerlo | `peerlo-api` | **Done** | Pending |
| pezzottflix | `pezzottflix-server` | **Done** | Pending |
| pezzottify-downloader | Puppeteer API and downloader HTTP server | **Done** | Pending |
| quentin-torrentino | `crates/server` | **Done** | Pending |
| sct | `sct-server` | **Done (migration branch)** | Pending |
| simple-agents | `simple-agents-service`; associated coding test servers | **Done** | Pending |
| simple-ai | `backend`, `inference-runner` | **Done** | Pending |

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
it is **not merged** into the main checkout, where dispatcher development
continues. Newly added Axum imports must be migrated when integrating that work.

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

sct completed Step 01 in commit `d577999`. Completed on isolated simple-server-step01 branch from 5cffda9; NOT MERGED into ongoing S3 work. Full scripts/check passes: 33 Rust/Postgres tests plus one doctest, strict Clippy/formatting, 49 contract tests, independent client packaging, frontend build and three real-server browser E2E scenarios. Four future milestone scenarios remain explicitly skipped. See docs/simple-server-migration.md in sct-step01.

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

Pezzottify has not been changed. Its integration requires explicit approval.

## Planned steps

Further capabilities
include HTTP support, observability, health, background tasks, database helpers,
authentication, authorization, and rate limiting. The HTML matrix already shows
these columns as planned; their exact grouping and order remain adjustable.
Hiding the transitional Axum API is incremental work across those steps.

The initial scope is the 17 Axum-based products inventoried in this workspace.
Custom servers in `rns-rs` and `lxmf-rs`, and embedded servers in `librespot` and
`wgtransport`, are not currently scheduled for this migration. They can be added
if adoption of an independent `simple-server` module becomes useful.
