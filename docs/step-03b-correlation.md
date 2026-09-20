# Step 03b: request correlation

The optional `correlation` feature supplies request ID selection, request and
response extensions, task-local access and response-header propagation. It
enables HTTP and Tokio support but does not depend on the logging initializer
or lifecycle module. It works when polled without a Tokio runtime; it neither
starts a runtime nor spawns work. Existing default features are unchanged.

## Contract

- `Correlation::default()` uses `x-request-id` and always generates a fresh ID.
  `Correlation::new(HeaderName)` selects another header.
- `IncomingIds::AcceptValidated` explicitly trusts the first incoming header
  value if it contains 1–64 ASCII letters, digits, `-`, `_`, `.`, or `:`.
  Missing, invalid, non-ASCII and overlong values cause generation; they do not
  reject the request. Repeated headers use the first value, matching Crumbles'
  established contract. Comma-separated lists are invalid.
- Generated IDs contain 128 random bits encoded as 32 lowercase hex characters.
  Entropy initialization uses rand's system-seeded generator; failure to seed
  can panic. IDs are labels, not credentials or authorization evidence.
- `RequestId` is immutable, cloneable, displayable and available through request
  extensions (`Extension<RequestId>` in handlers). `current_id()` returns an
  `Option<RequestId>` while the downstream future is running.
- Request headers remain unchanged. The response header and response extension
  are overwritten with the selected ID, including error responses. Status,
  body, other headers and upgrade metadata remain untouched.
- The scope covers construction and polling of the downstream future through
  response creation. It restores an outer scope after nesting, cancellation or
  unwinding. Spawned tasks do not inherit it. Streaming body polling and upgraded
  connection lifetimes are outside that scope: explicitly capture the ID for
  deferred work. No body is read, buffered or rewritten.
- Applications own error envelopes, spans, audit records, outbound propagation
  and CORS header exposure. These should use the selected ID rather than parsing
  the original request header again. The module installs no subscriber or span.
- Install correlation once, outside middleware that can reject requests, and
  on the complete router (not only matched routes) to cover 404/405 responses.
  Middleware outside this boundary can still replace headers or responses.

## Use

```rust
use simple_server::{
    axum::{Router, middleware::from_fn_with_state},
    correlation::{Correlation, middleware},
};

let app: Router = Router::new()
    .layer(from_fn_with_state(Correlation::default(), middleware));
```

For existing application middleware, `Correlation::run(request, callback)`
provides the same selection/scope/propagation around an application-owned
response future. This permits existing tracing and error normalization to stay
inside the ID scope without coupling their policies to simple-server.

## Qualification

### Application-selected IDs

Services with an established wire contract can use
`Correlation::run_selected(request, HeaderRequestId::new(header_value), propagation, callback)`.
This explicitly bypasses `IncomingIds` and shared generation/validation. The
application owns selection, generator, validation bounds and rejection policy.
`HeaderRequestId` retains any legal HTTP header bytes, including opaque bytes,
empty strings and legacy lengths; it is not a validated `RequestId` and must
not be assumed safe for string interpolation or authentication.

`Propagation` selects `RequestHeader::{Unchanged, IfMissing, Overwrite}` and
`ResponseHeader::{Overwrite, Preserve}`. Defaults leave incoming headers alone
and overwrite the response. IfMissing/Preserve retain repeated header values.
The request extension and `current_header_id()` hold the selected ID. The
response extension reflects the final first header value, including an explicit
downstream override in Preserve mode. This compatibility mode can therefore
deliberately differ between the request ID and response ID.

The raw selected scope clears the validated `RequestId` extension and makes
`current_id()` return None, even when nested inside a validated request scope.
Applications may retain their own extension types. Default `run` additionally
provides HeaderRequestId/current_header_id alongside its validated API; all
existing validation, generation and propagation defaults remain unchanged.
Both paths share the same cancellation, nesting and deferred-body boundaries.

### Checks

Library tests cover generated-ID uniqueness samples, explicit trust, alphabet
and size limits, repeated headers, response consistency, custom header names,
handler extraction, 404/405/extractor errors, concurrent and nested requests,
spawn boundaries, cancellation, streaming and upgrade response preservation.
The feature is checked independently from logging and lifecycle.

Crumbles is the initial compatibility pilot. Its caller-ID acceptance,
`x-correlation-id` header, error envelope, tracing span, service context and
audit semantics must remain consistent. The integration daemon's durable
business correlation keys are a separate domain concept. Adoption and final
verification are recorded in the [migration tracker](migration-status.md).
