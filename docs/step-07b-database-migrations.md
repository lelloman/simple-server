# Step 07b: migration planning and reporting

Status: shared preflight implemented, no consumer adoption yet, 2026-09-28. It
follows the optional [07a connection policy](step-07a-database-connections.md).

## Scope and boundary

Provide an optional, driver-neutral **preflight planner** that compares an
application's ordered migration manifest with the state its existing ledger
reports. The application reads its ledger, computes or supplies checksums using
its established algorithm, and supplies normalized records. The planner returns
an ordered pending suffix or a precise incompatibility. It does not execute SQL,
open a connection, create/replace a ledger, choose a transaction boundary,
adopt a legacy database, or infer that a schema is healthy from version numbers.
The application's existing migration runner and schema validation remain the
authority. No automatic repair or rollback is part of 07b.

The API is independent of SQLite, SQLx and `rusqlite`. This keeps
the PostgreSQL catalog in SCT and the differing SQLite driver versions usable
without linking database drivers into `simple-server`.

## Input and output contract

- A plan has one **namespace** per independent database/ledger and an ordered
  list of entries: stable version, stable name, and an optional **opaque** digest.
  The planner must never hash or normalize SQL itself. Existing byte/hash rules
  differ, and changing them would falsely flag historical migrations. Numeric
  versions use numeric order; text versions use lexicographic order, suitable
  for sorted filename ledgers. A plan cannot mix the two kinds.
- Recorded entries have the same identity fields plus an application-supplied
  completion state (`applied` or `dirty`). An adapter can supply a separate
  observed high-water mark, such as SQLite `user_version`, when its ledger uses
  one. A missing digest is valid for an entry only when both sides lack it; a
  mismatch in availability is not silently accepted.
- Input validation rejects duplicate or out-of-order versions, empty identity,
  inconsistent ledger records, and a dirty record. Comparison checks every
  recorded entry against the matching manifest entry, then requires the
  recorded history to be a **prefix** of the manifest. It distinguishes unknown
  or newer recorded versions, gaps, renamed entries, changed digests, and
  high-water/ledger disagreement. None is treated as an empty pending plan.
- The result contains the matched prefix and ordered pending entries, with a
  read-only report and a count of historically verified digests, suitable for
  startup diagnostics or a dry run. It makes no claim that applying the suffix
  is safe until the application validates its
  schema, locks/transaction rules and external dependencies.

The initial API should fail closed on unknown ledger state. Explicit
application-owned adapters can map special histories into the normalized
records only after tests prove parity. A version-only ledger cannot gain
checksum guarantees by passing a newly computed manifest digest as if it were
historically recorded.

Enable `database-migrations` independently of `database-sqlite`, then construct
`database::migrations::MigrationPlan` and `MigrationObservation` from the
application's own manifest and ledger. Call `plan.inspect(&observed)` before
the application's existing migration runner. `None` for `high_water_mark`
means the ledger has no separate high-water observation; map a genuinely empty
SQLite ledger's `user_version = 0` sentinel to `None`. The returned
`MigrationReport` owns its applied and pending entries. An error stops the
preflight; it never modifies the ledger. A version-only ledger must use `None`
digests on both sides, and its `verified_digests` count remains zero.

## Why these rules are needed

| Existing component | Migration boundary that 07b must preserve |
| --- | --- |
| Pezzottify SQLite stores | `PRAGMA user_version`, store-specific schema inspection, legacy layouts and transactional catalog upgrades. Version alone is not proof of layout. |
| Favzetto backend | Sorted SQL files and a version ledger, each file in its own transaction, followed by separate legacy repairs. The ledger does not record digests. |
| Crumbles integration runtime and Simple Agents service | Strict version/name/checksum/dirty ledgers; a dirty or altered entry is a startup failure. Their runners own transaction and recovery behavior. |
| Meteonesto weather pipeline | A checksum ledger and `user_version` must agree; its application checks migration name and checksum. The planner must accept both observations without choosing a winner. |
| Fausto SQLite storage | A version/name table, a legacy unversioned adoption path, and migrations marked transactional or nontransactional. The planner must not wrap or silently adopt those migrations. |
| Lello Auth SQLite, LelloStore and Pezzottflix | Lello Auth has an application-owned version table; LelloStore and Pezzottflix use SQLx migration state. A shared planner cannot reinterpret or replace those ledgers. |
| SCT | Server catalog uses PostgreSQL and SQLx `Migrator`; the offline archive SQLite catalog has a fixed format marker rather than a migration ledger. These are separate namespaces and may have different applicability. |

## Implementation sequence and acceptance

1. **Done:** add a small optional `database-migrations` feature with pure in-memory
   manifest/observation validation and a typed report/error API. No database
   driver dependency or default behavior. Keep 07a and 07b independently
   selectable.
2. **Done:** test empty and fully applied histories, pending suffixes, duplicate/gapped
   records, altered names/digests, dirty entries, newer versions, and high-water
   disagreement. Include a version-only ledger fixture so the API cannot claim
   historical checksum verification it did not perform.
3. **Next:** pilot on a service whose current runner can consume the report without
   changing its ledger, SQL ordering, transaction boundaries, legacy handling,
   or error semantics. Compare preflight and real startup against fresh,
   existing, drifted and partially failed databases. If a runner cannot retain
   those behaviors, leave that component Pending rather than adapting its
   history to fit the helper.

Execution adapters are a later, separately reviewed increment. Backup and
checkpoint coordination remain [07c](step-07-database-survey.md), and typed
domain access remains in consumers.
