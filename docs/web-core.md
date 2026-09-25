# Shared HTTP core and Pezzottify canary

The opt-in `web` feature supplies simple-server-owned routing, handler,
extraction and response interfaces. Axum remains the internal backend; application
handlers do not implement its traits or return its types. The feature does not
enable auth, logging, background tasks or lifecycle automatically.

```rust
use simple_server::web::{Json, Router, State, routing::post};

async fn echo(State(prefix): State<String>, Json(value): Json<String>) -> Json<String> {
    Json(format!("{prefix}{value}"))
}

let app: Router = Router::new()
    .route("/echo", post(echo))
    .with_state("echo: ".to_owned());
```

## Routing and serving

`Router<S>` represents routes awaiting application state `S`. `route`, `merge`,
`nest`, `fallback` and `with_state` compose shared routers. `routing::{get, post,
put, patch, delete, head, options, trace}` produce a `MethodRouter<S>` with matching
chainable method functions. Their bounds use the shared `Handler<T, S>` contract.
Functions with zero through sixteen arguments implement it automatically.

GET also handles HEAD, stripping the response body while retaining its headers.
Unmatched paths return 404, unsupported methods return 405 with Allow, and literal
paths take precedence over captures. Nesting, percent-decoded path captures,
trailing-slash behavior and route-conflict validation preserve the current pinned
backend's behavior. Conflicting or invalid route definitions may panic during
construction. Fallback handlers are explicit and application-owned.

After state is supplied, `Router<()>` implements Tower `Service` for the shared
request and response types with an infallible service error. With `lifecycle`
enabled, `web::serve(listener, router, shutdown)` serves it without exposing backend
types. It uses the existing graceful drain contract and installs no signals.

With `body-limit` enabled, `.body_limit(BodyLimit::max(n))` applies the existing
shared limit to routes already registered. It limits cooperating extractors,
not arbitrary raw-body reads. Installing no limit retains the backend's 2 MiB
default for JSON, bytes and string extraction.

## Extraction

- `State<T>` clones application state, or obtains substate via the shared
  `FromState<S>` trait. It does not use the backend's FromRef trait.
- `Path<T>` and `Query<T>` deserialize request-head values using serde.
- `Json<T>`, `String`, `Bytes` and raw `Request` can consume the body. Only the
  last handler argument may consume it; invalid signatures fail to compile.
- Custom head extraction uses the existing shared `FromRequestParts<S>` trait.
  `Extract<T>` continues working, including existing required/optional Session
  policies. Custom body extraction uses the new `FromRequest<S>` trait.
- Extractors run left-to-right. A rejection prevents subsequent extraction and
  handler execution. Mutated request parts remain visible to subsequent arguments;
  the body is neither pre-read nor buffered by the handler adapter.
- `HeaderMap` reads request headers. `Result<Extractor, Rejection>` explicitly
  captures errors for custom response policies. No blanket optional extraction
  silently discards failures.

Built-in extraction errors use the existing buffered `RejectionResponse` and
preserve status, content type and error text: malformed path/query/JSON → 400,
JSON data errors → 422, missing/unsupported JSON media type → 415, extractor body
limit → 413. This remains customizable by capturing the rejection. No application
JSON error envelope, authentication policy or CSRF policy is imposed.

## Responses and bodies

`Request<B = Body>` and `Response<B = Body>` are standard `http` messages whose
default body is owned by simple-server. `Body` has private backend storage,
implements standard `http_body::Body`, accepts strings/bytes, and offers
`collect(limit)` with an explicit maximum. Body failures use a shared `BodyError`.

The shared `IntoResponse` trait supports JSON, strings, bytes, status codes, empty
responses, bodies, standard HTTP responses, buffered extraction rejections,
`Result`, and `(StatusCode, R)`, `(HeaderMap, R)` and `(StatusCode, HeaderMap, R)`.
JSON uses application serde types and preserves the existing encoding/content-type
contract. Response maps preserve extensions, headers (including repeated cookies),
status and body. Applications implement the trait for domain errors; there is no
blanket assumption about error status or disclosure.

## Incremental compatibility and canary

The separately enabled `web-compat` feature exposes one explicitly transitional
function, `web::compat::into_axum_router`, to mount shared route groups inside a
legacy router. This is an intentional temporary backend type escape; it is not
required by a standalone shared router and will be removed with the Axum re-export.
Keep these conversions at application assembly boundaries, outside shared handlers.
Existing outer middleware and extractor-limit extensions continue to apply.

Pezzottify's five embedding endpoints are the first canary: list, get, upsert,
delete and search. Its embedding module uses shared routers, state/path/query/JSON,
session extraction, response traits and standard status codes. Two conversions
at the legacy route assembly boundary preserve existing auth, permissions, rate
limits, CSRF and route placement. Its ApiError adapter delegates to the existing
single buffered renderer. New real HTTP embedding tests passed before migration
and again afterward; central trackers contain commits and test evidence.

## Remaining scope

This is the ordinary HTTP foundation, not completion of Axum removal. General
middleware/route layers, connection metadata, redirects/forms, additional
extractors, multipart, SSE, streaming producers/ranges, WebSocket protocols,
alternate listeners/TLS and framework-independent integration test fixtures still
need APIs as their consumers migrate. Existing middleware remains at the legacy
assembly boundary in this canary. The other Pezzottify route groups still use
transitional APIs. Module status totals do not imply full abstraction.

Validation includes differential request/response checks against the prior router,
before/after consumer HTTP tests, head-before-body ordering, explicit rejection
capture, substate extraction, sixteen-argument handlers, compile-fail body ordering,
shared body limits, Tower composition and real HTTP serving/shutdown.
