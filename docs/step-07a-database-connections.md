# Step 07a: SQLite connection policy

`database-sqlite` is an optional, driver-independent feature. It does not
enable an HTTP feature or add a SQLite driver. The shared
`database::sqlite::ConnectionPolicy` records only settings the application
selects. `commands()` returns ordered PRAGMAs for **each newly opened
connection**. The service chooses when to open a connection, how to create a
pool, and how to execute these statements. It can read effective PRAGMAs into a
`ConnectionObservation` and call `verify()` after a successful query such as
`SELECT 1`; verification checks only the selected fields.

The settings are foreign-key enforcement, busy timeout, journal mode,
synchronous mode and WAL auto-checkpoint interval. An empty policy changes
nothing. Invalid values and observed mismatches are explicit errors. A journal
mode request may fail to take effect (for example, WAL for an in-memory
database); `verify()` detects that. SQLite PRAGMAs that are database-wide or
affect backup policy should be selected by the application, not by a default.

Favzetto and Pezzottify require incompatible transitive SQLite C bindings
through their current SQLx SQLite and `rusqlite` versions. Keeping the shared
feature driver-independent allows both to adopt the same contract without
linking both drivers into one build. Native driver types, pool construction,
schema/migration ledgers, query mapping, transactions, and backup remain in
the consumer.

This is 07a only. Migration planning/reporting, backup coordination and a
bounded synchronous execution adapter are proposed separately in the
[Step 07 survey](step-07-database-survey.md).
