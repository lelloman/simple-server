# Rich SQLite capabilities

Status: 07e schema core implemented, 2026-09-29; Pezzottify versioned-helper canary complete.
See the [implemented API and coverage](step-07e-sqlite-schema.md). Backup and
blocking execution are published in crates.io 0.1.7 (2026-10-04); consumer
adoption remains Pending. See [07c](step-07c-sqlite-backup.md) and
[07d](step-07d-database-blocking.md) for the implemented contracts.

## Objective and composition

Provide reusable connection setup, schema descriptions and validation,
migration coordination, backup preparation and bounded synchronous execution.
Services choose the pieces they need. Existing 07a and 07b remain compatible;
enabling database features does not require HTTP, authentication or lifecycle.

| Increment | Responsibility | State |
| --- | --- | --- |
| 07a — connection policy | Selected PRAGMAs and effective-setting verification | Implemented; applicable rollout complete |
| 07b — migration preflight | Existing ledger/marker inspection and pending suffix | Implemented; applicable rollout complete |
| 07e — SQLite schema | Versioned descriptions, creation plans, observations and validation | Shared core implemented; Pezzottify canary complete |
| 07c — backup coordination | Checkpoint/copy strategies, verification and outcome reports | Published 0.1.7; consumer canaries pending |
| 07d — synchronous execution | Bounded workers, priorities, lanes, deadlines and drain | Published 0.1.7; consumer canaries pending |

07c/07d retain their existing roadmap identifiers. 07e is implemented ahead of
them to address the current Pezzottify requirement without renumbering history.
The umbrella is `database::sqlite`, with optional `schema` and `backup`
submodules. The executor belongs under `database::blocking`, since its scheduling
contract need not depend on SQLite. Each new capability has its own optional
Cargo feature. Schema descriptions and validation must be usable without the
migration planner, backup coordinator or executor.

## Evidence and limits of the starting point

Pezzottify's `sqlite_persistence/versioned_schema.rs` defines whole-schema
snapshots containing tables, columns, indexes, unique constraints and foreign
keys. Each snapshot has one version and an optional migration function. This
is not independent version counters for each table or column. Its catalog also
classifies older unmarked layouts before choosing a migration path.

The shared design generalizes these descriptions rather than copying all their
implementation details. The current local validator checks column order,
declared types, nullability, defaults, primary keys, index existence, selected
unique constraints and foreign keys. Some index properties are not compared,
and some introspection errors are discarded. Shared validation must report
unverified properties and propagate observation failures.

Other required boundaries come from Crumbles' strict ledger and staged copies,
Fausto's transactional/nontransactional migrations, and SCT's offline SQLite
archive, which is separate from its PostgreSQL server. Typed domain repositories
and SQL queries remain consumer code; this project does not introduce an ORM.

## 07e: descriptions and observations

Public concepts (see the [implemented contract](step-07e-sqlite-schema.md) for
exact field names and coverage):

- `SchemaSnapshot`: application namespace, logical version and table/index
  descriptions. Versions refer to complete expected layouts. Optional change
  provenance may identify the version that introduced a column, but does not
  establish a separate persisted column ledger.
- `TableSpec` / `ColumnSpec`: safely quoted identifiers, declared type, explicit
  nullability, ordered primary-key membership, uniqueness and default expression.
  Unsupported table options are reported explicitly. Include composite keys, ordered index terms, partial
  indexes and composite foreign keys with update/delete actions.
- `SqlExpression`: explicitly trusted application SQL for defaults, predicates
  or expressions. It is never constructed from request input implicitly.
  Identifiers and SQL expressions are distinct types.
- `SchemaObservation`: driver-neutral facts read from SQLite metadata, including
  the database/schema namespace and the properties the adapter can observe.
  Unavailable facts are distinct from absent objects and query failures.
- `ValidationPolicy` / `SchemaReport`: explicit comparison rules and structured
  differences with table/column/index paths, expected and observed values,
  checked properties and unsupported checks. Reports exclude row contents.

Support borrowed/static definitions and owned runtime descriptions without
requiring code generation. Do not restrict the public type model to Pezzottify's
four declared types: retain declared type text and make any affinity-based
comparison an explicit policy. Distinguish declared nullability and key metadata
from inferred runtime constraints. Adapters must not invent missing information.

Initial structured coverage includes ordinary tables, columns, primary and
unique keys, regular/partial indexes and foreign keys. Generated columns,
STRICT/WITHOUT ROWID properties, expression indexes, CHECK constraints, triggers,
views and virtual tables must either have supported observations/comparisons or
be reported as outside coverage. Opaque application-managed objects can coexist;
they must never be advertised as structurally verified merely because they exist.

Two explicit policies are required:

1. **Exact:** require the selected complete scope to match, including unexpected
   objects and ordered fields where order is part of the contract.
2. **Required subset:** check declared requirements and permit extras. Reports
   explicitly identify the subset scope and do not claim whole-schema parity.

Neither policy is a silent default. Pezzottify adoption must select its current
comparison behavior deliberately; stronger checks are introduced only after
historical fixtures show compatibility. Default-expression comparison starts
conservatively: exact text plus a narrowly specified, tested normalization.
It must not strip arbitrary parentheses or claim SQL-expression equivalence.

## Creation and migration execution boundary

`create_plan(snapshot)` generates deterministic, inspectable creation statements
for supported descriptions. It does not run them or write version markers.
Unsupported definitions fail before any SQL executes. Creation and observation
are tested as a round trip with real databases.

Startup composition is explicit:

1. Acquire the service's migration ownership/lock and open/configure connections.
2. Read the real marker/ledger and observe the schema under the service's
   consistency boundary. Observation outside that boundary is diagnostic only.
3. Let the application classify fresh, known legacy, managed or incompatible
   state. A marker of zero is not enough to classify a database as fresh.
4. Run 07b preflight and validate the relevant existing schema description.
5. Execute application-authored migration SQL/callbacks under declared
   transaction rules; validate the target before committing its version where
   the engine and migration permit an atomic operation.
6. Re-observe and report the resulting schema/version. Preserve the original
   database error as the cause of any failure.

No schema diff automatically generates ALTER/DROP/rebuild operations. A diff
cannot infer renames, data transformations or application invariants. No legacy
layout is silently adopted and no checksum history is synthesized.

The first implementation provides descriptions, plans and validation, leaving
execution in the existing consumer runners. A later shared runner may coordinate
explicit callbacks, but must declare per-migration versus whole-upgrade
transactions, caller-owned transactions, nontransactional operations and their
recovery semantics. It must not automatically retry partially executed work.
Existing `user_version` offsets and ledger formats remain application adapters;
Pezzottify's offset is not a library default.

## Driver adapters and typed access

Keep the core free of SQLx, rusqlite and SQLite native linking dependencies.
The initial adapters live in consumers and translate native metadata into shared
observations. If this translation becomes repetitive, extract separately
versioned adapter crates after proving compatibility with the actual driver
versions. A feature flag alone does not solve incompatible native SQLite
dependencies; do not add both bindings to the core crate.

Native connections, transactions, errors, row mappings and prepared statements
remain usable. Closures or repository methods may return typed application
records. Shared schema types describe storage structure, not Rust domain models.
Any future row-mapping helper requires a concrete repeated consumer need.

## 07c: backup and checkpoint coordination

Provide distinct strategies with explicit preparation, operation, verification
and publication phases:

- **Checkpoint preparation:** Pezzottify-style registry, explicit checkpoint
  mode, per-database busy/page counts and failures. Checkpoint success alone is
  not a completed backup or permission to copy files while writes/checkpoints
  can race. The application must hold an appropriate coordination guard through
  the file-copy phase, or select a consistent-copy strategy instead.
- **Consistent staged copy:** Crumbles-style copy into a new destination,
  schema/integrity checks and an explicit publish step. Track incomplete output
  and never overwrite the source or an existing published artifact implicitly.

Reports distinguish prepared, copied, verified, published, busy and failed.
Multi-database results identify each file and do not claim an atomic cross-file
snapshot unless the caller supplies a tested coordination protocol. Restore is
a separate explicit action with validation and exclusive ownership; automatic
restore after a migration failure is out of scope. Services own destinations,
permissions, retention, encryption, external payloads and deployment fencing.

## 07d: bounded synchronous execution

Generalize Pezzottify's worker model with configurable priority classes,
application-defined lane identities/limits, bounded queues, admission deadlines,
execution deadlines, fairness, panic isolation and explicit shutdown/drain.
No application-specific lane names or workload budgets become shared defaults.

Cancellation before dispatch can prevent an operation from starting. Once a
synchronous operation starts, caller timeout does not prove that it stopped or
rolled back. Keep worker/lane capacity occupied until the operation actually
finishes, report that distinction, and never automatically retry a timed-out
write. Interrupt support is an optional driver capability. The same scheduler
must serve async and blocking callers without blocking async runtime workers.

## Delivery and acceptance gates

1. Implement 07e pure descriptions, deterministic DDL plans, comparison policies
   and structured reports. Test unsupported checks as thoroughly as mismatches.
2. Add real-SQLite adapter fixtures for empty/current/older layouts, changed
   defaults, column order, composite keys, missing/wrong indexes, partial indexes,
   foreign-key actions, unusual quoted identifiers and metadata read failures.
   Validate supported SQL generation against each driver's SQLite version.
3. Canary on Pezzottify, starting with the shared versioned-schema helper and one
   representative store, then its other versioned stores. Preserve historical
   fixtures, marker offsets, legacy classification and error/transaction behavior.
   Test rollback and restart after failure; never use live databases.
4. Canary on a SQLx service with an actual need for schema validation before
   extracting any driver adapter. Source-audit applicability first; do not add
   new schema policy to every service just to populate a status column.
5. Implement 07c against both checkpoint and staged-copy fixtures, including
   competing readers/writers, partial failure, destination collision, failed
   verification and cleanup. Then implement 07d with deterministic admission,
   fairness, timeout, cancellation, panic and shutdown tests.

Each runtime increment receives a concrete API review and its own acceptance
record. Consumer work follows the isolated-worktree/rebase/cleanup workflow and
updates both migration trackers. 07a/07b Done status never implies adoption of
these proposed capabilities. Nothing in this design changes the excluded status
of Quentin Torrentino.
