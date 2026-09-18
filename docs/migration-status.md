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

Last verified against local checkouts: 2026-09-18.

| Project | Server components | 1. Axum centralization |
| --- | --- | --- |
| pezzottify | `pezzottify-server` | **Done** |
| favzetto | `backend` | **Done** |
| androidoscopy | `server` | Pending |
| crumbles | `crumbles`, `crumbles-integration` | Pending |
| fausto | `server`; associated plugin API and plugins | Pending |
| lello-auth | `lello-auth-server`, `lello-auth-axum`; associated examples | Pending |
| lellostore | `backend` | Pending |
| meteonesto | `weather-api`, `weather-gateway`, `weather-pipeline` control API | Pending |
| observo | `observo-server` | Pending |
| paranza | `apps/paranza-server` | Pending |
| peerlo | `peerlo-api` | Pending |
| pezzottflix | `pezzottflix-server` | Pending |
| pezzottify-downloader | Puppeteer API and downloader HTTP server | Pending |
| quentin-torrentino | `crates/server` | Pending |
| sct | `sct-server` | Pending |
| simple-agents | `simple-agents-service`; associated coding test servers | Pending |
| simple-ai | `backend`, `inference-runner` | Pending |

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

## Planned steps

The next planned step is lifecycle and entry-point setup. Further capabilities
include HTTP support, observability, health, background tasks, database helpers,
authentication, authorization, and rate limiting. The HTML matrix already shows
these columns as planned; their exact grouping and order remain adjustable.
Hiding the transitional Axum API is incremental work across those steps.

The initial scope is the 17 Axum-based products inventoried in this workspace.
Custom servers in `rns-rs` and `lxmf-rs`, and embedded servers in `librespot` and
`wgtransport`, are not currently scheduled for this migration. They can be added
if adoption of an independent `simple-server` module becomes useful.
