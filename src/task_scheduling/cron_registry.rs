use super::CronSchedule;
use std::{
    collections::BTreeMap,
    fmt,
    num::NonZeroUsize,
    time::{Duration, SystemTime},
};

/// How an entry advances after an overdue occurrence is delivered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissedTickPolicy {
    /// Deliver the oldest pending occurrence once, then advance beyond `now`.
    #[default]
    Skip,
    /// Deliver historical occurrences in order. Each poll is explicitly bounded.
    CatchUp,
}

/// Initial or replacement settings. Replacement resets timing from the supplied clock.
#[derive(Clone, Debug)]
pub struct CronEntryConfig {
    pub schedule: CronSchedule,
    pub enabled: bool,
    pub missed_ticks: MissedTickPolicy,
}
impl CronEntryConfig {
    pub fn new(schedule: CronSchedule) -> Self {
        Self {
            schedule,
            enabled: true,
            missed_ticks: MissedTickPolicy::Skip,
        }
    }
}

/// Identity of one entry configuration, scoped to this registry instance.
/// Not a durable identifier and not comparable across registries or restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CronRevision(u64);

/// A timing notification, not an execution admission or a durable claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueOccurrence {
    pub id: String,
    pub revision: CronRevision,
    pub scheduled_for: SystemTime,
}

#[derive(Clone, Debug)]
pub struct CronEntrySnapshot {
    pub id: String,
    pub revision: CronRevision,
    pub config: CronEntryConfig,
    /// `None` when disabled or when no future matching date exists.
    pub next_due: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CronRegistryError {
    Closed,
    EmptyId,
    DuplicateId(String),
    UnknownId(String),
    RevisionsExhausted,
}
impl fmt::Display for CronRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cron registry operation rejected: {self:?}")
    }
}
impl std::error::Error for CronRegistryError {}

/// Dynamic UTC cron timing, independent of job execution, limits and persistence.
///
/// The caller owns this registry and serializes mutations, typically by selecting
/// between `next_due()`, its own control channel and shutdown. There is no internal
/// task, lock, command queue or occurrence history. Entry storage is proportional
/// to registrations; applications decide their registration limits.
///
/// Pure operations take an explicit wall-clock sample for deterministic use.
/// All new/enabled/replaced schedules start strictly after that sample. Clock
/// rollback does not rewind delivered ticks; forward jumps follow each entry's
/// missed-tick policy. Separate IDs allow multiple schedules for the same job.
#[derive(Debug, Default)]
pub struct CronRegistry {
    entries: BTreeMap<String, CronEntrySnapshot>,
    revision: u64,
    closed: bool,
}
impl CronRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn check_open(&self) -> Result<(), CronRegistryError> {
        if self.closed {
            Err(CronRegistryError::Closed)
        } else {
            Ok(())
        }
    }

    fn next_revision(&mut self) -> Result<CronRevision, CronRegistryError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(CronRegistryError::RevisionsExhausted)?;
        Ok(CronRevision(self.revision))
    }

    pub fn register(
        &mut self,
        id: impl Into<String>,
        config: CronEntryConfig,
        now: SystemTime,
    ) -> Result<CronRevision, CronRegistryError> {
        self.check_open()?;
        let id = id.into();
        if id.is_empty() {
            return Err(CronRegistryError::EmptyId);
        }
        if self.entries.contains_key(&id) {
            return Err(CronRegistryError::DuplicateId(id));
        }
        let revision = self.next_revision()?;
        self.insert(id, config, now, revision);
        Ok(revision)
    }

    /// Replace an existing entry atomically. Already delivered notifications and
    /// work started by the caller are untouched; their old revision becomes stale.
    pub fn replace(
        &mut self,
        id: &str,
        config: CronEntryConfig,
        now: SystemTime,
    ) -> Result<CronRevision, CronRegistryError> {
        self.check_open()?;
        if !self.entries.contains_key(id) {
            return Err(CronRegistryError::UnknownId(id.into()));
        }
        let revision = self.next_revision()?;
        self.insert(id.into(), config, now, revision);
        Ok(revision)
    }

    fn insert(
        &mut self,
        id: String,
        config: CronEntryConfig,
        now: SystemTime,
        revision: CronRevision,
    ) {
        let next_due = if config.enabled {
            config.schedule.next_after(now)
        } else {
            None
        };
        self.entries.insert(
            id.clone(),
            CronEntrySnapshot {
                id,
                revision,
                config,
                next_due,
            },
        );
    }

    /// Controls automatic ticks only. Repeating the same state preserves the next
    /// deadline and revision. Re-enabling skips the disabled period, even in CatchUp.
    pub fn set_enabled(
        &mut self,
        id: &str,
        enabled: bool,
        now: SystemTime,
    ) -> Result<CronRevision, CronRegistryError> {
        self.check_open()?;
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| CronRegistryError::UnknownId(id.into()))?;
        if entry.config.enabled == enabled {
            return Ok(entry.revision);
        }
        let mut config = entry.config.clone();
        config.enabled = enabled;
        self.replace(id, config, now)
    }

    pub fn remove(&mut self, id: &str) -> Result<CronEntrySnapshot, CronRegistryError> {
        self.check_open()?;
        self.entries
            .remove(id)
            .ok_or_else(|| CronRegistryError::UnknownId(id.into()))
    }

    pub fn get(&self, id: &str) -> Option<&CronEntrySnapshot> {
        self.entries.get(id)
    }

    /// Inspect entries in ID order. Snapshots are configuration/timing views,
    /// not durable checkpoints or execution state.
    pub fn entries(&self) -> impl Iterator<Item = &CronEntrySnapshot> {
        self.entries.values()
    }

    /// Check whether a notification still belongs to an enabled, current entry
    /// of this registry. This does not establish a durable claim or exactly-once execution.
    pub fn is_current(&self, occurrence: &DueOccurrence) -> bool {
        !self.closed
            && self
                .entries
                .get(&occurrence.id)
                .is_some_and(|entry| entry.config.enabled && entry.revision == occurrence.revision)
    }

    pub fn next_deadline(&self) -> Option<SystemTime> {
        if self.closed {
            return None;
        }
        self.entries
            .values()
            .filter_map(|entry| entry.next_due)
            .min()
    }

    /// Deliver up to `limit` occurrences, ordered by deadline then ID, advancing
    /// only delivered entries. Catch-up backlog is represented by a cursor, not
    /// buffered notifications. Cost is O(entries * delivered occurrences).
    ///
    /// Delivery consumes the tick; the caller decides admission, overlap, retries
    /// and persistence. A discarded notification is not automatically redelivered.
    pub fn poll_due(&mut self, now: SystemTime, limit: NonZeroUsize) -> Vec<DueOccurrence> {
        let mut due = Vec::new();
        if self.closed {
            return due;
        }
        while due.len() < limit.get() {
            let next = self
                .entries
                .values()
                .filter_map(|entry| entry.next_due.map(|at| (at, &entry.id)))
                .filter(|(at, _)| *at <= now)
                .min();
            let Some((scheduled_for, id)) = next else {
                break;
            };
            let id = id.clone();
            let entry = self.entries.get_mut(&id).expect("selected entry exists");
            entry.next_due = entry
                .config
                .schedule
                .next_after(match entry.config.missed_ticks {
                    MissedTickPolicy::Skip => now,
                    MissedTickPolicy::CatchUp => scheduled_for,
                });
            due.push(DueOccurrence {
                id,
                revision: entry.revision,
                scheduled_for,
            });
        }
        due
    }

    /// Wait for and consume one tick. Cancellation before returning a tick does
    /// not advance any cursor. UTC is rechecked at least once per second while
    /// there is an active deadline, so wall-clock changes are noticed.
    ///
    /// Empty/disabled/exhausted registries wait until the future is cancelled,
    /// allowing the caller's select loop to register or enable entries. A closed
    /// registry returns `None` immediately. No hidden task is spawned.
    pub async fn next_due(&mut self) -> Option<DueOccurrence> {
        loop {
            if self.closed {
                return None;
            }
            let now = SystemTime::now();
            if let Some(due) = self.poll_due(now, NonZeroUsize::MIN).pop() {
                return Some(due);
            }
            match self.next_deadline() {
                Some(deadline) => {
                    tokio::time::sleep(
                        deadline
                            .duration_since(now)
                            .unwrap_or_default()
                            .min(Duration::from_secs(1)),
                    )
                    .await
                }
                None => std::future::pending::<()>().await,
            }
        }
    }

    /// Irreversibly stop ticks and reject mutations. Retain entries for inspection.
    /// This does not cancel or drain work already started by the application.
    pub fn close(&mut self) {
        self.closed = true;
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}
