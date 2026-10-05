# Unix HTTP transport

Enable `unix-http` on Unix platforms to serve an already-bound
`simple_server::net::UnixListener` with `web::unix::serve(listener, router, shutdown)`.
The transport uses the same owned `Router`, extractors, response/body and
WebSocket APIs as TCP serving. The application owns socket creation, stale-file
removal, permissions, signal installation, shutdown deadline and final cleanup.
A requested shutdown stops accepting and drains active HTTP requests. Upgraded
WebSocket sessions retain their application-owned shutdown policy.

`web::unix::UnixClient::request(socket_path, request)` forwards an owned HTTP
request and returns an owned streaming response. Client clones share a pool.
Bodies are not collected, preserving backpressure and upstream body failures.
Methods, path/query, headers and explicit HTTP version are retained. The socket
path selects the transport independently of the original URI authority; the
client rewrites that URI internally. Callers decide whether to forward Host,
authorization, request-ID or other headers and own timeout/retry policy.

The HTTP/1 client accepts nonempty UTF-8 socket paths and origin-form paths
beginning with `/`. Non-UTF-8 paths and unsupported request targets fail with
`UnixHttpErrorKind::InvalidRequest`; socket/HTTP failures return `Transport`.
Both preserve an inspectable error source. No backend transport types appear in
the public API. The module is available only with `cfg(unix)`; enabling the
feature does not provide a Windows named-pipe implementation.

The correlation feature also provides `HeaderRequestId::uuid_v4()` for services
whose request-ID protocol requires lowercase, hyphenated UUID v4 values. This
selects only the generator: raw caller-header acceptance, duplicate handling and
application extension overrides remain caller policy.
