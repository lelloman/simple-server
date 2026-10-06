//! Embedded forward migrations executed by SQLx inside the engine.
use super::{Error, PgPool, command};
use sha2::{Digest, Sha384};
use std::borrow::Cow;
#[derive(Debug)]
pub enum MigrateError {
    Source(Box<dyn std::error::Error + Send + Sync>),
    Database(Error),
}
impl From<Error> for MigrateError {
    fn from(e: Error) -> Self {
        Self::Database(e)
    }
}
impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => e.fmt(f),
            Self::Database(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for MigrateError {}
#[derive(Clone, Copy, Debug)]
pub enum MigrationType {
    Simple,
}
impl MigrationType {
    pub fn is_up_migration(self) -> bool {
        true
    }
}
#[derive(Clone, Debug)]
pub struct Migration {
    pub version: i64,
    pub description: Cow<'static, str>,
    pub migration_type: MigrationType,
    pub sql: Cow<'static, str>,
    pub checksum: Vec<u8>,
    pub no_tx: bool,
}
impl Migration {
    pub fn new(
        version: i64,
        description: Cow<'static, str>,
        migration_type: MigrationType,
        sql: Cow<'static, str>,
        no_tx: bool,
    ) -> Self {
        let checksum = Sha384::digest(sql.as_bytes()).to_vec();
        Self {
            version,
            description,
            migration_type,
            sql,
            checksum,
            no_tx,
        }
    }
}
pub struct Migrator {
    pub migrations: Cow<'static, [Migration]>,
    pub ignore_missing: bool,
    pub locking: bool,
    pub no_tx: bool,
}
impl Migrator {
    /// Load forward-only numbered SQL files, preserving SQLx filename descriptions.
    pub async fn new(path: &std::path::Path) -> Result<Self, MigrateError> {
        let path = path.to_owned();
        let migrations =
            crate::runtime::spawn_blocking(move || -> Result<Vec<Migration>, std::io::Error> {
                let mut migrations = Vec::new();
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if !name.ends_with(".sql") {
                        continue;
                    }
                    let (version, description) = name
                        .trim_end_matches(".sql")
                        .split_once('_')
                        .ok_or_else(|| std::io::Error::other("invalid migration filename"))?;
                    let version = version.parse::<i64>().map_err(std::io::Error::other)?;
                    if description.ends_with(".up") || description.ends_with(".down") {
                        return Err(std::io::Error::other(
                            "only forward migrations are supported",
                        ));
                    }
                    let sql = std::fs::read_to_string(entry.path())?;
                    let no_tx = sql.starts_with("-- no-transaction");
                    migrations.push(Migration::new(
                        version,
                        description.replace('_', " ").into(),
                        MigrationType::Simple,
                        sql.into(),
                        no_tx,
                    ));
                }
                migrations.sort_by_key(|m| m.version);
                Ok(migrations)
            })
            .await
            .map_err(|e| MigrateError::Source(Box::new(std::io::Error::other(e.to_string()))))?
            .map_err(|e| MigrateError::Source(Box::new(e)))?;
        Ok(Self {
            migrations: migrations.into(),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Migration> {
        self.migrations.iter()
    }
    pub async fn run(&self, pool: &PgPool) -> Result<(), MigrateError> {
        let migrations:Vec<_>=self.iter().map(|m|serde_json::json!({"version":m.version,"description":m.description,"sql":m.sql,"no_tx":m.no_tx})).collect();
        command(serde_json::json!({"op":"postgres","action":"migrate","pool":pool.resource.id(),"migrations":migrations,"ignore_missing":self.ignore_missing,"locking":self.locking,"no_tx":self.no_tx}),&[]).await?;
        Ok(())
    }
}
