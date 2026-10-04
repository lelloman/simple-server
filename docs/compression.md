# Streaming response compression

Enable the opt-in `compression` feature. Installing the layer remains explicit:

```rust
use simple_server::web::{Router, compression::{CompressionLayer, CompressionQuality}};
let app: Router = Router::new().layer(
    CompressionLayer::new().quality(CompressionQuality::Fastest).min_size(256)
);
```

This module offers gzip only, even when Cargo feature unification enables other
codecs in its internal backend. It negotiates Accept-Encoding with quality values
and sets Content-Encoding/Vary. Content-Length is removed on compressed bodies.
Unaccepted encodings use identity; the layer does not generate a 406 or infer an
application policy when identity is disallowed. A feature alone installs nothing.

The default policy compresses known bodies of at least 32 bytes and unknown-size
streams. It excludes image/ prefixes (except image/svg+xml), application/grpc
prefixes (including grpc-web), and text/event-stream. Size uses the body hint or
Content-Length when available. Existing Content-Encoding or Content-Range always
prevents recompression. The SSE exclusion prevents buffering event streams behind
the compressor; other unknown-size streams are eligible and remain lazy.

`gzip(false)` disables gzip. `quality` selects Default/Fastest/Best without exposing
backend enums. `min_size(u64)` restores the default MIME exclusions with a new
threshold. `compress_when` replaces the MIME/size predicate with a thread-safe
closure taking status, HTTP version, headers and extensions; it cannot inspect
the body. Setters replace earlier policy, so calling min_size after a custom
predicate restores the default MIME policy. Range/encoded guards still apply.

CompressionService returns the shared owned Body. No Axum/Tower HTTP
compression types appear in public signatures. Readiness, service errors and body
errors are retained. Bodies are streamed with backpressure; response headers do
not require polling/buffering the payload. Dropping a response body or pending
response future releases its inner resources. Compression does not promise flush
boundaries per application chunk, preserve wire Content-Length or create a
filesystem/precompressed-file policy. Trailers follow the backend's streaming
codec behavior; no new trailer-preservation guarantee is introduced.

Applications own placement relative to CORS, tracing, authorization and static
services. Compression offers no route-specific policy automatically and changes
neither session handling nor authorization. Avoid compressing secrets together
with attacker-controlled data when that matters to the application's threat model;
use route placement or a custom predicate to exclude those responses.

Pezzottflix uses default gzip policy around its combined API/health/frontend
router, in the same position as its prior Tower HTTP 0.5 layer. Differential tests
compare 1,280 old/new cases (MIME, size, negotiation, encoded/range responses and
Vary). Status, non-Vary headers and gzip body bytes match. Vary token sets match;
the newer backend avoids duplicate Accept-Encoding tokens that the old backend
could append. This harmless header normalization is intentional. gRPC prefix
exclusion is explicitly configured to retain the older predicate's behavior.
Tests also cover decoding/quality, disable/threshold/custom policies, chunks,
errors, readiness, cancellation, owned routing/HEAD and actual loopback HTTP.
