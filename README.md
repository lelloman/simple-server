# simple-server

A small, modular Rust library for shared infrastructure across Axum services.
The first stage centralizes Axum versions through a transitional re-export.
Subsequent stages will extract shared capabilities and gradually hide Axum behind
the library's own interfaces.

**Status:** Axum dependency centralization and opt-in lifecycle helpers are
implemented. Favzetto, LelloStore, LelloAuth, Fausto, and Meteonesto have adopted lifecycle helpers;
Pezzottify adoption awaits approval. Publication to crates.io remains disabled.

Open the [HTML migration matrix](docs/migration-status.html) in a browser for adoption status
across all 17 planned consumer projects and the completed steps for each one.

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

Lifecycle helpers are available now; the remaining items below are planned.

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
