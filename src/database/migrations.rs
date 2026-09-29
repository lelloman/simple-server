//! Read-only preflight of an application's migration manifest against its ledger.
//!
//! This module never opens a database, runs SQL, creates a ledger, or declares
//! that a schema is healthy. The application owns those operations and supplies
//! the ordered ledger records, including its historical digest algorithm.

use std::error::Error;
use std::fmt;

/// A stable migration version. Text versions use lexicographic order, as with
/// sorted migration filenames; numeric versions use numeric order. A plan and
/// its observation must use one kind throughout.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MigrationVersion {
    Number(u64),
    Text(String),
}

impl From<u64> for MigrationVersion {
    fn from(value: u64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for MigrationVersion {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for MigrationVersion {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

/// A manifest or ledger identity. Digests are opaque: the application supplies
/// their original encoding and calculation, without normalization by this crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationEntry {
    pub version: MigrationVersion,
    pub name: String,
    pub digest: Option<String>,
}

/// The application's ordered, immutable manifest for one database namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationPlan {
    pub namespace: String,
    pub entries: Vec<MigrationEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionState {
    Applied,
    Dirty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordedMigration {
    pub entry: MigrationEntry,
    pub state: CompletionState,
}

/// Existing ledger observations, in the ledger's version order. Map an empty
/// ledger's sentinel high-water value (for example SQLite `user_version = 0`)
/// to `None`. When supplied, a high-water value must match the last entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationObservation {
    pub namespace: String,
    pub entries: Vec<RecordedMigration>,
    pub high_water_mark: Option<MigrationVersion>,
}

/// A report only: the caller must still validate schema shape and execute its
/// own migration runner. `verified_digests` counts historical digest comparisons,
/// and remains zero for version-only ledgers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReport {
    pub namespace: String,
    pub applied: Vec<MigrationEntry>,
    pub pending: Vec<MigrationEntry>,
    pub verified_digests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationError {
    EmptyNamespace,
    NamespaceMismatch {
        expected: String,
        actual: String,
    },
    EmptyVersion,
    EmptyName {
        version: MigrationVersion,
    },
    EmptyDigest {
        version: MigrationVersion,
    },
    MixedVersionKinds,
    ManifestOrder {
        previous: MigrationVersion,
        current: MigrationVersion,
    },
    LedgerOrder {
        previous: MigrationVersion,
        current: MigrationVersion,
    },
    Dirty {
        version: MigrationVersion,
    },
    NewerDatabaseVersion {
        version: MigrationVersion,
    },
    UnknownRecordedVersion {
        version: MigrationVersion,
    },
    HistoryGap {
        expected: MigrationVersion,
        actual: MigrationVersion,
    },
    NameMismatch {
        version: MigrationVersion,
        expected: String,
        actual: String,
    },
    DigestAvailabilityMismatch {
        version: MigrationVersion,
    },
    DigestMismatch {
        version: MigrationVersion,
        expected: String,
        actual: String,
    },
    HighWaterMismatch {
        expected: Option<MigrationVersion>,
        actual: Option<MigrationVersion>,
    },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "migration preflight failed: {self:?}")
    }
}

impl Error for MigrationError {}

impl MigrationPlan {
    /// Inspect a store that records only its latest applied version, without a
    /// per-migration ledger. The caller must map an empty-store sentinel to
    /// `None` and retain its own schema-shape and legacy-layout checks.
    ///
    /// Entries through the marker are reported as applied, but their names and
    /// digests cannot be historically verified. `verified_digests` is always
    /// zero, even when the manifest contains digests.
    pub fn inspect_version_only(
        &self,
        applied_version: Option<&MigrationVersion>,
    ) -> Result<MigrationReport, MigrationError> {
        if self.namespace.trim().is_empty() {
            return Err(MigrationError::EmptyNamespace);
        }
        validate_manifest(&self.entries)?;

        let applied_count = match applied_version {
            None => 0,
            Some(version) => {
                if let Some(first) = self.entries.first()
                    && !same_kind(version, &first.version)
                {
                    return Err(MigrationError::MixedVersionKinds);
                }
                self.entries
                    .iter()
                    .position(|entry| &entry.version == version)
                    .map(|index| index + 1)
                    .ok_or_else(|| classify_unexpected(version, self.entries.last()))?
            }
        };
        Ok(MigrationReport {
            namespace: self.namespace.clone(),
            applied: self.entries[..applied_count].to_vec(),
            pending: self.entries[applied_count..].to_vec(),
            verified_digests: 0,
        })
    }

    /// Compare this manifest with existing ledger observations without mutation.
    pub fn inspect(
        &self,
        observed: &MigrationObservation,
    ) -> Result<MigrationReport, MigrationError> {
        if self.namespace.trim().is_empty() || observed.namespace.trim().is_empty() {
            return Err(MigrationError::EmptyNamespace);
        }
        if self.namespace != observed.namespace {
            return Err(MigrationError::NamespaceMismatch {
                expected: self.namespace.clone(),
                actual: observed.namespace.clone(),
            });
        }
        validate_manifest(&self.entries)?;
        validate_ledger(&observed.entries)?;

        let mut verified_digests = 0;
        for (index, record) in observed.entries.iter().enumerate() {
            let version = &record.entry.version;
            let Some(expected) = self.entries.get(index) else {
                return Err(classify_unexpected(version, self.entries.last()));
            };
            if !same_kind(version, &expected.version) {
                return Err(MigrationError::MixedVersionKinds);
            }
            if *version != expected.version {
                if self
                    .entries
                    .iter()
                    .skip(index + 1)
                    .any(|entry| entry.version == *version)
                {
                    return Err(MigrationError::HistoryGap {
                        expected: expected.version.clone(),
                        actual: version.clone(),
                    });
                }
                return Err(classify_unexpected(version, self.entries.last()));
            }
            if record.entry.name != expected.name {
                return Err(MigrationError::NameMismatch {
                    version: version.clone(),
                    expected: expected.name.clone(),
                    actual: record.entry.name.clone(),
                });
            }
            match (&expected.digest, &record.entry.digest) {
                (Some(expected), Some(actual)) if expected != actual => {
                    return Err(MigrationError::DigestMismatch {
                        version: version.clone(),
                        expected: expected.clone(),
                        actual: actual.clone(),
                    });
                }
                (Some(_), Some(_)) => verified_digests += 1,
                (None, None) => {}
                _ => {
                    return Err(MigrationError::DigestAvailabilityMismatch {
                        version: version.clone(),
                    });
                }
            }
        }
        let high_water_expected = observed
            .entries
            .last()
            .map(|record| record.entry.version.clone());
        if observed.high_water_mark.is_some() && observed.high_water_mark != high_water_expected {
            return Err(MigrationError::HighWaterMismatch {
                expected: high_water_expected,
                actual: observed.high_water_mark.clone(),
            });
        }
        Ok(MigrationReport {
            namespace: self.namespace.clone(),
            applied: self.entries[..observed.entries.len()].to_vec(),
            pending: self.entries[observed.entries.len()..].to_vec(),
            verified_digests,
        })
    }
}

fn validate_entry(entry: &MigrationEntry) -> Result<(), MigrationError> {
    if matches!(&entry.version, MigrationVersion::Text(value) if value.trim().is_empty()) {
        return Err(MigrationError::EmptyVersion);
    }
    if entry.name.trim().is_empty() {
        return Err(MigrationError::EmptyName {
            version: entry.version.clone(),
        });
    }
    if matches!(&entry.digest, Some(value) if value.trim().is_empty()) {
        return Err(MigrationError::EmptyDigest {
            version: entry.version.clone(),
        });
    }
    Ok(())
}

fn same_kind(left: &MigrationVersion, right: &MigrationVersion) -> bool {
    matches!(
        (left, right),
        (MigrationVersion::Number(_), MigrationVersion::Number(_))
            | (MigrationVersion::Text(_), MigrationVersion::Text(_))
    )
}

fn validate_manifest(entries: &[MigrationEntry]) -> Result<(), MigrationError> {
    for (index, entry) in entries.iter().enumerate() {
        validate_entry(entry)?;
        if let Some(previous) = index.checked_sub(1).and_then(|i| entries.get(i)) {
            if !same_kind(&previous.version, &entry.version) {
                return Err(MigrationError::MixedVersionKinds);
            }
            if previous.version >= entry.version {
                return Err(MigrationError::ManifestOrder {
                    previous: previous.version.clone(),
                    current: entry.version.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_ledger(entries: &[RecordedMigration]) -> Result<(), MigrationError> {
    for (index, record) in entries.iter().enumerate() {
        validate_entry(&record.entry)?;
        if let Some(previous) = index.checked_sub(1).and_then(|i| entries.get(i)) {
            if !same_kind(&previous.entry.version, &record.entry.version) {
                return Err(MigrationError::MixedVersionKinds);
            }
            if previous.entry.version >= record.entry.version {
                return Err(MigrationError::LedgerOrder {
                    previous: previous.entry.version.clone(),
                    current: record.entry.version.clone(),
                });
            }
        }
        if record.state == CompletionState::Dirty {
            return Err(MigrationError::Dirty {
                version: record.entry.version.clone(),
            });
        }
    }
    Ok(())
}

fn classify_unexpected(
    version: &MigrationVersion,
    last: Option<&MigrationEntry>,
) -> MigrationError {
    if last.is_none_or(|entry| same_kind(version, &entry.version) && *version > entry.version) {
        MigrationError::NewerDatabaseVersion {
            version: version.clone(),
        }
    } else {
        MigrationError::UnknownRecordedVersion {
            version: version.clone(),
        }
    }
}
