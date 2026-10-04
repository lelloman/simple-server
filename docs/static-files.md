# Static files

The opt-in `static-files` feature provides owned streaming HTTP services under
`simple_server::web::static_files`. Neither Axum nor Tower HTTP types appear in
these services' public constructors or responses. It enables the web router and
filesystem backend; it installs no middleware globally.

```rust
use simple_server::web::{Router, static_files::{StaticDir, StaticFile}};

let app: Router = Router::new()
    .nest_service("/assets", StaticDir::new("assets"))
    .nest_service("/favicon.ico", StaticFile::new("frontend/favicon.ico"))
    .fallback_service(
        StaticDir::new("frontend").fallback_file("frontend/index.html")
    );
```

`StaticDir` enables directory `index.html` lookup by default; disable it with
`append_index_html_on_directories(false)`. Directory requests without a trailing
slash redirect and preserve their query. Nesting strips the mount prefix before
filesystem lookup; redirect locations follow the underlying backend contract,
so a nested directory redirect may omit the mount prefix. Test redirect routing
when serving an indexed directory below a prefix.

Without a fallback, missing paths return 404. `fallback_file(path)` preserves the
configured file's status, normally 200, for SPA navigation. It also applies to
missing assets and invalid paths, matching Pezzottify's existing contract; it does
not infer whether a request is an asset or HTML navigation. `not_found_file(path)`
serves a custom error page with status 404. Calling either again replaces the
previous fallback. Missing fallback files still return 404.

GET and HEAD are supported. Other methods return 405 and do not invoke the
fallback. Single byte ranges, Last-Modified/If-Modified-Since and
If-Unmodified-Since behavior are preserved. MIME type comes from the extension;
responses stream with backpressure instead of buffering the entire file. Initial
filesystem failures become HTTP responses (missing/denied paths are 404, other
I/O errors are 500); later read failures are body errors.

`precompressed_gzip()` and `precompressed_br()` explicitly enable negotiation of
existing `.gz`/`.br` siblings on directory or single-file services. They do not
compress responses or change the separate fallback file's configuration. No
Cache-Control, ETag generation, security headers or dynamic compression policy is
added. Apply application middleware explicitly where needed.

Parent/backslash path segments do not expose files outside the root. Symlinks are
followed: these services are not a filesystem sandbox, and the application must
control its root and symlinks. An explicit fallback can return its configured
file even for an invalid path. Relative filesystem paths resolve against the
process working directory; no startup existence check is required.

Pezzottify can preserve its current frontend behavior with `StaticDir::new(root)
.fallback_file(root.join("index.html"))`. Tests cover both real TCP requests and
42 differential request cases against its prior Tower HTTP configuration, along
with standalone directory/file behavior, range/conditional responses, missing
files, methods, percent-encoded traversal and precompressed variants.
