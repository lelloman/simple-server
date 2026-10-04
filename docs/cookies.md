# Mutable HTTP cookies

The optional `cookies` feature enables `web::cookies` and `web`. It works without
`auth`, `auth-cookies`, or the legacy `tower-cookies` compatibility feature. It
adds no middleware unless the application installs it.

```rust
use simple_server::web::{Router, routing::get};
use simple_server::web::cookies::{Cookie, CookieManagerLayer, Cookies, SameSite, time};

async fn set_preference(cookies: Cookies) -> &'static str {
    cookies.add(Cookie::build(("preference", "compact"))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(7))
        .build());
    "saved"
}
let app: Router = Router::new()
    .route("/preference", get(set_preference))
    .layer(CookieManagerLayer::new());
```

`Cookies`, `CookieManagerLayer`, `CookieManager` and their response future belong
to simple-server. They contain no public Axum or Tower Cookies types and do not
wrap the external Tower Cookies jar. Parsing/delta bookkeeping uses the standard
`cookie` crate internally. `Cookie`, `CookieBuilder`, `SameSite`, `Expiration` and
`time` are exposed from that data library, just as shared HTTP APIs expose the
standard `http` value types; they are not backend framework types.

## Parsing and state

The layer creates a fresh request jar from all Cookie header values and installs
it as a typed request extension. Parsing is lazy. Each UTF-8 header is split at
semicolons and parsed with the cookie library's percent-decoding parser.
Malformed/undecodable cookies are skipped; a non-UTF-8 header is skipped in full.
Duplicate names use the last parsed occurrence across header values. This is the
existing Tower Cookies contract, not a new credential-selection policy.

`get(name)` returns an owned cookie. `list()` returns an owned snapshot, with
unspecified order. Repeated handler/middleware extractions and `clone()` share
one request's mutation state behind a short-lived lock. No user callback runs
under the lock. Different requests have independent jars. Body bytes, streaming,
status, headers, response extensions, service readiness/errors and cancellation
remain owned by the inner service.

An absent layer returns HTTP 500, `text/plain; charset=utf-8`, with exactly
`Can't extract cookies. Is \`CookieManagerLayer\` enabled?`. For custom composition,
`Cookies::default()` makes an empty detached jar and `Cookies::from_headers()`
captures input headers. Applications may install this extension themselves and
explicitly propagate its deltas; extraction does not emit output on its own.

## Mutation and output

`add(cookie)` adds or replaces by name. `remove(cookie)` removes an original
incoming cookie and generates an expiry delta using the supplied path/domain and
attributes. Removing an absent cookie, or adding then removing a cookie that was
not original, produces no deletion header. To expire a cookie regardless of
whether it arrived, add an explicit empty cookie with `Max-Age=0` instead; this
matches lello-auth's existing logout policy.

Reading alone emits no cookies. On a successful downstream response—including
an HTTP error response or extractor rejection—the layer appends each valid delta
as a separate Set-Cookie header, preserving existing fields. It does not merge
or overwrite them. Cookie serialization and attribute behavior follow the cookie
value library, preserving the legacy contract. Invalid HTTP header values are
skipped rather than panicking; other deltas still propagate. Output order is
unspecified. An inner service error propagates unchanged, without a fabricated
HTTP response. The layer does not poll or collect response bodies.

`delta()` snapshots the pending cookies without draining them.
`append_delta(&mut HeaderMap)` applies the same serialization policy manually and
returns the number appended. These methods allow inspection/custom validation
without an automatic security policy. Deltas are not cleared: append once per
response to avoid duplicates. Mutations after response headers have been handed
off cannot change the already-sent response.

Install the layer once, outside all cookie-consuming middleware and routes. As
with the legacy layer, every installation creates/replaces its own extension;
installing it twice is not deduplicated. There is no implicit session store,
signing/encryption, CSRF validation, domain/path, SameSite, Secure or HttpOnly
policy chosen by the layer. Those decisions remain application-owned.

## Relationship to authentication and compatibility

`auth-cookies` selects incoming credentials for authentication; it does not own
mutable browser state or response headers. This module also serves anonymous
forms, CSRF nonces, preferences and OIDC/device-link state. Use it alongside auth
when required, without moving those policies into the shared cookie layer.

Existing `tower-cookies` extraction remains available unchanged for consumers
that have not migrated. It is a separate typed jar and layer; enabling both
features does not automatically bridge their extensions. To migrate lello-auth,
replace imports with `simple_server::web::cookies`, enable `cookies` instead of
`tower-cookies`, and remove direct Tower Cookies dependencies after testing the
server and embedded/external-OIDC examples. Current consumers still use published
older versions; implementation availability is not consumer adoption.

The feature is implemented locally and is not yet published in a new crate
release. Existing published 0.1.3 does not contain it.
