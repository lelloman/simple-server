# Request extraction foundation

The optional `extract` feature provides application-owned request-head extraction.
It depends only on `http`, works without a runtime, and exposes no Axum types in
the application contract. Enable `http` as well to use its internal framework
adapter with existing routers and middleware.

```rust
use simple_server::extract::{Extract, FromRequestParts, Parts, StatusCode};

struct AppState;
struct Session { user_id: usize }

impl FromRequestParts<AppState> for Session {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Call the application's verifier / AuthLayer::authenticate here.
        // No headers or extensions are trusted automatically by this API.
        let _ = (parts, state);
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn handler(Extract(session): Extract<Session>) -> String {
    session.user_id.to_string()
}
```

## Contract

- `FromRequestParts<S>` receives mutable standard HTTP request parts and borrowed
  application state. It returns a `Send` future without requiring allocation.
- `Extract<T>` delegates to that trait each time it is requested. It neither
  caches values nor installs authentication middleware. The domain value can be
  obtained by destructuring or `into_inner()`.
- Extraction cannot consume the request body. Request-head mutations survive for
  subsequent extractors and middleware. The existing router controls order.
- Optional behavior is explicit: implement the trait for `Option<Session>` and
  request `Extract<Option<Session>>`. There is no blanket conversion from failure
  to `None`, and no implicit `Option<Extract<Session>>` adapter. Applications decide
  whether missing or invalid credentials are anonymous, and which errors remain
  failures. For generic states, a local optional-session newtype can be used to
  satisfy Rust's orphan rules.
- Rejections implement `IntoRejectionResponse`. `StatusCode` produces an empty
  response; `Infallible` covers infallible extraction. `RejectionResponse` wraps
  `http::Response<Vec<u8>>`, preserves headers (including duplicates), extensions,
  status and body bytes, and converts internally to the HTTP backend's body.
  `From<http::Response<Vec<u8>>>`, `into_http()`, and dereferencing allow standard
  HTTP construction and inspection. Rendering is application-owned; there is no
  built-in JSON envelope, redirect policy, auth status or error disclosure policy.

The auth module remains independent. A session extractor can use
`AuthLayer::authenticate` and preserve its own validation, permissions and error
policy. Existing identity extensions are not automatically accepted as proof of
authentication by the extraction module.

## Incremental adoption and remaining work

Pezzottify is the first consumer. Its session implementations use this contract;
handlers and permission/rate middleware use `Extract<Session>` or
`Extract<Option<Session>>`. Direct extraction in report admission calls the same
trait. Its session source file no longer imports Axum.

The [shared HTTP core](web-core.md) now adds routing, built-in state/path/query/JSON
extractors and ordinary successful responses. Pezzottify's embedding endpoints
are its first canary; other routes still use transitional APIs. General middleware,
multipart, SSE, WebSockets and streaming need further interfaces before the Axum
re-export can be removed. Buffered extraction rejections do not establish the
streaming-response contract.

Validation covers a standalone `extract` build, application state borrowed across
an await, required/optional policy, repeated extraction, request-head mutations,
unchanged request bodies, and exact rejection bytes/status/headers/extensions.
Consumer verification is recorded in both migration trackers.
