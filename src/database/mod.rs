//! Optional database contracts. Drivers, schemas and queries remain
//! application-owned.

#[cfg(feature = "database-migrations")]
pub mod migrations;

#[cfg(feature = "database-sqlite")]
pub mod sqlite;
