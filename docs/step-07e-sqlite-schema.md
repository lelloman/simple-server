# Step 07e: SQLite schema descriptions and validation

Implemented in the shared library, 2026-09-29. Pezzottify has completed the versioned-helper canary; other consumers await applicability review.
Enable `database-sqlite-schema` and use `database::sqlite::schema`. This feature
adds no runtime dependencies and does not enable HTTP or migration preflight.
It includes the existing lightweight connection-policy feature. Backup and
blocking execution remain separate planned increments of the
[richer SQLite design](step-07-sqlite-design.md).

## Creating descriptions and plans

`SchemaSnapshot` contains an application namespace, attached database identifier,
logical version and ordered `TableSpec` descriptions. The version labels the
snapshot/report; it is not a persisted marker or proof of migration history.
Use 07b independently for ledger/version checks. Strings and description lists
use `Cow`, supporting borrowed application definitions and owned runtime input.
There is no required macro, code generator or ORM.

`ColumnSpec` describes declared type text, metadata nullability and a default.
Table-level lists describe ordered primary keys, unique constraints and composite
foreign keys. `IndexSpec` describes named indexes, ordered column terms,
collations, sort direction, uniqueness and an optional partial-index predicate.
Column-level UNIQUE is represented as a one-column table unique constraint.

`Identifier::new` rejects empty/NUL-containing names and quotes embedded quotes.
Names compare using ASCII case-insensitive matching. Declared type text is also
quoted during generation; it is not interpreted as raw SQL. An empty declared
type represents an untyped column. `SqlExpression::trusted` explicitly accepts
application-authored SQL for defaults and predicates. It checks empty/NUL text,
not SQL grammar or safety; never populate it from untrusted request data.

`create_plan(&snapshot)` validates definitions, references to local columns,
name collisions and unsupported requirements, then returns deterministic
`CREATE TABLE` and `CREATE INDEX` statements. Defaults are enclosed in one SQL
expression wrapper. Statements target the selected attached database. The plan
contains no transaction, `IF NOT EXISTS`, ALTER, DROP or version-marker writes.
The caller executes it under its existing transaction/ownership rules. SQLite
still validates expression syntax and engine-specific constraints at execution.

Run the executable borrowed-description example:

```sh
cargo run --locked --no-default-features --features database-sqlite-schema --example sqlite_schema
```

## Observing and comparing

Adapters populate `SchemaObservation` and `TableObservation` from the real
connection within the application's migration lock/transaction boundary. Read
complete inventories, preserve column/key order and include unexpected objects.
Exclude SQLite internal objects and implicit primary-key indexes; represent
implicit UNIQUE constraints separately from explicit indexes.

Every observation group is `Observed::Known(value)` or
`Observed::Unavailable(reason)`. A metadata read or decoding failure must return
its original driver error from the adapter, not an empty collection or a partial
success. There is no shared driver dependency or blanket production adapter.
The test adapter in `tests/database_sqlite_schema.rs` is deliberately limited to
reviewed fixtures; it is not a general SQLite DDL parser.

`validate(&snapshot, &observation, policy)` returns `SchemaReport` or a definition
error for invalid expected input/ambiguous duplicate observations. Select all
four `ValidationPolicy` fields explicitly:

- `scope`: Exact rejects extra tables/columns/indexes/constraints/objects.
  RequiredSubset permits extras and lists them in `outside_scope` without
  claiming they were structurally validated. An empty expected primary key is
  still an explicit requirement for no key; columns describe complete properties.
- `column_order`: Ordered compares relative order within the selected scope;
  ByName permits reordered columns. Key and index-term order is always checked.
- `declared_types`: Exact or AsciiCaseInsensitive. There is no implicit affinity
  conversion (e.g. INTEGER and BIGINT are not treated as equivalent).
- `expressions`: Exact or TrimWhitespace, which removes only outer ASCII
  whitespace. Neither rewrites parentheses, literals or SQL expression semantics.

The report contains the selected policy, snapshot identity, checked paths,
excluded extras, and differences classified as Missing, Unexpected, Mismatch or
Unverified. `is_match()` is false for both mismatches and unavailable checks. A
true RequiredSubset result verifies only its declared requirements; even an Exact
result does not establish data integrity, application invariants or ledger health.
Reports include schema metadata, never row data.

## Explicit coverage boundaries

The initial model supports ordinary rowid tables and the structured properties
above. It does not yet describe generated columns, virtual tables, STRICT or
WITHOUT ROWID tables, CHECK clauses, triggers, views, expression indexes,
AUTOINCREMENT, non-default column collations, conflict clauses, implicit foreign
key target columns or deferral. Populate `TableSpec::unsupported` for expected
requirements outside this model; creation fails before returning a partial plan
and validation reports them as Unverified.

`TableObservation::ordinary_table = Known(())` is a deliberate adapter assertion
that additional table properties are absent. PRAGMAs alone cannot establish that
assertion for arbitrary DDL. Inspect the DDL, validate it against a known supported
format, or return Unavailable. Never mark unknown extended tables as ordinary.
Unknown index properties can mark the indexes group Unavailable. List views,
triggers and other out-of-model objects in `other_objects`; Exact rejects them,
while RequiredSubset explicitly excludes them. Read failures are still errors.

This conservative coverage supports migrating Pezzottify's shared descriptions
without silently claiming a full SQL parser. A canary must explicitly preserve
its existing comparison policy and historical fixtures. Stronger checks are a
separate behavior change. Unsupported constructs can remain application-owned
while a later increment adds properly tested shared representations.

## Verification and next gate

The regression suite uses actual SQLite databases, including a file-backed
upgrade/rollback/reopen fixture. It covers composite keys, foreign-key actions,
partial and altered indexes, defaults/types/nullability, quoted identifiers,
attached schemas, missing/extra objects, unsupported/unknown metadata, invalid
specifications and borrowed descriptions. Creation does not alter an existing
`user_version`; an application-owned failed transaction preserves records and
its old marker across restart.

Pezzottify has completed its canary. SimpleAI and Paranza use shared creation
plans; Peerlo has Partial tracker adoption. Remaining rollout was stopped at user
request; see the [checkpoint](migration-status.md#step-07e-consumer-rollout-checkpoint--2026-09-30). Driver
introspection follows SQLite's [PRAGMA documentation](https://www.sqlite.org/pragma.html),
with generated statements checked against [CREATE TABLE](https://www.sqlite.org/lang_createtable.html)
and [CREATE INDEX](https://www.sqlite.org/lang_createindex.html).

## Explicit validation profiles

The Pezzottify canary required mixed comparison depth: exact ordinary columns
inside a subset of tables, with presence-based checks for selected constraints.
`validate_with_profile` accepts a `ValidationProfile` for this use case.
`validate` remains equivalent to `ValidationProfile::uniform(policy)` and keeps
its original strict checks.

A profile selects column scope and case matching independently of table scope;
ordered or first-column primary-key comparison; full index definitions or
case-sensitive names only; ordered UNIQUE constraints or unordered named-column
sets; full foreign keys or case-sensitive per-column targets and deletion actions;
and whether additional table properties must be verified. Every excluded property
is listed in `SchemaReport::outside_scope`, and the report records the full profile.
Unavailable metadata for any selected property still prevents a match.

`TableObservation::index_names` and `unique_column_sets` are separate metadata
projections. Adapters never need to invent index terms/predicates or claim a
complete constraint definition to perform presence-based validation. Named-column
sets alone do not prove unconditional uniqueness: expressions, partial predicates,
collations and term order remain excluded. First-column primary-key comparison
does not validate later key members. A profile that excludes additional table
properties may inspect regular `table_info` columns without claiming coverage of
hidden/generated columns or DDL clauses. Applications must document this scope.

Pezzottify retains its existing one-layer default-parenthesis normalization in
its adapter before shared comparison. The general library does not treat
arbitrary SQL expressions as equivalent or enable that normalization implicitly.


## Pezzottify canary

The production versioned helper uses shared plans and a mixed-depth profile that
preserves its previous validation policy. Native I/O, descriptor syntax, default
normalization, SQL upgrades and transaction ownership remain local. All 38
historical snapshots pass differential tests; file-backed startup, rollback,
restart and retry pass. See the [canary evidence](migration-status.md#step-07e-pezzottify-schema-canary--2026-09-29).
The shared suite now includes 22 schema tests and passes the full check script.


## Driver-owned idempotent creation

Adapters adding IF NOT EXISTS to generated indexes must test malformed existing
schemas: SQLite DQS_DDL may interpret a missing double-quoted column as a string
literal. SimpleAI and Peerlo temporarily disable that fallback during generated
index execution, restore the setting on success/error, and test existing-index
acceptance and partial-bootstrap failure order. Execution remains driver-owned.
