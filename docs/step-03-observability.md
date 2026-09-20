# Step 03: observability

Status: 03a is locally adopted by 15 products; two products are N/A.
Module [03b](step-03b-correlation.md) is implemented and piloted in Crumbles;
the remaining services await individual assessment. Module 03c remains planned. See the
[migration status](migration-status.md) for verification and applicability.

Step 03 contains three independently adoptable modules. Each has its own
contract, tests, and consumer migration evidence.

| Module | Responsibility | Dependency boundary |
| --- | --- | --- |
| 03a — Logging setup | Explicitly install text, pretty, compact, or JSON logging, with optional filter reload and span events | Usable without HTTP, lifecycle, or a Tokio runtime |
| 03b — Request correlation | Generate or validate a request ID, expose it to application code, and propagate it in responses | HTTP integration without requiring the shared logging initializer |
| 03c — HTTP tracing | Request spans, response status, and precisely defined timing | Works with an application-owned tracing subscriber; integrates with correlation when enabled |

Applications retain configuration loading, startup order, domain events, audit
semantics, and error-response formats. Adopting one module does not require
adopting the other two. Existing `simple-server` defaults remain unchanged.

## Sequence

1. Define and implement [03a: logging setup](step-03a-logging.md) in `simple-server`.
2. Pilot 03a in Favzetto and verify its configuration and logging behavior.
3. Use the pilot evidence to finalize the API before migrating other consumers.
4. Define 03b, implement it, and qualify existing correlation/error contracts.
5. Define 03c, implement it, and qualify ordinary, streaming, and upgraded HTTP.

Crumbles is a useful compatibility consumer for 03b: it already shares
`x-correlation-id` between tracing, response headers, and application errors.
SCT currently creates `x-request-id` in its error-response construction. These
contracts must be examined during each migration, not replaced implicitly.

## Constraints for 03b and 03c

The [03b contract](step-03b-correlation.md) defines its public API and boundaries.
The following requirements also guide the remaining 03c design.

- One request ID should agree across logs, response headers, and application
  error bodies. Applications provide the error-body integration.
- Header names are configurable. Generate fresh IDs by default; accepting
  validated caller IDs requires explicit configuration with bounded validation.
- Shared HTTP logs use route templates and bounded fallback labels, rather than
  raw URLs. Query strings, credentials, and request/response bodies are not
  automatically recorded.
- Timing distinguishes response-header latency from streamed-body completion.
  Body cancellation and errors need explicit outcomes; WebSocket session
  lifetime remains separately instrumented.
- Middleware placement must cover intended error paths without changing
  response bodies, buffering streams, or duplicating existing request logging.

Metrics, telemetry exporters/OpenTelemetry, health endpoints, and shared audit
event formats are outside Step 03's initial scope.
