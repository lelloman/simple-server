# Step 04 proposal: HTTP request and response policies

Status: module split accepted. [04a body limits](step-04a-body-limits.md) is
implemented and rolled out locally; see the [adoption tracker](migration-status.md)
for applicability and verification evidence. [04b response headers](step-04b-response-headers.md)
is implemented with canary qualification tracked separately; 04c remains planned. The source assessment below itself involved
no runtime tests.

The old “HTTP support” roadmap label conflates unrelated concerns. The five
reviewed services already serve HTTP; the useful next extraction is optional
request/response policy mechanics. Applications keep policy values, route
placement, error formats and domain decisions. None is enabled automatically.

## Reviewed sources

Inspected local development checkouts on 2026-09-21: Crumbles master `4a44b33`,
Pezzottify dev `69988032`, Meteonesto master `9a83f3e`, Simple AI master `2b9c6a6`,
Simple Agents main `62b44a3`. These are checkout observations, not clean-tree
qualification; unrelated working changes were not modified.

Paths below are relative to each consumer repository.

| Service | Existing HTTP mechanics | Important differences / source evidence |
| --- | --- | --- |
| Crumbles | Configured multipart body ceiling, smaller setup limits, explicit CORS, browser security headers, embedded frontend fallback | `crumbles/src/server/mod.rs` builds a body ceiling from upload size plus multipart overhead. `routes/setup.rs` has its own ceiling. `security_headers.rs` preserves existing resource headers except configured HSTS; CSP depends on OIDC origin and transport configuration. `error.rs` owns nested errors, field validation, version conflicts and correlation IDs. |
| Pezzottify | Route-specific body limits, API no-store default, opt-in private caching, streaming/range responses and static frontend | `pezzottify-server/src/server/ingestion_routes.rs` permits 5 GiB multipart requests; `route_builder.rs` has a 2 MiB limit for one route group. `http_layers/http_cache.rs` preserves explicit Cache-Control, merges Vary, excludes partial/media/SSE responses and covers early API errors through an outer layer. `stream_track.rs` owns media range semantics. `api_error.rs` owns incident IDs and retry headers. |
| Meteonesto | Pipeline body cap, header-time timeout and concurrency admission; gateway upstream deadline; weather response caching | `weather-pipeline/src/control_plane.rs::request_envelope` checks declared length, acquires a permit and times out handler execution; the router also applies DefaultBodyLimit. Its failures use application/problem+json and existing request IDs. `weather-gateway/src/app.rs::proxy` applies a deadline to both upstream headers and chunk reads. `weather-api/src/api.rs` has ETags/cache policy and computation deadlines. These are distinct timeout contracts. |
| Simple AI | Backend and runner CORS, route-specific multipart ceilings, long-lived inference streaming | `backend/src/main.rs` permits any origin/method/header. `inference-runner/src/main.rs` uses permissive CORS. Backend OCR/extract limits are 25 MiB and audio 200 MiB; runner OCR is 100 MiB and audio 200 MiB. `backend/src/routes/chat.rs` retains a capacity reservation in streaming state and finalizes domain accounting on stream termination/drop. |
| Simple Agents | Repeated route body limits and no-store response layers; UI CSP; custom browser boundary; session SSE | `crates/simple-agents-service/src/sessions_http.rs` applies 5 MiB and overwrites Cache-Control with no-store; auth/broker/native endpoints have smaller limits. `ui.rs::browser_boundary` checks configured origin, client marker and Fetch Metadata, rejects browser runner access, and renews sessions. This is application authentication/CSRF behavior, not a CORS layer. Errors use versioned protocol types. |

## Recommended optional modules

### 04a: Request body limits — first pilot

Provide a small public interface for route-scoped body ceilings. All five
services have real adoption candidates. Preserve extractor behavior and rejection
responses first; do not silently replace an extractor limit with an unconditional
wire-body limiter. Multipart part/file limits remain application-owned.

A one-line wrapper alone offers little deduplication: its value is establishing
a tested library boundary toward removing direct Axum usage. Require explicit
byte limits and route placement; no new global default. Any stronger streaming
body enforcement would be a separately specified behavior change.

Suggested pilot: Simple Agents, which has several repeated route declarations.
Second canary: Pezzottify, to verify large multipart exceptions and media routes.
Acceptance: same below/at/above-limit behavior and rejection bodies, requests
without Content-Length, existing route overrides, and unchanged response streams.

### 04b: Response header policies

Extract reusable header application with explicit insert-if-absent versus
replace semantics, plus correct Vary merging. Start with no-store and explicit
cache/header values. Preserve repeated values, existing headers, status, body
frames and extensions. Route selection and cache eligibility remain local.

Evidence: Pezzottify's default no-store differs from Simple Agents' unconditional
no-store, and Crumbles preserves resource-specific CSP. An undifferentiated
“secure defaults” middleware would change behavior. CSP generation, trusted-proxy
configuration, ETag generation, auth-sensitive caching and range policy remain
application decisions.

Suggested pilot: Pezzottify and Simple Agents. Acceptance includes early auth/
limit failures, existing Cache-Control, Vary duplicates/wildcards, HEAD, partial
responses and SSE, without body buffering.

### 04c: CORS configuration — smaller, independently useful candidate

Expose explicit origins, methods, allowed/exposed headers and credentials over
the existing implementation. Crumbles and Simple AI are contrasting consumers:
configured credential-aware allowlists versus permissive access. Preserve each
service's behavior and preflight/layer placement; introduce no permissive default.
Simple Agents' browser boundary is outside this module.

This is primarily API encapsulation rather than substantial duplicate-code
removal. Qualify preflights, ordinary/error responses and exposed correlation
headers before migration. Keep it optional even if 04a/04b are adopted.

## Defer from the initial step

- **Timeouts/admission:** Meteonesto provides a concrete candidate, but handler
  timeout, upstream whole-response deadline and long-lived SSE/WS need distinct
  contracts. Never apply a global timeout to Simple AI or session streams by
  default. Admission also overlaps future task/rate-limiting work.
- **Universal errors or validation:** existing public schemas and error mappings
  differ. Keep them local; future shared failures should allow application-owned
  response construction instead of mandating a new JSON envelope.
- **Compression:** the searched Rust sources did not establish repeated installed
  compression middleware in these five consumers. Dependency availability is not
  evidence of adoption.
- **Static assets, ETags, range handling and router replacement:** current uses
  include embedded SPAs, file serving, cached weather products and audio streams.
  They need a separate focused inventory before extracting a common contract.
- **Authentication/CSRF, health, metrics and rate limiting:** retain their existing
  dedicated scope; do not fold them into generic HTTP policy helpers.

The matrix separates 04a, 04b and 04c. The 04a rollout is complete locally, with
eleven products adopted and six assessed N/A. The [04b contract](step-04b-response-headers.md) now defines header operations;
04c still requires a separate contract before implementation.
