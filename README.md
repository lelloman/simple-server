# simple-server

A small, modular Rust library for shared infrastructure across Axum services.
The first stage centralizes Axum versions through a transitional re-export.
Subsequent stages will extract shared capabilities and gradually hide Axum behind
the library's own interfaces.

**Status:** Axum dependency centralization. Lifecycle and other infrastructure
modules are not implemented yet. Publication to crates.io is disabled during
this initial stage.

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

The re-export intentionally exposes Axum during migration. It does not start a
runtime, install signals, or change application startup, shutdown, or middleware.
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

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --no-default-features
cargo doc --no-deps --all-features
```

The crate is imported as `simple_server`. Pezzottify and Favzetto are the first
design consumers; dependency centralization proceeds service by service.

## Repository

Fucina is the canonical repository. This project's public GitHub repository is
a one-way publication mirror; development changes are integrated in Fucina.
