# Step 07: database capability survey

Status: design survey, 2026-09-28. The first optional connection-policy
capability is specified in [Step 07a](step-07a-database-connections.md);
migrations, backup and typed access remain future work. Step 07 is separate
from the completed HTTP abstraction.

## Representative services

This survey covers five distinct database arrangements. Paths are relative to
the sibling project directories in `lelloprojects`.

| Service | Connection and execution | Schema changes | Backup or recovery boundary |
| --- | --- | --- | --- |
| Pezzottify | `rusqlite`; multiple application-owned files; connection policy sets foreign keys, a two-second busy timeout and `synchronous=NORMAL`; a priority-aware executor bounds synchronous work (`pezzottify/pezzottify-server/src/sqlite_persistence/connection.rs`, `src/db_executor.rs`). | `PRAGMA user_version`, schema-shape validation and store-specific migrations; catalog recognizes legacy layouts and migrates in a transaction (`src/sqlite_persistence/versioned_schema.rs`, `src/catalog_store/store.rs`). | A registry disables WAL auto-checkpointing; explicit TRUNCATE checkpoints prepare registered files for external backup (`src/backup/db_registry.rs`, `src/backup/mod.rs`). |
| Favzetto | SQLx SQLite pool, one connection, WAL and foreign keys (`favzetto/backend/src/db/mod.rs`). | Sorted SQL files, an application-owned ledger and a transaction per file, followed by legacy schema repairs (`backend/src/db/mod.rs`). | No database backup mechanism identified in the inspected backend code. |
| Crumbles integration runtime | SQLx SQLite pool with private directory and file rules, foreign keys, busy/acquire timeouts and bounded connections (`crumbles/crumbles-integration/src/db.rs`). | Strict version/name/checksum/dirty ledger, transactional forward migrations and rejection of unmanaged or altered state (`src/db.rs`). | Staged upgrade copies live SQLite with `VACUUM INTO`, migrates and integrity-checks the copy, and fences restoration (`src/db.rs`). |
| Fausto | `rusqlite` connection behind a mutex; optional SQLite extension registration and WAL (`fausto/core/src/storage/sqlite/mod.rs`). | Embedded SQL files and a version table; migration metadata distinguishes transactional from nontransactional DDL; legacy unversioned data has an adoption path (`src/storage/sqlite/mod.rs`). | No generic SQLite-file backup facility identified in the inspected storage module. Domain copy/migration checkpoints are a separate concern. |
| SCT | SQLx PostgreSQL pool with lazy connection, bounded size and acquire timeout (`sct/crates/sct-core/src/catalog.rs`). | Embedded SQLx `Migrator` with locking, transactional migrations and rejection of missing history (`src/catalog.rs`). | Application-level checkpoint/export and recovery workflows span database and payload storage (`crates/sct-server/src/checkpoint_api.rs`, `crates/sct-core/src/recovery.rs`); they are not a generic SQL backup operation. |

This is a representative design sample, not an adoption audit of every service.
In particular, Crumbles' integration runtime is a separate database component
within the Crumbles repository.

## What is actually shared

1. **Connection policy:** setting timeouts, foreign-key enforcement, WAL or pool
   limits and a readiness probe is common, but values and driver types differ.
   A small optional adapter could expose explicit configuration and verification
   for `rusqlite` and SQLx SQLite. PostgreSQL needs a separate adapter.
2. **Migration safety:** every service needs ordered changes and understandable
   failures. Their existing ledgers are incompatible: Pezzottify uses
   `user_version`, Favzetto and Fausto have local tables, Crumbles validates a
   strict checksum ledger, and SCT uses SQLx's ledger. A shared runner must never
   reinterpret one of these ledgers or silently mark a legacy database current.
   Start with a driver-independent migration plan/report contract and optional
   adapters that preserve each service's ledger and transaction rules. Migration
   SQL, compatibility detection and schema validation remain application-owned.
3. **Typed access:** the drivers already map rows into Rust types; services own
   their domain repositories and query semantics. Step 07 should document and
   compose those boundaries, not introduce a competing ORM or a generic record
   model. Any helper should keep the driver's native row and error types usable.
4. **Backup:** there is no single `backup()` implementation. Pezzottify needs
   coordinated WAL checkpoints before file backup; Crumbles uses a consistent
   staged copy and integrity check; SCT's recovery spans PostgreSQL state and
   external payloads. A useful common contract is explicit *prepare, perform,
   verify, report* phases with failures visible to the caller. Engine-specific
   operations and the artifact/retention policy belong to adapters and services.
5. **Synchronous work:** Pezzottify and Fausto use synchronous `rusqlite`, while
   the SQLx services are async. A bounded blocking-operation adapter may be
   useful, but should be designed against Pezzottify's existing lane, priority,
   cancellation and shutdown behavior before offering it as a shared feature.

## Proposed implementation sequence

- **07a — contracts and connection adapters:** define opt-in readiness and
  connection policy for SQLite drivers, with explicit values and observable
  verification. Do not change a consumer's defaults implicitly.
- **07b — migration planning and reporting:** compare ordered plans to recorded
  state, detect gaps, changed migrations and newer database versions, and report
  what would run. Implement execution only for a driver/ledger combination with
  proven parity; never replace an existing ledger as part of a first adoption.
- **07c — backup coordination:** model preparation and verification outcomes;
  prototype a SQLite-specific adapter against both Pezzottify's checkpoint
  contract and Crumbles' staged-copy contract. Treat them as distinct strategies.
- **07d — optional synchronous execution:** assess whether the existing task
  primitives can host bounded database work without weakening Pezzottify's
  priority, lane, cancellation or shutdown guarantees.

Before selecting a canary, compare the exact behavior and fixtures of the
candidate adapter with an existing service. The initial implementation should
have tests for existing databases, failed/partial migrations, version drift,
busy connections and backup failure. A service only counts as adopted when its
real startup and data paths use the helper and its original behavior is verified.

No service should be required to add a database, switch drivers, replace its
schema ledger, hand over transaction ownership, or route all queries through
simple-server to adopt another module.
