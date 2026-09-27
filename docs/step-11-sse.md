# Owned server-sent events

The optional `sse` feature provides `web::sse`, enables `web`, and requires no
compatibility feature. Public types are owned by simple-server; backend storage
is private. Existing SSE behavior is reused internally to preserve clients'
framing, validation, keepalive and streaming contracts.

## Event production

`Event` is cloneable and supports builder calls in wire order:

- `data(text)`: text data, with carriage returns and newlines split into data
  fields exactly as the existing encoder. Unicode, whitespace and trailing
  delimiters are preserved; empty text retains existing encoder behavior.
- `json_data(value)`: compact serde JSON, returning owned `EventError` on
  serialization failure. It preserves Display and the underlying error source.
- `event(name)`, `id(value)`, `retry(duration)`: event name, reconnect cursor and
  reconnect-delay hint. Empty IDs reset a browser cursor. Retry is whole
  milliseconds, truncating sub-millisecond fractions.
- `comment(text)`: repeated comments are allowed.
- `into_data_writer()`: an owned `fmt::Write` implementation for incrementally
  formatted data; call `into_event()` before yielding it.

Validation preserves the established builder contract: event names and comments
reject CR/LF; IDs reject CR/LF/NUL. Repeated single-value fields panic; the first
nonempty data write reserves the data field (empty writes are no-ops). Comments
can be repeated. These are builder programmer errors, not HTTP rejections.
Applications should validate untrusted IDs/names before invoking builders.

## Response and lifecycle

`Sse::new(stream)` accepts a Send + static stream of `Result<Event, E>`, where
`E` converts to the standard shared boxed error. Unpin is not required. It
implements the shared `IntoResponse` with HTTP 200, `Content-Type:
text/event-stream` and `Cache-Control: no-cache`. Shared header/status response
tuples remain available for explicit application policies.

The adapter does not poll, collect or prefetch the stream during construction
or conversion. Body demand polls the stream once per event; each event is one
body frame. This preserves streaming backpressure and bounded consumer channels.
Stream errors propagate through the shared body without converting them into
application SSE events or changing a status already sent to the client.
Dropping the body drops the upstream stream/receiver and its captured resources.
Applications retain ownership of detached producer tasks, subscriptions, event
history, IDs, authorization and replay/Last-Event-ID handling.

There is no keepalive by default. `.keep_alive(KeepAlive::default())` sends an
empty comment `:\n\n` after 15 idle seconds. Configure `interval`, `text`, or a
complete `event` for a custom heartbeat. Normal events reset the idle timer;
ready data, errors and EOF take priority over a due heartbeat. A completed stream
is not kept alive. Timers start at response conversion and configured keepalive
requires a Tokio runtime with time enabled. No application task is spawned.

The shared HTTP tracing wrapper observes these bodies with its existing
completion/error/cancellation semantics. No subscriber is installed by SSE.
Proxy buffering, compression and timeout policies remain application-owned.

## Consumer migration

The inspected active endpoints are Pezzottify search results, SimpleAI admin
runner events, and simple-agents session events. All use existing backend SSE
builders and 15-second keepalive. Session events additionally use JSON and event
IDs for their existing reconnect cursor. Quentin Torrentino also has an SSE
boundary but remains intentionally skipped.

For an applicable consumer:

1. Enable `sse` in the simple-server dependency at a reviewed source revision.
2. Replace backend SSE imports with `simple_server::web::sse::{Sse, Event,
   KeepAlive}` (retain a local alias such as `SseEvent` if useful).
3. Return the shared SSE directly, or use shared `IntoResponse` to produce the
   existing owned response. Remove the compatibility response bridge.
4. Keep producer logic, auth, event ordering/schema, reconnect cursors and
   shutdown ownership unchanged; verify production adoption before removing
   `web-compat` if other protocols still use it.
5. Run meaningful before/after HTTP and streaming checks using the consumer
   migration workflow and update both trackers with evidence.

The library implementation alone does not migrate or mark these consumers Done.

## Verification

The protocol suite compares exact bytes and headers against the existing pinned
encoder, including Unicode/CR/LF, empty data/IDs, comments, JSON, retry bounds,
formatted data and default heartbeat. It checks invalid/duplicate fields and
owned serialization errors. Streaming tests cover no eager polls, per-frame
backpressure, errors, drop cleanup and non-Unpin streams. Paused-time tests verify
opt-in/default/custom keepalive, resets, ready-event priority and EOF/error
priority. A real shared HTTP router verifies authentication rejection, named JSON
with Last-Event-ID-based application cursors, response headers, delivery before
completion and body/producer release after disconnect.

Baseline all-feature suite: **285 passed**. Final all-feature suite: **297 passed**
(including 11 SSE tests and the new API doctest). Minimal `sse` suite: **10
passed**. Formatting, strict all-target/all-feature Clippy, the existing feature
matrix, no-default-feature check and warnings-denied all-feature rustdoc pass.
The final rustdoc gate exposed existing extraction/auth comment warnings; their
links and code formatting were corrected without changing behavior.

Implemented in isolated `feat/owned-sse` from `main` at `c7f1a7f`; local
integration follows the repository's worktree/rebase workflow. No consumers,
pushes or deployments are included.
