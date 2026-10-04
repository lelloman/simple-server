//! Driver-neutral backup phase coordination. Adapters own locking, consistent
//! copying, exclusive staging, verification and atomic no-replace publication.
//! This module performs no filesystem operations and never restores a database.
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointMode {
    Passive,
    Full,
    Restart,
    Truncate,
}
impl CheckpointMode {
    pub fn statement(self) -> &'static str {
        match self {
            Self::Passive => "PRAGMA wal_checkpoint(PASSIVE)",
            Self::Full => "PRAGMA wal_checkpoint(FULL)",
            Self::Restart => "PRAGMA wal_checkpoint(RESTART)",
            Self::Truncate => "PRAGMA wal_checkpoint(TRUNCATE)",
        }
    }
}

/// SQLite's three checkpoint result columns, including -1 for a non-WAL file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointObservation {
    pub busy: i64,
    pub log_pages: i64,
    pub checkpointed_pages: i64,
}
impl CheckpointObservation {
    /// A partial passive checkpoint is not sufficient preparation for file copy.
    /// Even a complete checkpoint still requires coordination through copying.
    pub fn complete(self) -> bool {
        self.status() == CheckpointStatus::Prepared
    }
    pub fn status(self) -> CheckpointStatus {
        if self.busy < 0
            || self.log_pages < 0
            || self.checkpointed_pages < 0
            || self.checkpointed_pages > self.log_pages
        {
            CheckpointStatus::Invalid
        } else if self.busy > 0 || self.checkpointed_pages < self.log_pages {
            CheckpointStatus::Busy
        } else {
            CheckpointStatus::Prepared
        }
    }
}

/// Preparation is not a backup and does not authorize an uncoordinated copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointStatus {
    Prepared,
    Busy,
    Invalid,
    Failed,
}
#[derive(Debug)]
pub struct CheckpointReport<E> {
    pub source: PathBuf,
    pub mode: CheckpointMode,
    pub status: CheckpointStatus,
    pub observation: Option<CheckpointObservation>,
    pub error: Option<E>,
}
/// Reports each registry entry independently, retaining errors and page counts.
/// Duplicate entries are retained; no global snapshot/locking is implied.
pub fn prepare_checkpoints<E>(
    sources: impl IntoIterator<Item = PathBuf>,
    mode: CheckpointMode,
    mut checkpoint: impl FnMut(&Path, CheckpointMode) -> Result<CheckpointObservation, E>,
) -> Vec<CheckpointReport<E>> {
    sources
        .into_iter()
        .map(|source| {
            let (status, observation, error) = match checkpoint(&source, mode) {
                Ok(observation) => (observation.status(), Some(observation), None),
                Err(error) => (CheckpointStatus::Failed, None, Some(error)),
            };
            CheckpointReport {
                source,
                mode,
                status,
                observation,
                error,
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /// Adapter must prevent writes/checkpoints racing with the raw file copy.
    CheckpointCopy(CheckpointMode),
    /// Adapter uses an engine-consistent copy, e.g. SQLite backup or VACUUM INTO.
    ConsistentCopy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub strategy: Strategy,
}

/// Explicit verification scope. Both checks are required for publication.
/// An adapter supplies its application-specific expected schema/marker checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verification {
    pub integrity: bool,
    pub schema: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Acquire,
    Prepare,
    Reserve,
    Copy,
    Verify,
    Publish,
    Cleanup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Published,
    Busy,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    InvalidDestination,
    InvalidCheckpoint,
    VerificationFailed,
}

/// Native adapter errors are retained without string conversion. Cleanup failure
/// is separate from the original failure; retained staging must be handled by
/// the application. This is a per-file report, never a cross-file snapshot claim.
#[derive(Debug)]
pub struct Report<E> {
    pub request: Request,
    pub outcome: Outcome,
    pub completed: Vec<Phase>,
    pub failed_at: Option<Phase>,
    pub checkpoint: Option<CheckpointObservation>,
    pub verification: Option<Verification>,
    pub rejection: Option<Rejection>,
    pub error: Option<E>,
    pub cleanup_error: Option<E>,
    pub retained_staging: bool,
}

/// Safety contract of an adapter (no native database driver is required here):
/// - acquire checks canonical source/destination aliases and holds appropriate
///   ownership/fencing until publication or cleanup finishes;
/// - reserve creates *new*, owned staging, never truncates another artifact;
///   an error must leave no owned output, or clean it before returning;
/// - copy implements the requested consistency strategy;
/// - publish is atomic and refuses to replace an existing destination. On error
///   staging remains owned and destination was not published. On success staging
///   remains owned until cleanup; cleanup failure cannot undo publication;
/// - cleanup only removes this adapter's staging, never source/published output.
///
/// Destination permissions, retention, encryption and restore stay local.
pub trait Adapter {
    type Error;
    type Guard;
    type Staging;
    fn acquire(&mut self, request: &Request) -> Result<Self::Guard, Self::Error>;
    fn prepare(
        &mut self,
        request: &Request,
        guard: &Self::Guard,
    ) -> Result<Option<CheckpointObservation>, Self::Error>;
    fn reserve(
        &mut self,
        request: &Request,
        guard: &Self::Guard,
    ) -> Result<Self::Staging, Self::Error>;
    fn copy(
        &mut self,
        request: &Request,
        guard: &Self::Guard,
        staging: &Self::Staging,
    ) -> Result<(), Self::Error>;
    fn verify(
        &mut self,
        request: &Request,
        staging: &Self::Staging,
    ) -> Result<Verification, Self::Error>;
    fn publish(&mut self, request: &Request, staging: &Self::Staging) -> Result<(), Self::Error>;
    fn cleanup(&mut self, staging: &Self::Staging) -> Result<(), Self::Error>;
}

/// Synchronous coordination; use a bounded blocking executor from async callers.
/// No retries occur, including after a partially completed copy or publish error.
/// The owned guard stays alive for all phases, including failure cleanup.
pub fn run<A: Adapter>(adapter: &mut A, request: Request) -> Report<A::Error> {
    let mut report = Report {
        request,
        outcome: Outcome::Failed,
        completed: Vec::new(),
        failed_at: None,
        checkpoint: None,
        verification: None,
        rejection: None,
        error: None,
        cleanup_error: None,
        retained_staging: false,
    };
    if report.request.source == report.request.destination
        || report.request.destination.as_os_str().is_empty()
    {
        report.failed_at = Some(Phase::Acquire);
        report.rejection = Some(Rejection::InvalidDestination);
        return report;
    }
    let guard = match adapter.acquire(&report.request) {
        Ok(guard) => {
            report.completed.push(Phase::Acquire);
            guard
        }
        Err(error) => {
            report.failed_at = Some(Phase::Acquire);
            report.error = Some(error);
            return report;
        }
    };
    match adapter.prepare(&report.request, &guard) {
        Err(error) => {
            report.failed_at = Some(Phase::Prepare);
            report.error = Some(error);
            return report;
        }
        Ok(observation) => {
            report.checkpoint = observation;
            match (report.request.strategy, observation) {
                (Strategy::CheckpointCopy(_), Some(checkpoint)) if checkpoint.complete() => {}
                (Strategy::CheckpointCopy(_), Some(checkpoint))
                    if checkpoint.status() == CheckpointStatus::Busy =>
                {
                    report.outcome = Outcome::Busy;
                    report.failed_at = Some(Phase::Prepare);
                    return report;
                }
                (Strategy::ConsistentCopy, None) => {}
                _ => {
                    report.rejection = Some(Rejection::InvalidCheckpoint);
                    report.failed_at = Some(Phase::Prepare);
                    return report;
                }
            }
            report.completed.push(Phase::Prepare);
        }
    }
    let staging = match adapter.reserve(&report.request, &guard) {
        Ok(staging) => {
            report.completed.push(Phase::Reserve);
            staging
        }
        Err(error) => {
            report.failed_at = Some(Phase::Reserve);
            report.error = Some(error);
            return report;
        }
    };
    report.retained_staging = true;
    let result = (|| {
        adapter
            .copy(&report.request, &guard, &staging)
            .map_err(|e| (Phase::Copy, e))?;
        report.completed.push(Phase::Copy);
        let verification = adapter
            .verify(&report.request, &staging)
            .map_err(|e| (Phase::Verify, e))?;
        report.verification = Some(verification);
        if !verification.integrity || !verification.schema {
            report.rejection = Some(Rejection::VerificationFailed);
            report.failed_at = Some(Phase::Verify);
            return Ok(false);
        }
        report.completed.push(Phase::Verify);
        adapter
            .publish(&report.request, &staging)
            .map_err(|e| (Phase::Publish, e))?;
        report.completed.push(Phase::Publish);
        Ok(true)
    })();
    match result {
        Ok(true) => {
            report.outcome = Outcome::Published;
            match adapter.cleanup(&staging) {
                Ok(()) => {
                    report.completed.push(Phase::Cleanup);
                    report.retained_staging = false;
                }
                Err(error) => {
                    report.failed_at = Some(Phase::Cleanup);
                    report.cleanup_error = Some(error);
                }
            }
        }
        result => {
            if let Err((phase, error)) = result {
                report.failed_at = Some(phase);
                report.error = Some(error);
            }
            match adapter.cleanup(&staging) {
                Ok(()) => {
                    report.completed.push(Phase::Cleanup);
                    report.retained_staging = false;
                }
                Err(error) => report.cleanup_error = Some(error),
            }
        }
    }
    report
}

/// Async driver adapter with the same ownership and publication contract as
/// Adapter. SQLx and other async drivers can keep their native async operations.
/// Futures are Send so the coordinator can run on a multithreaded runtime.
pub trait AsyncAdapter: Send {
    type Error: Send;
    type Guard: Send + Sync;
    type Staging: Send + Sync;
    fn acquire(
        &mut self,
        request: &Request,
    ) -> impl std::future::Future<Output = Result<Self::Guard, Self::Error>> + Send;
    fn prepare(
        &mut self,
        request: &Request,
        guard: &Self::Guard,
    ) -> impl std::future::Future<Output = Result<Option<CheckpointObservation>, Self::Error>> + Send;
    fn reserve(
        &mut self,
        request: &Request,
        guard: &Self::Guard,
    ) -> impl std::future::Future<Output = Result<Self::Staging, Self::Error>> + Send;
    fn copy(
        &mut self,
        request: &Request,
        guard: &Self::Guard,
        staging: &Self::Staging,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
    fn verify(
        &mut self,
        request: &Request,
        staging: &Self::Staging,
    ) -> impl std::future::Future<Output = Result<Verification, Self::Error>> + Send;
    fn publish(
        &mut self,
        request: &Request,
        staging: &Self::Staging,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
    fn cleanup(
        &mut self,
        staging: &Self::Staging,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

/// Async coordination. Do not abandon this future after reserve: cancellation
/// may leave owned staging and a partially finished native operation. Run it in
/// an application-owned task and await it through shutdown. No detached task is
/// created here; restore and retry remain explicit application decisions.
pub async fn run_async<A: AsyncAdapter>(adapter: &mut A, request: Request) -> Report<A::Error> {
    let mut report = Report {
        request,
        outcome: Outcome::Failed,
        completed: Vec::new(),
        failed_at: None,
        checkpoint: None,
        verification: None,
        rejection: None,
        error: None,
        cleanup_error: None,
        retained_staging: false,
    };
    if report.request.source == report.request.destination
        || report.request.destination.as_os_str().is_empty()
    {
        report.failed_at = Some(Phase::Acquire);
        report.rejection = Some(Rejection::InvalidDestination);
        return report;
    }
    let guard = match adapter.acquire(&report.request).await {
        Ok(guard) => {
            report.completed.push(Phase::Acquire);
            guard
        }
        Err(error) => {
            report.failed_at = Some(Phase::Acquire);
            report.error = Some(error);
            return report;
        }
    };
    match adapter.prepare(&report.request, &guard).await {
        Err(error) => {
            report.failed_at = Some(Phase::Prepare);
            report.error = Some(error);
            return report;
        }
        Ok(observation) => {
            report.checkpoint = observation;
            match (report.request.strategy, observation) {
                (Strategy::CheckpointCopy(_), Some(checkpoint)) if checkpoint.complete() => {}
                (Strategy::CheckpointCopy(_), Some(checkpoint))
                    if checkpoint.status() == CheckpointStatus::Busy =>
                {
                    report.outcome = Outcome::Busy;
                    report.failed_at = Some(Phase::Prepare);
                    return report;
                }
                (Strategy::ConsistentCopy, None) => {}
                _ => {
                    report.rejection = Some(Rejection::InvalidCheckpoint);
                    report.failed_at = Some(Phase::Prepare);
                    return report;
                }
            }
            report.completed.push(Phase::Prepare);
        }
    }
    let staging = match adapter.reserve(&report.request, &guard).await {
        Ok(staging) => {
            report.completed.push(Phase::Reserve);
            staging
        }
        Err(error) => {
            report.failed_at = Some(Phase::Reserve);
            report.error = Some(error);
            return report;
        }
    };
    report.retained_staging = true;
    let result = async {
        adapter
            .copy(&report.request, &guard, &staging)
            .await
            .map_err(|e| (Phase::Copy, e))?;
        report.completed.push(Phase::Copy);
        let verification = adapter
            .verify(&report.request, &staging)
            .await
            .map_err(|e| (Phase::Verify, e))?;
        report.verification = Some(verification);
        if !verification.integrity || !verification.schema {
            report.rejection = Some(Rejection::VerificationFailed);
            report.failed_at = Some(Phase::Verify);
            return Ok(false);
        }
        report.completed.push(Phase::Verify);
        adapter
            .publish(&report.request, &staging)
            .await
            .map_err(|e| (Phase::Publish, e))?;
        report.completed.push(Phase::Publish);
        Ok(true)
    }
    .await;
    match result {
        Ok(true) => {
            report.outcome = Outcome::Published;
            match adapter.cleanup(&staging).await {
                Ok(()) => {
                    report.completed.push(Phase::Cleanup);
                    report.retained_staging = false;
                }
                Err(error) => {
                    report.failed_at = Some(Phase::Cleanup);
                    report.cleanup_error = Some(error);
                }
            }
        }
        result => {
            if let Err((phase, error)) = result {
                report.failed_at = Some(phase);
                report.error = Some(error);
            }
            match adapter.cleanup(&staging).await {
                Ok(()) => {
                    report.completed.push(Phase::Cleanup);
                    report.retained_staging = false;
                }
                Err(error) => report.cleanup_error = Some(error),
            }
        }
    }
    report
}
