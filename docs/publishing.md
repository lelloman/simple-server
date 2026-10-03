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

## Prepared release: 0.1.1

Adds additive SQLite extended creation descriptions for AUTOINCREMENT, FTS5 and
triggers. Existing creation/comparison APIs and runtime dependencies are unchanged.
Peerlo's metadata canary is tested against this source before registry publication.
This version is **not published yet**. Publication requires separate approval;
consumer registry integration remains pending until the download is verified.

## Verified crates.io release: 0.1.0 (29 September 2026)

- Public package: https://crates.io/crates/lelloman-simple-server/0.1.0
- Source commit: `c95506164669c37a12bc06339a3b977a4b8d6a1b`.
- License: `MIT OR Apache-2.0`; both license texts are in the archive.
- Crate SHA-256: `1f3187c81c94701cd041df7ab17ce968b73c967db77c551170e3c24960b6c77a`.
- Full `scripts/check` passed: formatting, strict all-target/all-feature Clippy,
  default/all-feature tests and feature matrix (569 test executions),
  no-default-feature compilation, and rustdoc with warnings denied.
- Clean-checkout publish dry run passed; publication uploaded 124 files,
  approximately 212 KiB compressed, and Cargo confirmed registry availability.
- An independent consumer with a fresh Cargo home and no registry credentials
  downloaded, compiled and ran with `database-sqlite-schema`. Its lockfile points
  to crates.io; the downloaded archive SHA-256 matches its lockfile checksum.
- Consumer migrations and their verification are tracked separately in
  `migration-status.md`; publication alone does not update application manifests.

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
