# simple-server

A small, modular Rust library for shared infrastructure across Axum services.
The first stage centralizes Axum versions through a transitional re-export.
The end goal is to completely abstract Axum away behind the library's own
interfaces, including routing, handlers, extractors, responses, and middleware.
The re-export is temporary and will be removed once all consumers are migrated;
Axum can remain an internal implementation detail. See the
[design and completion criteria](docs/design.md#end-goal-completely-abstract-axum-away).

**Status:** Axum dependency centralization and opt-in lifecycle, logging,
correlation, HTTP tracing, body limits, response-header helpers and CORS configuration are implemented. All 17 inventoried products have adopted lifecycle helpers locally
with documented application scopes. Crumbles and SCT include those migrations on local `master`. Publication to crates.io remains disabled.

Open the [HTML migration matrix](docs/migration-status.html) in a browser for adoption status
across all 17 planned consumer projects and the completed steps for each one.

All consumer migrations follow the [reusable worktree and verification workflow](docs/consumer-migration-workflow.md),
including applicability checks and updates to both adoption trackers.

## Axum centralization

`simple-server` pins Axum to **0.8.9**. Consumers replace their direct `axum`
dependency with a dependency on this repository at a reviewed Git revision,
using the public HTTPS mirror for builds outside the private forge:

```toml
[dependencies]
simple-server = { git = "https://github.com/lelloman/simple-server", rev = "<reviewed-commit>", features = ["ws", "multipart"] }
```

```rust
use simple_server::axum::{Router, routing::get};

let app: Router = Router::new().route("/health", get(|| async { "ok" }));
```

The default `http` feature enables Axum with its standard defaults. `ws`,
`multipart`, `macros`, and `http2` forward optional capabilities. With
`default-features = false`, Axum is absent unless an HTTP feature is selected.

The re-export intentionally exposes Axum during migration. By itself it does not
start a runtime, install signals, or change application startup or shutdown.
Companion crates such as `axum-extra` must resolve against the same Axum version;
check the resolved graph with `cargo tree -i axum` after each migration.

For coordinated development, a consumer may instead use a sibling path dependency
(for example, `path = "../../simple-server"` from a nested server crate). Such a
consumer must include the sibling in CI and Docker build contexts and record the
reviewed `simple-server` commit used by those builds. A path dependency's Cargo
lockfile does not pin the sibling's source revision.

Independent service lockfiles can select different transitive dependency
versions. Exact Axum uniformity requires the same pin across the adopted
`simple-server` revisions; an Axum pin change is an explicit migration.

## Initial scope

Lifecycle, logging, request correlation and HTTP tracing helpers are available now.

[Step 03: observability](docs/step-03-observability.md) is split into independently
adoptable logging setup (03a), request correlation (03b), and HTTP tracing (03c).
[03a: logging setup](docs/step-03a-logging.md) is implemented, with verified
adoption and compatibility exceptions in the [migration matrix](docs/migration-status.html).
[03b: request correlation](docs/step-03b-correlation.md) is implemented;
[03c: HTTP tracing](docs/step-03c-http-tracing.md) provides optional safe request
spans and response-body lifecycle events via `http-tracing`. Consumer adoption is
tracked separately for each module.

- Listener setup, shutdown signals, graceful shutdown, and shutdown deadlines.
- Structured logging, request IDs, and HTTP tracing.
- Health and readiness endpoint plumbing with application-provided checks.
- Optional rate limiting with application-defined keys and route placement.

Metrics, background-task supervision, database helpers, authentication, and
authorization are follow-up modules, shaped by real service integrations.

Applications own their routes, state, configuration loading, database setup,
authorization rules, background jobs, and domain logic. Capabilities should be
optional, configuration explicit, and dependency upgrades independently adopted.

See [the design outline](docs/design.md) for boundaries and the adoption plan.
The [Step 02 lifecycle contract](docs/step-02-lifecycle.md) describes the scope,
shutdown behavior, validation requirements, and remaining adoption work.

## CORS

Enable the optional `cors` feature for explicit `CorsConfig` policies. It works
without Axum or default features. No cross-origin permissions are enabled by
default; applications choose origins, credentials, headers and layer placement.
`build()` rejects invalid wildcard/credential combinations before serving. See
the [04c contract](docs/step-04c-cors.md).

## Response headers

Enable the optional `response-headers` feature for explicit `insert_if_absent`,
`replace`, and `merge_vary` operations. This module also works with default
features disabled and has no Axum dependency. Applications own cache policy,
header values and route placement; the helpers never access response bodies.
See the [04b contract](docs/step-04b-response-headers.md).

## Lifecycle

Enable `lifecycle` explicitly. Core coordination also works with
`default-features = false, features = ["lifecycle"]`; HTTP adapters require both
`http` and `lifecycle`. Applications retain their Tokio runtime and `main()`.

```rust,no_run
use std::{io, time::Duration};
use simple_server::{axum::{Router, routing::get}, http};
use simple_server::lifecycle::{Lifecycle, ShutdownOptions, Signals};

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signals = Signals::install()?;
    let mut lifecycle = Lifecycle::new(ShutdownOptions {
        grace_period: Duration::from_secs(30),
    });
    let listener = http::bind("127.0.0.1:3000").await?;
    let app = Router::new().route("/", get(|| async { "hello" }));
    lifecycle.service("http", http::serve(listener, app, lifecycle.shutdown()))?;
    lifecycle.run(signals.wait(), async { Ok::<_, io::Error>(()) }).await?;
    Ok(())
}
```

See the runnable [example](examples/lifecycle.rs). `Shutdown::request()` also
triggers shutdown, and callers can supply their own trigger future instead of
OS signals. Standalone `Shutdown` and `http::serve` do not require the coordinator.
The HTTP adapter also accepts make-services carrying connection information.

The coordinator drains registered futures concurrently, then polls application
cleanup with the remaining budget. Service failures, early service exits, and
timeouts return `LifecycleError` with a detailed report. Zero permits no draining
wait. Panics propagate; dropping the coordinator does not run cleanup.

Deadlines bound cooperative waiting. They do not forcibly stop detached tasks,
blocking work, or upgraded WebSocket connections. Those require application-owned
cancellation and tracking. Signal handlers are process-wide and installed only
through `Signals::install()`; dropping them does not restore default OS behavior.

## Development

Run the complete check sequence with `bash scripts/check`, or run individual
checks:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features --features lifecycle
cargo test --features lifecycle
cargo check --no-default-features
cargo doc --no-deps --all-features
```

The crate is imported as `simple_server`. Pezzottify and Favzetto are the first
design consumers; dependency centralization proceeds service by service.

## Repository

Fucina is the canonical repository. This project's public GitHub repository is
a one-way publication mirror; development changes are integrated in Fucina.

## Request correlation (Step 03b)

Enable the optional `correlation` feature for generated request IDs, validated
caller IDs by explicit opt-in, request extensions, scoped access, and response
headers. It does not require shared logging or lifecycle setup. Applications
retain error bodies, tracing spans and audit policy. See the
[correlation contract and usage](docs/step-03b-correlation.md).

## Logging (Step 03a)

Enable `logging` explicitly; it works without default features or a runtime.
Applications supply configuration and call the initializer during startup:

```rust
use simple_server::logging::{LogFormat, LoggingOptions, try_init};

let mut options = LoggingOptions::new("info,my_service=debug");
options.format = LogFormat::Json;
try_init(options)?;
```

Defaults are text on stderr, automatic terminal styling, and visible targets.
Compact formatting and optional close/full span events are also available.
Use `try_init_reloadable(options)` when runtime filter changes are needed; its
cloneable handle provides `set_filter` and `current_filter`. Formatting and output
remain fixed, and invalid updates leave the active filter unchanged.
Invalid filters and repeated global initialization return errors. The module
neither installs a `log` facade bridge nor reads configuration from the environment.
See the [03a contract](docs/step-03a-logging.md) and
[standalone example](examples/logging.rs) for output and compatibility details.

## Body limits (Step 04a)

Enable `body-limit` for `simple_server::body_limit::BodyLimit::max(bytes)` and
apply it explicitly to the intended routes. It preserves existing extractor
limits and rejection responses without requiring lifecycle or observability.
Raw request-body readers are not automatically limited; multipart file/part
policies remain application-owned. See the [contract](docs/step-04a-body-limits.md).

## Health and readiness

The optional `health` feature runs application-owned checks without enabling
Axum or a runtime. See the [Step 05 contract](docs/step-05-health.md).
`Probe::liveness()` checks no dependencies; readiness requires an explicit check:

```rust
use simple_server::health::{Check, Probe};

let readiness = Probe::readiness(Check::new("database", || async {
    // Run the application's real dependency check here.
    Ok::<(), std::io::Error>(())
}));
// readiness.run().await returns the first original error and its check name.
// readiness.endpoint(render) exposes a Tower service using your HTTP response.
```

Checks run in order on every request and stop at the first error. Applications
retain response schemas, status codes, timeouts and lifecycle policy. Mount the
endpoint behind GET/HEAD routing and existing access controls. The canary uses
`get_service` through the transitional router; the health API itself exposes no
Axum types.
