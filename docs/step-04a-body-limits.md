# Step 04a: extractor body limits

The optional `body-limit` feature provides `body_limit::BodyLimit::max(bytes)`.
It requires HTTP but not lifecycle, logging, correlation or tracing. It introduces
no default limit and installs no middleware unless explicitly applied.

```rust
use simple_server::{axum::{Router, routing::post}, body_limit::BodyLimit};
let app: Router = Router::new()
    .route("/upload", post(|body: String| async move { body }))
    .layer(BodyLimit::max(1024));
```

## Contract

- Preserve Axum 0.8.9 `DefaultBodyLimit::max` semantics and layer ordering.
- Explicit byte count; zero allows only empty extracted bodies. There is no
  implicit service-wide policy or constructor default.
- Applies to cooperating extractors such as Bytes, String, JSON and Multipart.
  Raw body readers and custom extractors that bypass the default limit remain
  unaffected. This is not an unconditional wire-body security boundary.
- Keep each extractor's rejection status, headers and body. Applications retain
  custom rejection handling, domain validation and error envelopes.
- Enforce consumed bytes through existing extractors, including streamed bodies
  without Content-Length; do not add header-only enforcement or buffering.
- Multipart total-body limits do not replace application file/part limits.
- Preserve request/response semantics and stream laziness. Do not cap response
  bodies, impose timeouts, change WebSocket lifetimes or install subscribers.
- This is a composition boundary over the existing implementation, not a new
  limiter algorithm. Router and Layer interoperability remain transitional.

## Qualification and rollout

Shared differential tests compare legacy and shared limits for empty/below/at/
above-boundary byte and JSON extraction, invalid JSON, declared and absent lengths,
multipart framing, route-specific ceilings, raw reads and lazy response bodies.
The minimal feature build must work independently of lifecycle and observability.

Simple Agents is the pilot; Pezzottify is the large-upload canary. Migrate actual
production declarations without changing byte values or placement. Verify on
original behavior before replacement, then test and integrate dedicated worktrees.
The local rollout is complete: eleven products adopted the shared limit and six
are N/A after source assessment. See the [adoption tracker](migration-status.md)
for per-service verification, existing failures and integration evidence.
04b response headers and 04c CORS remain planned; their adoption is independent.
