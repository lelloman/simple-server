# simple-server

A small, modular Rust library for shared infrastructure across Axum services.
Applications keep ordinary Axum routers, handlers, and middleware while sharing
server lifecycle and operational conventions.

**Status:** initial library scaffold and design direction. No server modules
or runtime API are implemented yet; no existing services have been migrated.
Publication to crates.io is disabled during this initial stage.

## Initial scope

- Listener setup, shutdown signals, graceful shutdown, and shutdown deadlines.
- Structured logging, request IDs, and HTTP tracing.
- Health and readiness endpoint plumbing with application-provided checks.

Metrics, background-task supervision, and authentication integration are
potential follow-up modules, added when adoption demonstrates a shared need.

Applications own their routes, state, configuration loading, database setup,
authorization rules, background jobs, and domain logic. Capabilities should be
optional, configuration explicit, and dependency upgrades independently adopted.

See [the design outline](docs/design.md) for boundaries and the adoption plan.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

The crate is imported as `simple_server`. Dependencies and the first public API
will be introduced with the first implementation.

## Repository

Fucina is the canonical repository. This project's public GitHub repository is
a one-way publication mirror; development changes are integrated in Fucina.
