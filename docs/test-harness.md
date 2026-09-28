# Owned HTTP test harness

Enable `test-harness` in a dev-dependency to use `simple_server::testing`.
Enable `test-harness-ws` only when tests need a WebSocket client. Neither feature
is enabled by default; neither requires `web-compat`. The harness accepts the
owned `web::Router` directly and exposes no Axum, reqwest or tungstenite types.

## Fixtures and requests

`TestServer::new(router)` invokes a router in-process without a listener.
`TestServer::tcp(router).await?` starts a real loopback HTTP fixture with an
OS-assigned port, including peer connection information. `bind` accepts an
explicit loopback IP/port, including IPv6. `base_url` and `address` return `None`
for in-process fixtures; TCP URLs can be passed to application-owned clients or
external test processes. TCP construction/use requires a Tokio runtime with
network and time enabled. Construction does not initialize logging or signals.

```rust
use simple_server::{testing::TestServer, web::{Router, routing::get}};

# async fn example() -> Result<(), simple_server::testing::TestError> {
let server = TestServer::new(Router::new().route("/health", get(|| async { "ok" })));
let response = server.get("/health").send().await?;
response.assert_status_ok();
response.assert_text("ok");
server.shutdown().await?;
# Ok(()) }
```

Request methods include `get`, `post`, `put`, `patch`, `delete`, `head`, and
`request(Method, path)`. Use `header`, `headers`, `query`, `json`, `form`, `body`,
or `multipart`, followed by explicit `send().await?`. JSON/form serialization
and malformed headers return errors when sending. Paths must be origin-relative
and have no fragment; requests cannot select another host. URL encoding and
normalization are shared between the transports. Redirects and environment proxies
are disabled. Cookie jars are deliberately absent: set Cookie/Authorization
headers explicitly and inspect repeated Set-Cookie headers in the response.
Each fixture has its own HTTP client and router.

`TestResponse` exposes status, repeated headers and exact bytes. `text` validates
UTF-8 and `json` deserializes into a caller-selected type; both return errors.
Assertions include status, text and JSON-value equality with caller locations.
HTTP errors such as 401/404 are normal responses, not transport failures.

`MultipartForm` preserves ordered/repeated fields and binary data. `Part` supports
optional filename/MIME metadata. Names and filenames escape quotes/backslashes;
CR/LF/NUL metadata is rejected. The generated boundary is checked against each
part's bytes. Multipart building requires no production multipart feature; tests
that parse uploads still enable the service's normal extractor feature.

## Bounds and shutdown

`TestOptions` defaults to a five-second total request timeout and an 8 MiB
response limit. Customize via `with_options` for large downloads or slower tests.
The timeout covers HTTP dispatch and body collection. TCP responses are collected
incrementally with a byte limit; in-process responses use the owned bounded body
collector. Body/transport/timeout errors are returned with their original source.
This API buffers finite responses. It does not provide a streaming SSE client;
use an application-owned streaming client against a TCP fixture for SSE tests.

`shutdown().await?` requests graceful shutdown and waits for the server task up
to the configured timeout; timeout aborts that task and returns an error. Drop
requests shutdown and aborts the serving task without waiting. As with production
serving, upgraded sockets and application-spawned tasks remain independently
owned: close/drop clients and stop application tasks before graceful shutdown.

In-process fixtures have no socket, peer address or upgrade support. They are
intended for handler/middleware tests, rather than byte-for-byte HTTP framing
comparisons. Use TCP when network semantics, external clients, connection info,
or upgrades matter. TLS serving is a separate pending capability.

## WebSocket tests

With `test-harness-ws`, call `server.get("/ws").header(...).websocket().await?`
on a TCP fixture. The request must be GET without a body. Auth and subprotocol
headers are forwarded. `WebSocketOutcome::Connected` contains an owned client;
`Rejected` exposes HTTP status/headers. A rejection body contains only the bytes
received with the handshake and may be incomplete. Use HTTP `send` with explicit
upgrade headers if a full rejection body must be tested.

Clients send/receive `web::ws::Message`, including text, binary, ping/pong and
close frames. Negotiated response headers remain available. Each operation has
the configured timeout; incoming frames/messages have the response byte limit.
`receive` returns `None` on EOF and protocol errors as errors. Close the client
before stopping the fixture. Successful connection does not change application
authentication or automatically accept WebSocket authorization.

## Adoption

Fausto and LelloStore have adopted owned HTTP fixtures; LelloStore also uses
owned multipart/WebSocket test clients. Pezzottify still has raw Axum upstream
mock listeners that could use TCP fixtures. Legacy parser/error comparison
oracles remain separate decisions. Library availability alone does not imply
adoption.
