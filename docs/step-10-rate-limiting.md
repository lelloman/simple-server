# Step 10: optional rate limiting and admission

The `rate-limit` feature exports `simple_server::rate_limit`. Public APIs use
standard Rust, `http`, `http-body` and Tower types. They do not expose Axum or
require Tokio, a database, an authentication provider or a background worker.
This is local implementation, not deployment or completion of consumer Axum removal.

## 10a — Budgets and storage

- `Quota::replenishing(interval, burst)` defines the time to restore **one** unit
  and an independent burst capacity. The integer-duration GCRA implementation
  matches governor 0.8 and 0.10 in differential tests. Governor is a test-only
  dependency. There is no floating-point refill drift or implicit rounding of
  application configuration: callers supply the exact interval they require.
- `Quota::fixed_window(window, limit)` starts its window on first admission and
  resets at elapsed time **greater than or equal to** the window. This differs
  from calendar-aligned and sliding-window quotas. Do not silently substitute it
  for either, or for legacy strictly-greater-than boundaries.
- `Budget` is caller-owned state. `check_at(now, cost)` consumes a positive cost
  atomically under the caller's exclusive access. Rejection does not consume;
  cost above capacity is an explicit error. Backward clock readings clamp to
  the last observation. Unrepresentable time arithmetic returns `ClockRange`.
- `KeyedLimiter<K>` supplies shared process-local synchronization and storage.
  Global limits use `()`, and composite keys can include route/user/device/tier.
  Clones share one budget store and clock. `Clock` allows deterministic tests;
  the default clock measures elapsed monotonic time from construction.
- `StoreConfig::bounded(capacity, idle)` rejects unseen keys when full, unless
  `Overflow::SharedBucket` explicitly sends them to one additional shared bucket.
  The overflow bucket retains its state when map capacity becomes available.
  It is not a distributed/global quota across multiple processes or limiter
  instances. Bounded entry count does not bound key length: use bounded or hashed
  identities for untrusted input.
- `prune()` removes only idle, fully replenished entries with no active permit.
  Admission also tries this cleanup when an unseen key meets a full bounded map.
  No active restriction is evicted to make space. Capacity rejection is distinct
  from quota rejection. `stats()` exposes aggregate counts, never identities.
  Applications can schedule explicit maintenance using their existing lifecycle.
- `StoreConfig::unbounded()` is an explicit compatibility option, with no
  automatic cleanup. It is not the default. It permits a behavior-preserving
  migration of existing unbounded stores; adopting bounds is a separate policy
  decision. Applications may continue using their own bounded/durable stores
  with `Budget` and the policy callbacks instead.

## 10b — Policies, outcomes and concurrency

`Policy<C,E>` and `AsyncPolicy<C,E>` evaluate consuming callbacks in declaration
order. A successful evaluation yields `Admission`, which owns optional guards.
A later denial or cancelled future drops prior guards; **already consumed rate
units and committed backend effects are not refunded**. Use a single callback
with the application's transaction when several durable checks must be atomic.
Errors remain application-defined, so unavailable storage is not confused with
quota exhaustion. No retries, timeouts, fail-open behavior or workers are added.

`KeyedLimiter::acquire(key, cost, concurrency)` checks the concurrency cap before
charging rate, under the same mutex. A rejection does not take a slot. Hold the
returned admission through the operation; dropping it releases concurrency only.
`check` is the convenience operation for rate-only admission. The concurrency
limit is selected explicitly for each call; callers sharing a key must apply
consistent policy. Multiple keys/limiters are sequential unless the application
provides its own atomic callback. There are no queues or implicit waiting.

`FailureCounter` separates `check_at` from `record_failure_at` and `reset`.
Preflight checks do not charge attempts. Counters can use a failure window or
accumulate until reset. Reaching the threshold starts a cooldown; failures while
blocked do not extend it. Expired cooldown resets the counter. Successful login
resets only the dimensions explicitly selected by the service. Key normalization,
IP/account association, alerting, audit and storage stay application-owned.

Validation cases model Crumbles' success-reset scope, Lello Auth's shared overflow
bucket, and Meteonesto's rate/concurrency coupling. These are contract tests,
**not consumer migrations**. Their precise policies, multi-dimensional storage
and durable authority still require migration review. Crumbles' durable dispatch
limits and Pezzottify's database-backed download/report quotas are not replaced
with an in-memory limiter. Remaining unsupported behavior stays Pending/Partial,
not N/A.

## 10c — HTTP adaptation

`RateLimitLayer` evaluates an `AsyncPolicy<http::request::Parts,E>`. Applications
choose keys, exemptions and route placement, and render errors from `E` plus the
original request parts. It never reads or buffers a request body, trusts proxy
headers, installs an identity, exempts OPTIONS, or moves auth/CSRF middleware.
Identity can come from the auth module or any existing request extension.

The default `rejection_response` uses 429 for rate/cooldown/concurrency, 503 for
store/time capacity, and 422 for cost exceeding a bucket's capacity. A known retry
duration becomes `Retry-After`, rounded upward with a minimum of one second.
Unknown duration omits it. Custom renderers can preserve legacy rounding, status,
headers, body, metrics and error codes. No rate-limit headers are added on success.

The service calls the same inner instance whose readiness was polled. It forwards
request parts and body unchanged. `RateLimitBody` streams response frames, trailers
and size hints, holding admission guards through EOF, body error or drop. Empty
bodies release immediately. Inner service errors and cancellation release guards.
A WebSocket guard covers the HTTP upgrade only; the application must explicitly
hold its own admission if it limits the lifetime of the upgraded connection.

## Example

```rust
use std::{num::{NonZeroU32, NonZeroUsize}, time::Duration};
use simple_server::rate_limit::{KeyedLimiter, Quota, StoreConfig};

let quota = Quota::replenishing(Duration::from_millis(100),
                                NonZeroU32::new(20).unwrap()).unwrap();
let limiter = KeyedLimiter::new(quota,
    StoreConfig::bounded(NonZeroUsize::new(10_000).unwrap(), Duration::from_secs(300)));
limiter.check("application-selected-identity", NonZeroU32::new(1).unwrap()).unwrap();
```

## Verification and rollout

Baseline shared suite: 145 tests/doctests pass outside the socket-restricted
sandbox. New tests cover weighted refill/burst/fixed-window boundaries, invalid
configuration, clock overflow/backward samples, both existing governor versions,
concurrent charging, memory capacity and expiry with active restrictions,
outcome/reset scope, ordered charges, asynchronous cancellation, streaming/error/
EOF/drop guard release, inner readiness, untouched bodies, explicit OPTIONS policy,
backend errors and real HTTP public/protected/key isolation and Retry-After.

Minimal-feature tests verify operation without Axum; the production dependency
tree contains only HTTP/Tower/body primitives, not governor or Tokio. Full-suite,
strict Clippy, formatting and canary evidence are recorded in the
[migration tracker](migration-status.md#step-10-rate-limiting--2026-09-24).
