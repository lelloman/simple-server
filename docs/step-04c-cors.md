# Step 04c: explicit CORS configuration

The optional `cors` feature provides `cors::CorsConfig` and a validated Tower
layer, service and future. It works with default features disabled and does not
depend on Axum, lifecycle, logging, correlation or tracing. Public interfaces use
HTTP primitives and Tower traits; tower-http is an internal implementation detail.

## Contract

- Default configuration grants no cross-origin permissions. Applications explicitly
  select origins, methods, allowed request headers, exposed response headers,
  credentials and optional preflight max-age. Setters replace previous choices.
- Origins are exact `http::HeaderValue` matches; origin parsing and configuration
  validation stay application-owned. Empty origin lists grant no origin access.
- Wildcards require explicit `allow_any_*` / `expose_any_header` setters. `build`
  returns an error for literal `*` list entries or any wildcard combined with
  credentials. Validation is independent of setter order and happens at startup.
- This is response-header policy, not authentication or CSRF protection. Denied
  origins do not cause an HTTP rejection or prevent handler execution. Credential
  headers alone grant nothing without a matching allowed-origin header.
- Preserve tower-http CORS behavior: all OPTIONS requests short-circuit the inner
  handler with a default-body 200 response, including OPTIONS without Origin.
  Ordinary responses preserve status, extensions and the untouched body; inner
  readiness and errors propagate. No response buffering or per-request boxing.
- Default Vary adds Origin, Access-Control-Request-Method and
  Access-Control-Request-Headers, appending to any existing response Vary. Computed
  CORS headers replace matching inner headers; absent computed headers do not
  remove inner headers. Applications should avoid conflicting inner CORS policy.
- CORS headers apply to ordinary success/error HTTP responses. Inner service
  errors remain errors; this layer does not turn them into responses.
- Application layer placement remains unchanged, including which authentication,
  security-header, correlation and trace layers wrap preflight responses.

No automatic permissive preset, origin reflection, dynamic predicates, private
network policy, custom Vary or framework escape hatch is provided in this initial
scope. Services requiring these remain pending until their needs are supported.

## Qualification and canaries

The shared contract tests compare 300 policy/origin/method/status combinations
against the previous middleware, including existing headers, preflight bypass,
HTTP errors, extensions and bodies. Separate tests cover invalid configurations,
restrictive defaults, setter replacement, readiness, service errors and opaque
body identity. Minimal-feature tests ensure CORS works without Axum.

Crumbles and Simple AI are the selected canaries: configured credential-aware
allowlists versus wildcard backend and runner policies. The backend deliberately
does not expose all response headers; the runner does. Before/after service tests
must preserve this difference, Crumbles' CSRF/correlation headers, preflights and
middleware placement. See both adoption trackers for actual integration evidence.

Both local canaries are now integrated: Crumbles master `433d709` and Simple AI
master `f73daa6`, using shared source `3aa9332`. Crumbles preserves security headers
but no correlation header on preflights, matching its previous middleware order.
The remaining 15 products await applicability assessment and rollout.
