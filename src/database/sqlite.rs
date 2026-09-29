//! Explicit SQLite connection policy and observed-state verification.
//!
//! A policy has no implicit settings. Apply [`ConnectionPolicy::commands`] on
//! every newly opened connection, using the application's SQLite driver. Read
//! the five corresponding PRAGMAs and pass them to [`ConnectionPolicy::verify`]
//! after a successful readiness query such as `SELECT 1`. The policy does not
//! open connections, own a pool, perform migrations or select a backup method.

use std::{fmt, time::Duration};

/// A SQLite journal mode selected explicitly by the application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalMode {
    Delete,
    Truncate,
    Persist,
    Memory,
    Wal,
    Off,
}

impl JournalMode {
    const fn sql(self) -> &'static str {
        match self {
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
            Self::Persist => "PERSIST",
            Self::Memory => "MEMORY",
            Self::Wal => "WAL",
            Self::Off => "OFF",
        }
    }
}

/// SQLite's synchronous setting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Synchronous {
    Off,
    Normal,
    Full,
    Extra,
}

impl Synchronous {
    const fn value(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Normal => 1,
            Self::Full => 2,
            Self::Extra => 3,
        }
    }
}

/// Only explicitly supplied settings are applied or verified.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConnectionPolicy {
    foreign_keys: Option<bool>,
    busy_timeout: Option<Duration>,
    journal_mode: Option<JournalMode>,
    synchronous: Option<Synchronous>,
    wal_autocheckpoint: Option<u32>,
}

impl ConnectionPolicy {
    pub const fn new() -> Self {
        Self {
            foreign_keys: None,
            busy_timeout: None,
            journal_mode: None,
            synchronous: None,
            wal_autocheckpoint: None,
        }
    }

    pub const fn foreign_keys(mut self, enabled: bool) -> Self {
        self.foreign_keys = Some(enabled);
        self
    }

    pub const fn busy_timeout(mut self, timeout: Duration) -> Self {
        self.busy_timeout = Some(timeout);
        self
    }

    pub const fn journal_mode(mut self, mode: JournalMode) -> Self {
        self.journal_mode = Some(mode);
        self
    }

    pub const fn synchronous(mut self, mode: Synchronous) -> Self {
        self.synchronous = Some(mode);
        self
    }

    pub const fn wal_autocheckpoint(mut self, pages: u32) -> Self {
        self.wal_autocheckpoint = Some(pages);
        self
    }

    /// Return SQL to execute on each connection, in policy order. No statement
    /// is emitted for an unspecified setting.
    pub fn commands(&self) -> Result<Vec<String>, PolicyError> {
        let mut commands = Vec::new();
        if let Some(mode) = self.journal_mode {
            commands.push(format!("PRAGMA journal_mode = {}", mode.sql()));
        }
        if let Some(enabled) = self.foreign_keys {
            commands.push(format!("PRAGMA foreign_keys = {}", u8::from(enabled)));
        }
        if let Some(timeout) = self.busy_timeout {
            let milliseconds = timeout.as_millis();
            if milliseconds > i32::MAX as u128 {
                return Err(PolicyError::BusyTimeoutTooLarge);
            }
            commands.push(format!("PRAGMA busy_timeout = {milliseconds}"));
        }
        if let Some(mode) = self.synchronous {
            commands.push(format!("PRAGMA synchronous = {}", mode.value()));
        }
        if let Some(pages) = self.wal_autocheckpoint {
            if pages > i32::MAX as u32 {
                return Err(PolicyError::WalAutocheckpointTooLarge);
            }
            commands.push(format!("PRAGMA wal_autocheckpoint = {pages}"));
        }
        Ok(commands)
    }

    /// Check observed settings from one live connection. Callers must run an
    /// independent query to establish that the connection is usable.
    pub fn verify(&self, observed: &ConnectionObservation) -> Result<(), PolicyError> {
        if let Some(expected) = self.foreign_keys {
            check("foreign_keys", expected, observed.foreign_keys)?;
        }
        if let Some(expected) = self.busy_timeout {
            check(
                "busy_timeout",
                expected.as_millis(),
                observed.busy_timeout.as_millis(),
            )?;
        }
        if let Some(expected) = self.journal_mode {
            let actual = observed.journal_mode.as_str();
            if !actual.eq_ignore_ascii_case(expected.sql()) {
                return Err(PolicyError::Mismatch {
                    setting: "journal_mode",
                    expected: expected.sql().into(),
                    actual: actual.into(),
                });
            }
        }
        if let Some(expected) = self.synchronous {
            check("synchronous", expected.value(), observed.synchronous)?;
        }
        if let Some(expected) = self.wal_autocheckpoint {
            check("wal_autocheckpoint", expected, observed.wal_autocheckpoint)?;
        }
        Ok(())
    }
}

fn check<T: fmt::Display + PartialEq>(
    setting: &'static str,
    expected: T,
    actual: T,
) -> Result<(), PolicyError> {
    if expected == actual {
        Ok(())
    } else {
        Err(PolicyError::Mismatch {
            setting,
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

/// Values read from one connection after it has been configured.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionObservation {
    pub foreign_keys: bool,
    pub busy_timeout: Duration,
    pub journal_mode: String,
    pub synchronous: u8,
    pub wal_autocheckpoint: u32,
}

/// A policy value is invalid or a live connection disagrees with it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    BusyTimeoutTooLarge,
    WalAutocheckpointTooLarge,
    Mismatch {
        setting: &'static str,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BusyTimeoutTooLarge => write!(f, "SQLite busy timeout exceeds i32 milliseconds"),
            Self::WalAutocheckpointTooLarge => {
                write!(f, "SQLite WAL auto-checkpoint interval exceeds i32 pages")
            }
            Self::Mismatch {
                setting,
                expected,
                actual,
            } => write!(f, "SQLite {setting} is {actual}, expected {expected}"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Optional driver-neutral schema descriptions, creation plans and validation.
#[cfg(feature = "database-sqlite-schema")]
pub mod schema;
