# Step 05: health and readiness

## Evidence and scope

Inventory on 2026-09-21 found these distinct production contracts:

| Service | Existing behavior | Source |
| --- | --- | --- |
| Favzetto | `/health`: static JSON with package version; `/ready`: database, storage, then PDF runtime checks, first AppError wins | `backend/src/http/mod.rs`, `db/mod.rs`, `storage/mod.rs`, `pdf/mod.rs` |
| Crumbles | `/api/health`: static status/service JSON; no dependency readiness in this route | `crumbles/src/server/routes/mod.rs` |
| Simple Agents | `/healthz`: static JSON; `/readyz`: database application identity marker, 200/503 | `crates/simple-agents-service/src/lib.rs` |
| Meteonesto weather API | `/health/live`: static JSON; readiness depends on a current GFS or IFS publication; records metrics | `weather-api/src/api.rs` |
| Meteonesto gateway | Readiness requires OIDC keys and a successful upstream health request with its existing timeout; failure includes request correlation | `weather-gateway/src/app.rs` |
| SCT | Static liveness, separate database health, and writer readiness | `crates/sct-server/src/lib.rs`, `api.rs` |

This is a design inventory, not verified adoption or a complete component audit.
The pilot is Favzetto: it exercises both liveness and ordered dependency checks.
The completed rollout is recorded in [the adoption tracker](migration-status.md#step-05-rollout--complete-locally): 14 adopted and 3 N/A after scope assessment.

## Contract

The independent, opt-in `health` feature does not enable Axum, Tokio, lifecycle,
logging, database libraries, or authentication. Its public API uses standard Rust,
HTTP and Tower types; no Axum types or trait bounds are exposed.

- `Probe::liveness()` succeeds without dependency checks. Serving a response proves
  only that the process can answer that request.
- `Probe::readiness(first_check)` requires an explicit application check.
  `with_check` appends checks. Applications can put any composite/alternative
  condition inside one callback; the library does not invent database queries.
- `Check::new(name, callback)` creates a fresh asynchronous check on every run.
  Checks run sequentially in registration order, stopping at the first error.
  Success means every registered check returned success for this invocation.
- `Check<E, T = ()>::run()` retains a typed success payload `T` as well as
  the original error. This supports detailed aggregate reports without polling
  dependencies twice, caching across requests, or using mutable side channels.
  The application owns aggregation (for example, probe every engine and require
  at least one healthy engine), and maps the resulting payload to its response.
  `Probe` continues to compose unit-valued checks sequentially.
- `Probe::run()` returns `Result<(), CheckFailure<E>>`, retaining the failed
  check's name and the original application error. Nothing is cached, logged,
  serialized, or exposed to clients automatically.
- `Probe::endpoint(renderer)` is a cloneable Tower service returning an ordinary
  HTTP response. The application renders both success and failure, retaining
  its exact status, headers and body. It may keep a handler and call `run()`
  directly when request-specific extraction or formatting is needed.
- Endpoints leave method dispatch, path, auth, rate limits, metrics, and HEAD
  body suppression to the enclosing router. With today's transitional router,
  mount using `get_service`. The adapter itself accepts all methods, like other
  services placed behind a method router; it never reads a request body.
- Tower `poll_ready` is capacity readiness, not an application health probe.
  Each HTTP call evaluates independently, without serialization or spawning.
- No implicit timeout, polling interval, background task, startup state, shutdown
  readiness switch, retry or concurrency limit. Existing policies stay with the
  application. Timeouts can wrap individual checks using the application's
  runtime and error mapping. Dropping `run()` drops the active check future;
  cancellation of blocking work or detached tasks is the callback's responsibility.
- There is no aggregate health JSON schema. Sensitive error details remain under
  application response policy. Readiness failure need not be 503 when an existing
  service has a different compatibility contract.

## Canary acceptance

Favzetto must preserve `/health` JSON including version, `/ready` JSON, current
AppError status/body, database/storage/PDF order, unauthenticated access,
GET/HEAD/405 behavior, and rate limits. No new probe routes or deadlines.
Test success and failed dependencies, liveness during readiness failure, recovery,
method handling, existing middleware and real startup/shutdown. Record baseline
failures separately and integrate only through the worktree workflow.

## Shared-library verification

Baseline: all-feature suite passed before edits (local socket permission required).
After implementation: 68 all-feature tests/doctests passed; four health tests
also passed with `--no-default-features --features health`. These cover ordered
short-circuiting, original non-Clone errors, recovery without caching, concurrent
requests, cancellation, custom response preservation, and GET/HEAD/405 through
the transitional router. Strict all-target/all-feature Clippy and formatting
passed. The minimal normal dependency graph contains only `http` and
`tower-service` (plus HTTP's bytes/itoa); a no-feature build also passed.

## Canary outcome

Favzetto adopted both production endpoints in `14d5b0d` on local `master`.
Four new health regressions passed against both old and shared implementations.
232 unit/API/process tests passed; two previously recorded catalog tests fail
identically to baseline. See the [central evidence](migration-status.md#step-05-health-and-readiness)
for scope, lint debt, integration, cleanup and checks not rerun. The subsequent rollout completed the remaining applicable production consumers;
see the central tracker for per-service scope and limitations. No push or deployment.

## Rollout extension: value-preserving checks

Simple AI's inference runner requires an aggregate report on both healthy and
unhealthy outcomes, including every engine's metadata. `Check<E, T = ()>::run()`
now supports a typed success value without changing unit-valued `Probe` checks.
A regression verifies complete reports on both outcomes, a non-Clone payload,
and one fresh evaluation per invocation. The extended library passes 69
all-feature tests/doctests, five minimal-feature health tests, and strict Clippy.
