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

## Router composition and middleware

`Router::layer` applies a Tower layer to existing routes and fallbacks;
`route_layer` applies it only to matched routes, preserving unmatched 404s.
Layers receive the shared `routing::Route` service and operate on shared
`Request` / `Response` bodies. The last installed layer runs first. Requests,
headers, extensions and body streams cross the internal adapter without buffering.

`middleware::from_fn` and `from_fn_with_state` accept async functions with up to
eight shared head extractors followed by `Request` and `Next`. Extractors run in
order and can reject before the middleware or downstream handler runs. `Next::run`
waits for downstream readiness. State here is the explicitly supplied middleware
state, independent of the router's handler state.

`Extension<T>` installs a cloned request-scoped value as a Tower layer and reads
it as an extractor. A missing required extension is a server error. `MatchedPath`
contains the bounded route template; it is also available from request extensions
inside middleware. Unmatched fallback requests have no matched path.

`fallback_service` accepts a standard Tower service over shared requests and any
standard HTTP response body. This supports static-file services without exposing
backend router types. Service readiness is preserved. `routing::any` registers a
handler for all methods.

`Body::new` wraps a standard HTTP body; `Body::from_stream` forwards a fallible
byte stream lazily. Dropping the response drops its stream. Bounded collection
is available through `Body::collect` and `body::to_bytes`.

With `lifecycle`, `serve_with_connect_info` inserts the direct TCP peer address as
`ConnectInfo<SocketAddr>` for handlers and middleware. It deliberately ignores
forwarded headers. Both serving functions use the existing graceful shutdown
contract; neither installs signal handlers.

## Explicit protocol compatibility

`web-compat` also supplies transitional `Multipart` and `WebSocketUpgrade`
extractors (requiring the respective features), `response` for legacy streaming
responses, and `trace_with_observer` for existing tracing observers. These keep
ordinary handler signatures and router composition shared, but intentionally
retain backend multipart field/error, WebSocket and observer contracts. They are
migration boundaries, not completed protocol abstractions. SSE producers can use
`compat::response` without buffering their event stream.

## Pezzottify adoption

The five embedding endpoints were the first canary. Pezzottify now composes all
route groups, ordinary handlers, custom byte-range extraction, middleware,
static-file fallback, main/metrics serving and its shared HTTP test fixture using
`web`. The legacy embedding-router conversions and application `FromRef`
implementations are gone. Permissions, rate limits, CSRF, report admission,
caching, task ownership and range policies remain application-owned and unchanged.

The remaining explicit backend contracts are SSE event production, WebSocket
messages/sockets, multipart fields/errors, the tracing observer and independent
mock HTTP fixtures. Step 11 completion means shared routing and ordinary HTTP
contracts; it does not imply these remaining protocol abstractions are complete.

## Remaining shared scope

Forms, redirects, more extractors, fully shared multipart/SSE/WebSocket protocols,
backend-free tracing observers, alternate listeners/TLS and independent mock
fixtures still need APIs as consumers migrate. `compat::into_axum_router` remains
available for other incremental migrations, but Pezzottify no longer uses it.

Validation includes differential request/response checks against the prior router,
before/after consumer HTTP tests, extraction ordering, explicit rejection capture,
substate extraction, sixteen-argument handlers, compile-fail body ordering, body
limits, layer ordering, early rejection, service readiness, connection metadata,
body cancellation/trailers/errors and real HTTP serving/shutdown.

## Standard server integration and header arrays

A configured `Router` accepts standard `http::Request<B>` bodies with byte data,
not only the shared Body type. `Router::into_make_service()` returns an owned
Tower service factory that clones the router for each connection target. This
lets existing TLS servers consume shared routes directly without converting to
a backend router. It ignores connection targets and installs no peer metadata;
use `serve_with_connect_info` when that metadata is required. TLS configuration,
accept policy and shutdown remain owned by the calling server.

Response tuples also accept header arrays: `([(name, value); N], response)` and
`(status, [(name, value); N], response)`. Names/values use standard HTTP conversion
bounds. Entries replace matching headers in order; use HeaderMap append for
multiple cookie fields. Invalid conversions preserve the existing error response
contract. Androidoscopy exercises this with its controller login cookie and
shared routes served through its existing TLS adapter.

## Nested services, method fallbacks and request metadata

`Router::nest_service(prefix, service)` mounts a standard Tower service and strips
its prefix while preserving the query string. Route layers still run before that
service. This supports Crumbles' MCP transport without a backend router escape.
`routing::get_service` mounts standard services such as the shared health endpoint
with GET/HEAD semantics. `fallback_methods(MethodRouter)` preserves method-aware
fallback behavior, including HEAD stripping and 405/Allow responses.

`Uri` is a shared head extractor. `Option<Extension<T>>` distinguishes an absent
request extension from a present value; it does not suppress authentication or
other extractor errors. Required `Extension<T>` retains its missing-value error.

With `correlation`, `Correlation::run_http` establishes the existing ID/header and
task-local context for arbitrary standard HTTP request/response bodies. It neither
inspects nor converts those bodies. The existing backend `run` and `run_selected`
APIs remain compatible. Crumbles uses the new method with shared requests and
retains an explicit tracing compatibility boundary.

### Fausto compatibility requirements

`Correlation::run_selected` accepts standard HTTP request/response body types,
including the shared web body, while preserving application-selected opaque IDs.
`web::compat::WebSocketUpgrade::protocols` preserves server preference order when
negotiating a client-offered subprotocol.

The optional `multipart-owned` feature adds `web::compat::OwnedMultipart` for
consumers requiring owned multipart fields. It retains axum-extra parsing,
extractor body limits, rejection text, and runtime field exclusivity. Its field
and error types remain an explicit protocol compatibility boundary; this is not
yet complete multipart abstraction. The existing borrowed `Multipart` is unchanged.

### Browser and OAuth consumers

`web::Form<T>` preserves URL-encoded form behavior: GET/HEAD parse the query;
other methods require the form content type and parse the limited request body.
Parsing errors retain their HTTP status and text through `RejectionResponse`.
`Form<T>` can also render an encoded response. `Json<T>` supports field access
through `Deref`/`DerefMut` in addition to tuple destructuring.

`web::response::Html<T>` sets the HTML UTF-8 content type. `Redirect` provides
303 (`to`), 307 (`temporary`) and 308 (`permanent`) responses, retaining an HTTP
500 for a Location value invalid as a header. Header arrays may be returned
without an explicit body, including `(StatusCode, headers)` OAuth redirects.

`MethodRouter::layer` scopes a standard Tower layer to an individual method
router. `middleware::map_response` maps the downstream response, including
extractor failures; its function takes a shared `Response` and returns an async
shared `IntoResponse` value. Readiness remains handled by the existing `Next`
adapter, and bodies are not collected or otherwise inspected.

The opt-in `tower-cookies` feature implements the shared head-extraction trait
for `tower_cookies::Cookies`. The existing `CookieManagerLayer` owns parsing,
jar mutation and `Set-Cookie` output. A missing layer returns the same HTTP 500
text; no authentication policy or cookie defaults are introduced. Consumers can
disable tower-cookies' Axum extractor feature. This adapter is separate from
`auth` credential selection: it also serves unauthenticated browser flows and
response cookie mutations.

### Shared multipart interfaces

With `web` + `multipart`, use `web::multipart::{Multipart, Field,
MultipartError}` (also `web::extract::Multipart`). These are shared types with
private parser storage. A borrowed field enforces exclusivity at compile time.
With `multipart-owned`, `web::multipart::{OwnedMultipart, OwnedField}` provides
owned fields and runtime exclusivity, preserving the existing owned parser.
Both styles expose shared bytes, standard HTTP headers and shared errors only.

Fields expose `name`, `file_name`, `content_type`, `headers`, `chunk`, `bytes`,
`text`, and the standard `Stream<Item = Result<Bytes, MultipartError>>` contract.
`chunk`/`Stream` preserve lazy reads; dropping an upload releases its body.
`bytes`/`text` intentionally collect the field and remain subject to the request
body limit. Set `BodyLimit` on the route for the total request; application-specific
file/metadata limits, storage, validation and cleanup remain application-owned.
Missing/invalid boundary extraction returns the shared `RejectionResponse`.
Field/parser failures expose `status`, `body_text`, `Display`, an error source,
and shared response/rejection conversion without a backend error type.

The existing `web::compat::{Multipart, OwnedMultipart}` remain available with
their old field/error contracts for unmigrated consumers. They are distinct from
the new owned interfaces; changing the shared library does not automatically
remove remaining multipart exposure in other services. Lellostore uses the new
borrowed shared API for streamed uploads and bounded metadata.

`RawQuery` preserves the undecoded query and distinguishes absence from an empty
query. The WebSocket compatibility adapter now forwards `max_frame_size` and
`max_message_size`; the socket/message protocol types remain compatibility gaps.
