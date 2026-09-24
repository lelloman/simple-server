# Step 10: optional rate limiting and admission

The `rate-limit` feature exports `simple_server::rate_limit`. Public APIs use
standard Rust, `http`, `http-body` and Tower types. They do not expose Axum or
require Tokio, a database, an authentication provider or a background worker.
The separate opt-in `rate-limit-async` feature adds a Tokio delayed-release
waiting adapter; the base `rate-limit` feature remains runtime-independent.
This is local implementation, not deployment or completion of consumer Axum removal.

## 10a — Budgets and storage

- `Quota::replenishing(interval, burst)` defines the time to restore **one** unit
  and an independent burst capacity. The integer-duration GCRA implementation
  matches governor 0.8 and 0.10 in differential tests. Governor is a test-only
  dependency. There is no floating-point refill drift or implicit rounding of
  application configuration: callers supply the exact interval they require.
- `Quota::replenishing_with_policy(interval, burst, RefillPolicy::ExtraIdleCredit)`
  preserves governor 0.6's initial one-interval debt and full-burst tolerance.
  Fresh budgets admit the configured burst; a retained budget can admit one
  extra unit after a long idle interval. Single-check cost is still capped at
  the configured burst. This option is explicit; `replenishing` stays strict.
  The legacy policy is tested against governor 0.6 across idle and weighted checks.
- `Quota::fixed_window(window, limit)` starts its window on first admission and
  resets at elapsed time **greater than or equal to** the window. This differs
  from calendar-aligned and sliding-window quotas. Do not silently substitute it
  for either, or for legacy strictly-greater-than boundaries.
- `TokenBucket::per_minute(rate, burst)` is a caller-owned compatibility primitive
  for existing floating-point refill policies. It deliberately preserves
  `elapsed_seconds * rate / 60.0` arithmetic and whole-second ceiling retry times.
  `refill_at(now)` lets a service update allowance before checking concurrency;
  `check_at(now, cost)` charges after that gate, under the service's lock. Storage,
  heterogeneous quota selection, cleanup and permit ownership remain local.
  Timestamps normally clamp backward samples. Explicit
  `with_clock_regression(ClockRegression::Reanchor)` instead observes zero elapsed
  and stores the older timestamp, preserving services that sample time before
  acquiring a mutex and can receive observations out of order. Later samples
  refill from that older anchor. This is a deliberate compatibility policy.
  It does not replace the integer GCRA default or silently change its semantics.
- `PerSecondTokenBucket::new(rate, burst, now)` preserves existing unit-cost
  `min(tokens + elapsed_seconds * rate, burst)` accounting. Unlike the per-minute
  primitive, it deliberately accepts zero capacity and unvalidated `f64` rates,
  including negative/nonfinite values, for compatibility with existing parsers.
  `check_at(now)` preserves saturating elapsed time and reanchors to the latest
  observation; `last_observed()` supports caller-owned idle cleanup. Retry seconds
  use the legacy ceiling/saturating `u64` cast for positive rates and 60 seconds
  otherwise; zero and `u64::MAX` are possible. Render the duration directly to
  retain that response contract, since the generic renderer applies a minimum.
  New validated configurations should normally use `Quota`. Tests compare 72,000
  old-algorithm observations, including negative/nonfinite rates and zero capacity.
- `Budget` is caller-owned state. `check_at(now, cost)` consumes a positive cost
  atomically under the caller's exclusive access. Rejection does not consume;
  cost above capacity is an explicit error. Backward clock readings clamp to
  the last observation by default. Explicit
  `Budget::with_clock_regression(ClockRegression::Reanchor)` preserves raw caller
  samples, including fixed-window retry calculations before the window start.
  Unrepresentable time arithmetic returns `ClockRange`.
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

### Durable rolling windows and language bridges

`evaluate_rolling_windows` evaluates caller-configured `RollingWindow` quotas
against a `RollingWindowStore`. It owns per-window clock sampling, signed
microsecond cutoff subtraction, saturating remaining counts and the minimum
budget. An empty configuration uses the caller's explicit fallback budget.
The store counts applicable events strictly after each cutoff, with no upper
bound, and propagates clock/database failures. All windows are read, including
ones following an exhausted quota, so diagnostics stay complete and errors
cannot silently become quota results. Zero limits and durations are supported.

`RollingWindow::cutoff_at` and `remaining` also expose the two phases separately
for asynchronous database queries inside an existing transaction. They use the
same cutoff and saturation rules as the synchronous evaluator.

This is a read-only policy: the service owns its database, transaction, event
selection, locking and success accounting. Evaluation does not reserve quota;
concurrent callers needing atomic admission must supply the transaction/lock.
Signed timestamps allow windows crossing the Unix epoch; underflow is an explicit
error. No database, IPC, JNI or language runtime dependency is added to the crate.
Consumers can adapt the same API through a bounded language bridge while keeping
their authoritative storage in the calling process.

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
accumulate until reset. By default, reaching the threshold starts a cooldown;
failures while blocked do not extend it and expired cooldown resets the counter.
`with_policy` makes two choices explicit: `BlockedFailures::Count` records outcomes
already in flight during cooldown, retriggering the block at each threshold;
`CooldownExpiry::PreserveWindow` retains the independent failure window/count after
cooldown expiry. With counted outcomes, successful recording means no *new* block
was triggered, not that preflight would allow a request. Always use `check_at` for
preflight. `FailureCounter::with_clock_regression(ClockRegression::Reanchor)`
can also preserve out-of-order observations sampled before a caller's mutex.
With `PreserveWindow`, preflight leaves the failure window and stored deadline
intact: an older sample can still observe a block after a newer one saw it expire.
Window maintenance occurs when recording outcomes. This preserves services with
independent outcome/window policies. Successful login
resets only the dimensions explicitly selected by the service. Key normalization,
IP/account association, alerting, audit and storage stay application-owned.

`FailureWindow` separates non-consuming preflight from outcome recording, with a
window anchored at the first failure rather than the threshold. Recording an
outcome resets an expired window; preflight does not change it. Zero threshold
blocks after the first failure, and zero duration never blocks. `is_expired_at`
and `reset` allow caller-owned cleanup and success-reset scope.

`FailureLatch` accumulates failures until a threshold, then keeps a single
cooldown deadline until explicitly reset or discarded. In-flight failures never
extend it, including after expiry; preflight only reports the current deadline.
`is_expired_at` lets application preflight own cleanup. Zero duration latches
without a blocking period; zero legacy thresholds can explicitly select one.
Unrepresentable deadlines return `ClockRange` without consuming the threshold
outcome. These policies preserve Pezzottflix's distinct IP and email controls;
36,000 old-model sequences cover outcome/preflight/reset ordering and zero values.

Validation cases model Crumbles' success-reset scope, Lello Auth's shared overflow
bucket, and Meteonesto's rate/concurrency coupling. These are contract tests,
**not consumer migrations**. Their precise policies, multi-dimensional storage
and durable authority still require migration review. Crumbles' durable dispatch
limits and Pezzottify's database-backed download/report quotas are not replaced
with an in-memory limiter. Remaining unsupported behavior stays Pending/Partial,
not N/A.

### Anchored, calendar and composite snapshot policies

`WindowCounters<N>` keeps category counts under one caller-provided monotonic
anchor. Choose `WindowBoundary::AtOrAfter` or `After` explicitly. Refresh and
non-consuming preflight can run on idle ticks; `record` accounts for actual
attempts later, while `admit_at` combines checking and charging. Zero limits
and durations are supported. Observing counts/anchor does not refresh them.
Rejections expose exact remaining time; the application owns wire rounding.
Counters saturate at their integer maximum instead of wrapping.

`CalendarCounter<D>` uses equality of application-provided period keys, including
backward calendar changes. Preflight never resets stored state; recording updates
period/count. `CalendarGate<D>` adds a persisted minimum gap checked before the
calendar quota, with saturating backward elapsed time. Snapshot getters and
`from_parts` preserve durable storage ownership; dates, time zones and
serialization remain local. Recording is independent of preflight and saturates
at `u32::MAX` for pathological overflows instead of wrapping/panicking.

`LimitCheck::below` evaluates signed current-count limits; `projected` evaluates
usize resource usage plus cost and reserved space with saturating arithmetic.
`evaluate_limits` returns the first failed key in declaration order, preserving
quota-before-capacity error precedence. All of these policies are caller-owned,
read-only or explicit-record operations: no database, transaction, atomic commit,
queue, identity or persistence is installed implicitly.

`PollingGate` reconstructs persisted signed-epoch polling state with separate
check/record or combined admit operations. It preserves raw backward time and
zero/negative stored intervals; wide arithmetic avoids timestamp subtraction
overflow. Denial does not advance the timestamp or escalate the interval. The
service retains expiration, storage, protocol status and transaction behavior.

With opt-in `rate-limit-async`, `DelayedReleaseLimiter` provides FIFO waiting for
a semaphore slot followed by an independent release timer. A burst of N is
immediate; each slot returns after its configured hold time. This is deliberately
not an N-per-second budget or request-lifetime concurrency guard. Each admission
spawns one Tokio timer, beginning its delay when polled, matching existing
spawn/sleep/drop implementations. Waiting cancellation takes no permit; later
request cancellation does not refund an admitted slot. Zero capacity waits until
closed. The application owns transport, metrics and shutdown policy. The base
rate-limit module still adds no Tokio dependency.

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


Initial canaries are integrated: Simple AI uses shared backend budgets; Pezzottify
uses shared budgets and HTTP admission throughout its previously Governor-backed
routes. Pezzottify remains Partial because its MCP categories share a legacy
window anchor/boundary and allow zero limits; the independent fixed-window
primitive cannot preserve those semantics by direct substitution. Both canaries
explicitly retain unbounded stores to avoid introducing an unapproved capacity
policy. See the tracker for revisions, tests and remaining scope.
