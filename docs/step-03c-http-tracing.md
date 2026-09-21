# Step 03c: HTTP tracing

The opt-in `http-tracing` feature exposes `http_tracing::trace(request, callback)`.
It requires HTTP and `tracing`, but neither logging initialization, correlation,
lifecycle, nor a Tokio runtime. The application owns subscriber configuration.
No existing default feature or router behavior changes merely by updating the library.

## Contract

`trace` creates an INFO `http.request` span with method, route and eventual status.
It instruments callback construction, callback polling and response-body polling.
Standard HTTP methods are recorded verbatim; extension methods become `OTHER`.
Routes come exclusively from Axum's `MatchedPath`, capped at 256 bytes; missing
matches use `<unmatched>` and oversized templates use `<route-too-long>`.
Raw paths, URIs, query strings, headers, credentials, bodies and error text are
never automatically recorded. Application-emitted events retain application policy.

When the optional correlation feature is also enabled, a surrounding shared
validated-ID scope supplies `request_id`. Opaque legacy IDs are deliberately not
automatically logged: applications may attach their own approved fields to a
parent span. No new IDs are generated here. The span retains its ID while the
body is polled after the task-local correlation scope ends; this does not extend
the task-local scope into streaming or spawned work.

Events use the `simple_server::http_tracing` target:

| Event | Level | Fields / meaning |
| --- | --- | --- |
| `http.response_headers` | ERROR for 5xx, DEBUG otherwise | `status`, `header_latency_ms`: elapsed until the callback returns a response |
| `http.finished` | INFO | `outcome=complete`, `phase=body`, `duration_ms`: body end observed by the server |
| `http.finished` | INFO | `outcome=upgraded`, `phase=body`, `duration_ms`: HTTP 101 or successful CONNECT handoff |
| `http.finished` | ERROR | `outcome=error`, `phase=body`, `duration_ms`: body polling returned an error |
| `http.finished` | WARN | `outcome=cancelled`, `phase=headers` or `body`, `duration_ms`: instrumented future/body dropped before completion |

Durations are monotonic milliseconds from entry into the first poll of `trace`,
not queue time before middleware execution. Header timing measures response
creation, not flushing bytes to the network. Total duration includes downstream
body polling/backpressure; it is not evidence of client receipt. Cancellation
includes application replacement/dropping and panic unwinding, and must not be
interpreted as proof of client disconnect. An entirely unpolled future emits
nothing. Exactly one terminal event is emitted for an observed lifecycle (subject
to subscriber filtering); a body error is not followed by cancellation.
HEAD, 204, 304 and non-upgrade informational responses complete at headers:
HTTP deliberately omits their wire bodies, so dropping such a body is not a
cancellation. Their bodies and content-length handling remain untouched.
An already-ended/empty body also completes at response creation. Otherwise, completion
occurs at EOF or when the inner body reports its final frame. Frames, trailers,
errors, size hints and end-of-stream signaling are passed through without
buffering, pre-polling or draining. Upgrade bodies are returned unchanged;
WebSocket/CONNECT session lifetime requires separate application instrumentation.

## Placement and adoption

Use `axum::middleware::from_fn` for a normal router, or call `trace` inside an
existing middleware. Install at a point where matched route templates are visible.
Keep correlation outside tracing if the ID should be recorded. Include any
response normalization inside the callback so the observed body is the final
body returned; replacing a traced body outside the wrapper records cancellation
of that body. Place tracing around intended auth/rejection paths; middleware that
short-circuits outside it is not observed. Remove equivalent existing request
logging when adopting, avoiding duplicate request events.

Check application logging filters as well as router wiring. If the default
filter enables only the application's target, include the new shared target so
adoption is observable. Explicit user filters and runtime request-logging
switches remain authoritative. Keep opt-in redacted header/body diagnostics,
domain events, audits and metric boundaries application-owned; replacing
request lifecycle logging must not silently disable those behaviors.

This API is infallible like Axum middleware: service failures must become HTTP
responses before returning. Body outcomes are independent of HTTP status: a 500
response can have a completely transmitted body. It does not inspect domain errors,
change response status/headers/extensions, register global state, trace spawned
tasks automatically, or implement metrics, exporters, audit events or sampling.
Filters and subscriber span-close events remain application-owned.

## Application-owned event policy

`trace_with_observer(request, observer, callback)` retains the same safe request
span, response timing, body pass-through and lifecycle tracking, but replaces the
automatic events with an application-owned `Observer`. This supports an existing
structured stderr sink or custom response fields/severity without installing a
subscriber or duplicating shared HTTP events. `trace` remains equivalent to using
`TracingObserver`; its default behavior is unchanged.

`Observer: Send + 'static` has two default no-op methods:

- `on_response(&mut self, &Span, &Response, Duration)`: called once when the
  callback returns, inside the span. The duration is captured before the observer
  runs. Headers and extensions are read-only; applications may use an approved
  domain error code or content length. It does not expose body contents.
- `on_finish(&mut self, &Span, Outcome, Phase, Duration)`: called once for the
  terminal outcome, including future/body drops. `Outcome` is `Complete`,
  `Upgraded`, `Error` or `Cancelled`; `Phase` is `Headers` or `Body`. Their
  `as_str()` values match the default event fields. The callback may run after
  the correlation task-local scope has ended; capture needed approved fields
  when constructing the observer.

An observer can delegate either callback to `TracingObserver` to retain just
that part of the standard event policy. Application event schemas, privacy,
labels, metric boundaries and output destinations remain application-owned.
Callbacks run even without an enabled subscriber; default tracing events remain
subject to filtering. They execute synchronously, including in Drop, must not
panic or wait on asynchronous work, and should return promptly. These hooks do
not authorize adding fields that the application would otherwise redact.

## Qualification

Contract tests cover safe routes and fallback labels, statuses and method
normalization, correlation and interleaving, callback/body span context,
stream timing, trailers, errors, cancellation, empty bodies and upgrade handoff.
The feature is checked alone, with correlation, and in the full feature matrix.
The Crumbles canary and its scope/verification are recorded in
[migration-status.md](migration-status.md).
