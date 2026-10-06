//! Optional database contracts. Drivers, schemas and queries remain
//! application-owned.

/// Optional engine-backed SQLite execution; independent policy APIs remain above it.
#[cfg(feature = "sqlite-client")]
pub mod client;

#[cfg(feature = "database-migrations")]
pub mod migrations;

#[cfg(feature = "database-sqlite")]
pub mod sqlite;

#[cfg(feature = "database-blocking")]
pub mod blocking;

/// PostgreSQL execution through the shared engine.
#[cfg(feature = "postgres-client")]
pub mod postgres;
