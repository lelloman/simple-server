# Private Cargo releases on Fucina

`simple-server` is published under the `lelloman` owner on Fucina. This is a
versioned Rust source package, not a precompiled library. Cargo downloads it;
consumers no longer need a sibling checkout. Compilation reuse requires build
caching separately.

## Publish

Use a clean committed checkout. Run `bash scripts/check` and inspect
`cargo package --list --registry fucina` before releasing. The archive includes
the library, tests, examples, manifest, lockfile, README and this guide; it excludes internal
planning documents, agent settings, build outputs and credentials.

The registry is restricted through `publish = ["fucina"]` and configured in
`.cargo/config.toml`. Supply a dedicated `write:package` token in the owner-only
file `~/.config/fucina/simple-server-cargo-publish.token`, or use
`FUCINA_CARGO_TOKEN_FILE` / `FUCINA_CARGO_TOKEN`. The helper adds the `Bearer `
prefix expected by Forgejo, passes the token in the Cargo environment, and does
not change global Cargo settings or place credentials in command arguments.

```sh
python3 scripts/publish-fucina.py --dry-run
python3 scripts/publish-fucina.py
```

Never delete or overwrite a release to replace its contents. Bump the version,
update Cargo.lock, validate and commit before publishing the next version. Keep
the publishing Git revision and package checksum as release evidence. A source
Git push and consumer migrations are separate operations.

## Consume

Add to the consumer's `.cargo/config.toml`:

```toml
[registries.fucina]
index = "sparse+https://fucina.homelab/api/packages/lelloman/cargo/"
credential-provider = "cargo:token"
```

Then replace the path dependency, preserving the features that consumer needs:

```toml
[dependencies]
simple-server = { version = "=0.1.0", registry = "fucina", features = ["web"] }
```

Provide `CARGO_REGISTRIES_FUCINA_TOKEN` as `Bearer ` followed by a dedicated
consumer token. Developers and builders need LAN/VPN access, private DNS, and
trust in Fucina's Caddy root CA. Workstation system trust is not automatically
available inside Docker. Do not disable TLS verification. For Docker builds,
provide credentials using BuildKit secrets, never build arguments or image ENV.
Use an appropriate trusted CA bundle in the build stage.

Verify a first release using an independent consumer and an empty Cargo cache;
check Cargo.lock identifies Fucina and compare the downloaded crate checksum
with the release archive. Compile with the consumer's actual feature set.

Forgejo 15 package scopes follow owner permissions, and its read/write package
token scopes are not a proven write-isolation boundary. Keep consumer tokens in
trusted build environments. See the homelab `fucina/PACKAGES.md` runbook for
credential provisioning, backup, and rotation.

Publishing alone does not change existing service manifests or Docker builds.
