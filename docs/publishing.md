# Public Cargo releases

The canonical consumer package is `lelloman-simple-server` on crates.io. The
library target remains `simple_server`; the repository remains `simple-server`.

## Consume

```toml
[dependencies]
simple-server = { package = "lelloman-simple-server", version = "=0.1.0", features = ["web"] }
```

Retain each consumer's existing features and `default-features` setting. Commit
Cargo.lock; it records the crates.io source and archive checksum. Native builds,
CI and Docker builds download the same public package without credentials or a
sibling library checkout. Cargo still compiles the source; build caching is a
separate concern. No Fucina registry configuration is required.

## Publish

Use a clean committed checkout, run `bash scripts/check`, and inspect
`cargo package --list --registry crates-io` before publication. The explicit
package include list excludes internal planning, credentials and build outputs.
Authenticate with a crates.io publishing token using `cargo login` or the
`CARGO_REGISTRY_TOKEN` environment variable; never commit or print the token.

```sh
cargo publish --dry-run --locked --registry crates-io
cargo publish --locked --registry crates-io
```

Always bump the version for a subsequent release. Published versions cannot be
overwritten. Record the source commit and archive checksum, then independently
verify a consumer download and compile before migrating consumer lockfiles.
A Git push and deployment are separate operations.

## Historical private Fucina release

The original `simple-server` 0.1.0 package remains on the private Fucina registry.
It required private network access, registry credentials and CA trust. New public
consumers use the crates.io package above. The old publication evidence below is
retained for reproducibility; it describes a different package and archive.

## Verified release: 0.1.0 (29 September 2026)

- Published from clean Git revision `24f2db50f292ece609dd37ffc46fbc305e9c49d5`.
- Crate SHA-256: `76dd285fbdddd588b5caab30f6ac15559794a38a8717c8e81c5f56f93565268c`.
- `bash scripts/check` passed: formatting, all-target/all-feature Clippy,
  default/all-feature tests, the feature-isolation matrix, no-default-feature
  compilation, and documentation with warnings denied (522 test executions).
- Cargo publish dry run verified the extracted package. The real upload succeeded
  and Cargo confirmed registry availability. Archive: 119 files, about 193 KiB.
- An independent consumer with an initially empty Cargo cache downloaded and ran
  against Fucina with `web`, `ws`, `multipart`, `lifecycle`, `database-sqlite`, and
  `database-migrations`. Its lockfile points to the Fucina sparse index, and the
  downloaded crate matches both the lockfile checksum and the publication archive.
- Dedicated credentials: `simple-server-cargo-publish` and
  `simple-server-cargo-consume`, provisioned on homelab under the existing
  `fucina/secrets` directory, with owner-only workstation copies under
  `~/.config/fucina/` using the `.token` suffix. Tokens are not in source control.
