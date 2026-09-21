# Step 04b: response header policies

## Contract

The optional `response-headers` feature provides framework-independent operations
in `simple_server::response_headers`. It uses HTTP header primitives and does not
require Axum, Tokio, lifecycle or observability. With default features disabled,
its normal dependency graph contains only `http`, `bytes` and `itoa`.

- `insert_if_absent(headers, name, value)` installs a default only when the field
  is absent. All existing values survive, including empty/repeated values and
  sensitive flags.
- `replace(headers, name, value)` deliberately replaces every value of that
  field with one supplied value. All other fields remain untouched.
- `merge_vary(headers, required)` accepts one HTTP field name or `*`. It checks
  every existing Vary line and comma-separated token, ignoring ASCII case and
  surrounding whitespace. An existing wildcard or matching name is a no-op.
  Otherwise it appends one separate field line, preserving the supplied spelling
  and every existing value byte, duplicate, malformed value and sensitive flag.
  Invalid requested names return `InvalidHeaderName` without any mutation.

Repeated Vary field lines represent a combined list. Consumers must inspect all
values rather than assume the first field contains every name. The helper does
not collapse existing fields, discard opaque bytes, or clean up old duplicates.
This is intentional preservation, even if a previous local helper normalized
valid fields or silently discarded values it could not decode.

Applications supply all header values and choose route/layer placement. No-store
is expressed explicitly with Cache-Control and `no-store`; it can be a default
or an unconditional replacement. No headers or middleware are installed by
merely enabling the feature. No cache eligibility, CSP generation, ETags, auth,
range semantics, trusted-proxy policy, or error schema moves into this module.

Only the supplied header map is mutable. Response status, HTTP version,
extensions, and body identity are outside the API. Bodies are not polled,
buffered, mapped or reconstructed; data, trailers, stream errors, SSE and upgrade
handling remain owned by the existing response path. Applications can call the
helpers from their existing response middleware without changing its position.

The module exposes no Axum types or trait bounds. Framework-independent `http`
primitives are re-exported for callers; current consumer router middleware still
uses the transitional Axum API and is not the final routing abstraction.

## Verification and adoption

Shared contract tests cover absent/present/empty/repeated values, Set-Cookie and
sensitive flag preservation, Vary case/whitespace/duplicate/wildcard handling,
invalid input without mutation, and unchanged status/version/extensions with
lazy data/trailer/error frames. The minimal feature build and full feature matrix
are included in `scripts/check`, alongside formatting, strict Clippy and rustdoc.

Pezzottify and Simple Agents are the initial canaries. Before/after checks must
cover default versus overwrite behavior, existing Cache-Control, Vary lists,
early auth/limit errors, HEAD, partial/media responses and SSE. Real production
usage and integration evidence are recorded in the [adoption trackers](migration-status.md).
Other consumers require assessment before rollout. 04c remains planned.
