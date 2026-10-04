# Public Cargo releases

The canonical consumer package is `lelloman-simple-server` on crates.io. The
library target remains `simple_server`; the repository remains `simple-server`.

## Consume

```toml
[dependencies]
simple-server = { package = "lelloman-simple-server", version = "=0.1.7", features = ["web"] }
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

## Verified crates.io release: 0.1.7 (4 October 2026)

Adds optional driver-neutral SQLite backup coordination and bounded synchronous
execution. See the [backup contract](step-07c-sqlite-backup.md) and
[executor contract](step-07d-database-blocking.md). No consumer migration is
implied by publication.

Published with explicit user authorization from clean committed source
`d59f067532ca7858adcc07e78ab272ff27ba055e`. Archive SHA256:
`81ad9eef5bc3388727f06c97be1fa029c0baf39a6ba8efc021d2e9a033db74d0`.

Full `scripts/check` passes **758 test executions**, strict all-target/all-feature
Clippy, feature isolation, standalone sqlite-vec fixture, formatting and
warnings-as-errors rustdoc. The release updates the fixture lockfile to 0.1.7 and
adds both new feature-only suites to the standard check. Inspected **142 package
files**, including both new contracts; clean publish dry run and real upload pass.
Cargo confirms registry availability. An independent public archive download
matches its embedded source commit and all **68 source files** byte-for-byte.
A fresh credential-free Cargo home downloads, compiles and runs both new APIs
with default features disabled. Its lockfile records crates.io, 0.1.7 and the
same checksum. No Git push or deployment occurred.

## Verified crates.io release: 0.1.6 (4 October 2026)

Adds optional owned Unix HTTP serving/streaming clients and a UUID v4 header
request-ID generator. See [the Unix transport contract](unix-http.md).
Published following the user's go-ahead to finish the downloader from clean
committed source `58e9b7504176c82b538dca0cd3041468fb4a1ccd`. Archive SHA256:
`4f37d395af4d6ca8495ad190b8cecc5ab078ddfc92b8e9b3a41bc9eee80d3fce`.

The complete library check passes **706 test executions**, strict Clippy,
isolated Unix features, graceful-drain/streaming/error/WebSocket tests, SQLite
fixture, formatting and warnings-as-errors rustdoc. The clean committed dry run
and real publication verify 135 package files. Cargo confirms registry
availability. An independent public archive download matches the checksum,
embedded Git source revision and every library source file. Downloader, SCT and
SimpleAI lockfiles consume public 0.1.6 with this checksum, without overrides.
Their migration/verification evidence is recorded in both central trackers.
No Git push or service deployment was performed.

## Verified crates.io release: 0.1.5 (4 October 2026)

Adds optional owned streaming gzip response compression under
`web::compression`, with owned layer/service/future/body boundaries, explicit
installation, quality/toggle/u64 size/custom metadata policy controls, default
MIME exclusions and encoded/range guards. See [the contract](compression.md).

Published with explicit user authorization from clean tested source
`4a0f06ea61719ca5e9219f006988824c09d75bf6`. Archive SHA256:
`3df80ad94d2f2accc506f153787c81b6c11c13a0da670212c1e233d54ddcbab7`.
Full validation passes 687 test executions, strict Clippy, isolated features,
fixture, formatting and rustdoc. Inspected 132 package files; verification and
publication dry run pass. Cargo confirmed publication and registry availability.
An independent public archive download matches both the checksum and embedded
Git commit. Root and standalone SQLite fixture lockfiles both record 0.1.5.
Pezzottflix registry adoption and remaining-scope evidence are in the central
trackers. No Git push or deployment was performed.

## Verified crates.io release: 0.1.4 (4 October 2026)

Adds optional owned mutable request jars and cookie middleware under
`web::cookies`; see [the contract](cookies.md). Plain standard cookie data types
are exposed without Tower Cookies middleware/jar types. Existing compatibility
adapters and other features remain supported. Version 0.1.3 does not contain
this feature.

Published with user authorization from clean tested source
`40c41291d9b0f90705c93f8528eead16a9eb2477`. Archive SHA256:
`5526aea10c87982acc311e23401e6134f624f03d10470c982059163c325b595a`.
Full library validation passes 657 test executions, strict Clippy, feature
isolation and warnings-as-errors rustdoc. The root and standalone SQLite fixture
lockfiles both record 0.1.4. Package contents (130 files), package verification
and the publication dry run were checked before upload. Cargo confirmed registry
availability. Independent public archive download matches the checksum and
embedded Git commit. Lello-auth canary/adoption evidence is recorded in the
migration trackers. No Git push or deployment is implied by publication.

## Verified crates.io release: 0.1.3 (4 October 2026)

Adds optional owned directory/single-file serving via `static-files`, including
streaming GET/HEAD, ranges, modification-time conditionals, directory indexes,
explicit SPA/error-page fallback and precompressed asset negotiation. Existing
capabilities remain compatible; the filesystem backend is internal to the shared
API. See [the static-file contract](static-files.md).

Published with explicit user authorization from clean tested source
`dbc7f682f9656c1b29c6a52dfc011a94de27087a`. Archive SHA256:
`0c3c1347b1c85484b5d4ba18cd6d886688b8a1dc2d94d84bc596a6abec128ddb`.
Full library checks pass 625 test executions, strict Clippy, feature isolation
and rustdoc. Package contents and publication dry run were verified before
publication; Cargo confirmed registry availability. An independent public archive
download matches both checksum and embedded source commit. Consumer verification
and integration evidence are recorded in the migration trackers.

## Verified crates.io release: 0.1.2 (4 October 2026)

Additive STRICT/CHECK and generic virtual-table creation options. Existing public
snapshot literals, creation APIs and runtime dependencies remain compatible.
Real SQLite tests and a standalone sqlite-vec consumer harness verify the new
creation behavior. Published with user authorization from clean source
`358a226a29d66e6f8a73df4573799c8b0970c8cd`. Archive SHA256:
`d31aa705a4b51e0dfea8fdc334de20f8cf6048b049dedb970ef63227a7661d46`.
Full library checks, package verification and publication dry-run passed.
Independent crates.io archive download matches source commit and checksum.
Fausto and Crumbles consume the published package with locked registry checks;
see the migration trackers for adoption scope and test evidence.

## Verified crates.io release: 0.1.1 (3 October 2026)

Adds additive SQLite extended creation descriptions for AUTOINCREMENT, FTS5 and
triggers. Existing creation/comparison APIs and runtime dependencies are unchanged.
Peerlo's metadata canary is tested against this source before registry publication.
Published with explicit user approval from clean source
`bda33540e410bc759a80e8627e132923b8996859`. Archive SHA256:
`f7fd8567b285a8e1cb8315626df78c8d56b444907e8a9a5ac9e83f5db6e47f25`.
Full shared checks, package verification and publication dry-run passed.
Independent crates.io archive download matches checksum and embedded source commit.
Peerlo downloaded the published package without an override: 236 targeted tests
pass (one existing ignored doctest), and workspace check passes with existing CLI
warnings. Its master is integrated at `1ce8481`; temporary worktrees are removed.

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
