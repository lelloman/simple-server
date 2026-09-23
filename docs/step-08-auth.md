# Steps 08/09: one optional auth module

Authentication and authorization are designed and migrated together under the
single `auth` feature and `simple_server::auth` module. Step 09 is absorbed into
this milestone; Step 10 keeps its existing number. Database helpers (07) remain
deferred until after consumer Axum removal.

## Scope and consumer evidence

The first source assessment identified these actual production requirements:

| Service | Existing access flow | Ownership retained |
| --- | --- | --- |
| Favzetto | Exact Bearer or x-api-key, local administrator identity, admin checks; WebSocket query key forwarded by the application | Key configuration, fallback precedence, wire errors and route placement |
| LelloStore | Bearer → OIDC validation → roles/user registry → authenticated/admin extractors | Issuer/JWKS/claims validation, registry writes and outages |
| Simple AI | API key or JWT; explicit LAN identity only without Authorization; disabled-account checks | Trusted-network policy, API-key storage, OIDC and account state |
| Simple Agents | Strict single Bearer header; separate browser session flow; scoped durable permissions | Origin/session/CSRF boundary, revocation and transactional permissions |
| Crumbles | Auth session followed by named global/project/resource checks; some denials conceal existence with 404 | Permission model, transactional authorization and visibility semantics |
| Pezzottify | Session identity and named route permissions | Session persistence, CSRF, permission snapshots and business policy |
| Meteonesto | AccessControl-backed principal and permission checks with denial audit | Hot policy, principal kinds, audit writes and storage transactions |

These are design inputs, not claims of migration. Other services still require
assessment. No JWT implementation, OAuth flow, cookie format, password hashing,
role schema or database is mandated by this module.

## Public contract

- `Access<Context, Principal, Error>` runs a synchronous application verifier,
  then every registered check in order. `AsyncAccess` supports borrowed async
  callbacks via `AuthFuture`. Construction requires a verifier; a verifier-only
  flow accepts whatever identities that verifier accepts. Extra restrictions
  require explicit checks. No hidden anonymous fallback or permissive default.
- `evaluate` returns an identity only after all checks succeed. The first error
  is returned unchanged. `authorize` applies the configured checks to an identity
  already verified by the caller; it never authenticates or refreshes that identity.
  Context can include actions, loaded resources or transaction references. Keep
  mutation-sensitive checks in the application's transaction to avoid TOCTOU.
- `HeaderCredential` extracts an opaque header value, optionally after a literal
  scheme and one ASCII space. Scheme matching is explicitly exact or ASCII case
  insensitive. Values are never trimmed or decoded. Repeated headers and empty
  values are rejected by default; explicit compatibility options permit first-value
  selection and empty values. Invalid text, missing, repeated, wrong-scheme and
  empty errors contain no secret. Credential Debug is redacted; `expose` is explicit.
- Credential-source precedence and fallback are caller-owned. Parse errors must
  not silently trigger weaker identity fallback unless that is the service's
  deliberately preserved policy. This module does not compare stored secrets or
  validate token signatures; existing reviewed verifier libraries remain in use.
- `AuthLayer` adapts `AsyncAccess<http::request::Parts, ...>` to Tower. It preserves
  the body, headers, URI and unrelated extensions. An existing `Identity<P>` is
  removed before verification; the new identity is installed only after success.
  Mount it explicitly on protected routes. It does not exempt OPTIONS, change
  routing or install CSRF policy. Application rendering controls all failure
  statuses, bodies, challenges, redirects and headers, including provider outages.
- The layer forwards inner readiness and calls the same ready instance. Failed
  access never calls the handler. Evaluation cancellation drops the active future;
  no detached work is started. Committed application side effects cannot be undone.
- No credential logging, policy cache, timeout, retry, database, runtime or provider
  is installed. Clones share callbacks but do not retain request identities.

The feature depends only on `http`, `tower-layer` and `tower-service`. It builds
with default features disabled, without Axum or Tokio. Public types and bounds
contain no Axum APIs. The HTTP layer is optional to use; handlers/CLI/background
work can evaluate the same access flows directly.

## Example

```rust
use simple_server::auth::Access;

let access = Access::new(|token: &String| {
    if token == "test-fixture" { Ok("admin") } else { Err("unauthenticated") }
}).with_check(|principal, _| {
    if *principal == "admin" { Ok(()) } else { Err("forbidden") }
});
assert_eq!(access.evaluate(&"test-fixture".to_owned()), Ok("admin"));
```

The example is a fixture; production credential verification belongs in the
application callback. Resource checks can use `authorize` later with the verified
principal and a context containing the resource/action, preserving transaction
boundaries and application-specific 403/404 behavior.

## Verification and rollout

Library baseline: 137 tests/doctests; final: 145 pass. Strict all-feature/all-target
Clippy passes. Seven auth tests also pass with default features disabled; the normal
dependency tree contains only http, bytes, itoa, tower-layer and tower-service.
Added contract tests cover header ambiguity,
opaque values, redacted diagnostics, exact errors, check ordering, provider failure,
request-local identity, revocation, cancellation, stacked gates and inner readiness.
A real HTTP test exercises unauthenticated, forbidden, unavailable, successful and
public requests while counting actual handler calls. Minimal-feature build/tests
verify that auth works without Axum or a runtime dependency.

Favzetto is the first canary. Its old implementation passes a new 13-case credential
precedence/duplicate/spacing/fallback matrix plus a public-route check before
migration. Favzetto now adopts both identification and admin authorization in commit
`5b852cf099b8b2a8a93032bc6e084272286c9b42`, using shared revision
`0a629da7b5eb5aeeb0ed64aac2c5f96cd4d9717b`. Final all-target checks: 237 pass,
with the same two baseline catalog runtime-bridge failures. New checks cover
non-admin denial, malformed-text fallback and redacted AuthService Debug. Lifecycle
and logging subprocesses and authenticated WebSocket API cases pass. Configured
Clippy completes with existing warnings and no new auth findings. The auth source
and new test sections are formatted; unrelated formatting is preserved.

Favzetto master was rebased onto its canary commit, tested tree/ancestry verified,
and the temporary branch/worktree removed. Existing ignored frontend assets were
used only for compilation; browser/PDF/provider/container qualification was not
repeated. See Favzetto's `docs/step-08-auth.md` for the detailed compatibility record.
Its handlers use the synchronous flow, not the HTTP layer; the latter has shared
real-HTTP coverage.

The first rollout adds LelloStore, Crumbles and Simple AI, each assessed by a
GPT-6 Sol agent at medium reasoning in a separate service worktree. LelloStore
adopts JWT/user-registry verification and admin checks; Crumbles adopts HTTP
session/route checks and integration-daemon capability evaluation; Simple AI
adopts backend user authentication and admin checks. Provider verification,
resource policies and transaction authority remain application-owned. The shared
API required no change. Per-service scope, test evidence, retained protocol checks
and integration records are in the [migration tracker](migration-status.md#steps-0809-first-auth-rollout--2026-09-23)
and each consumer's `docs/step-08-auth.md`.

The second round adds Pezzottify's session/named-route flows and all three
Meteonesto services. Simple Agents' caller/browser and transactional admin/session
flows are also tested and integrated into main, with concurrent display-name/UI
edits preserved. Combined verification exposes a stale identity-JSON assertion
in that uncommitted work; the auth-only suite passed all 139 tests. No shared API
changes were needed.
The [second-round evidence](migration-status.md#steps-0809-second-auth-rollout--2026-09-23)
records tests, retained application policy, branch integration and cleanup limits.
The third round adds Lello Auth's optional-session, token and admin/resource
flows, Fausto's HTTP/WebSocket and admin/write flows, and Peerlo's bearer/Torznab
credential gates. Their application protocols and policies remain intact, with no
shared API extension. See the [third-round evidence](migration-status.md#steps-0809-third-auth-rollout--2026-09-23)
for scope, tests, lint qualifications and integration/cleanup records.

The fourth round adds Observo's IP/key/JWT route access, Pezzottflix's HTTP and
WebSocket sessions and named permissions, and SCT's token/session authentication
and transactional administrator/root-capability decisions. Each retains its
existing credential precedence, error mapping and application authority. No
shared API extension was needed. See the [fourth-round evidence](migration-status.md#steps-0809-fourth-auth-rollout--2026-09-23)
for verification, integration and cleanup.

The final round adds Androidoscopy's v2 controller and LAN authorization checks,
Paranza's runner/PCM gates, and Quentin Torrentino's API access flow. Pezzottify
Downloader is N/A: its public TCP routes have no caller gate, the child API uses
Unix-socket filesystem permissions, and Spotify credentials belong to outbound
provider sessions. Application policy and cryptographic protocols remain local.
No shared API extension was needed. The [final-round evidence](migration-status.md#steps-0809-final-auth-rollout--2026-09-23)
records test scopes, existing lint debt, applicability and integration/cleanup.

Combined auth rollout is complete locally: **16 Done, 1 N/A, 0 Pending**.
These statuses cover the documented production scopes; they do not imply that
every application-specific credential protocol or permission rule moved into
the shared library, or that consumer Axum removal is complete. Step 10 rate
limiting is next. Nothing was pushed or deployed.
