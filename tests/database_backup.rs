#![cfg(feature = "database-sqlite-backup")]
use rusqlite::Connection;
use simple_server::database::sqlite::backup::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Debug, PartialEq, Eq)]
enum Error {
    Native(String),
    Injected(Phase),
    Collision,
}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
struct Guard(Arc<AtomicBool>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
struct SqliteAdapter {
    guard: Arc<AtomicBool>,
    fail: Option<Phase>,
    cleanup_fail: bool,
    verification: Verification,
    source: Connection,
    prepare_override: Option<CheckpointObservation>,
    writer_during_copy: bool,
}
impl SqliteAdapter {
    fn new(path: &Path) -> Self {
        let source = Connection::open(path).unwrap();
        source.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT NOT NULL); INSERT INTO items VALUES(1,'before'); PRAGMA user_version=3;").unwrap();
        source.busy_timeout(std::time::Duration::ZERO).unwrap();
        Self {
            guard: Arc::new(AtomicBool::new(false)),
            fail: None,
            cleanup_fail: false,
            verification: Verification {
                integrity: true,
                schema: true,
            },
            source,
            prepare_override: None,
            writer_during_copy: false,
        }
    }
    fn check(&self, phase: Phase) -> Result<(), Error> {
        assert!(self.guard.load(Ordering::SeqCst));
        if self.fail == Some(phase) {
            Err(Error::Injected(phase))
        } else {
            Ok(())
        }
    }
}
impl Adapter for SqliteAdapter {
    type Error = Error;
    type Guard = Guard;
    type Staging = PathBuf;
    fn acquire(&mut self, r: &Request) -> Result<Guard, Error> {
        if self.fail == Some(Phase::Acquire) {
            return Err(Error::Injected(Phase::Acquire));
        }
        // Fixture uses a private directory; production adapters also resolve
        // canonical aliases, parent permissions, symlinks and fencing.
        let source = fs::canonicalize(&r.source).map_err(native)?;
        if r.destination.exists() && fs::canonicalize(&r.destination).map_err(native)? == source {
            return Err(Error::Collision);
        }
        self.guard.store(true, Ordering::SeqCst);
        Ok(Guard(self.guard.clone()))
    }
    fn prepare(&mut self, r: &Request, _: &Guard) -> Result<Option<CheckpointObservation>, Error> {
        self.check(Phase::Prepare)?;
        if let Some(o) = self.prepare_override {
            return Ok(Some(o));
        }
        match r.strategy {
            Strategy::ConsistentCopy => Ok(None),
            Strategy::CheckpointCopy(mode) => self
                .source
                .query_row(mode.statement(), [], |row| {
                    Ok(CheckpointObservation {
                        busy: row.get(0)?,
                        log_pages: row.get(1)?,
                        checkpointed_pages: row.get(2)?,
                    })
                })
                .map(Some)
                .map_err(native),
        }
    }
    fn reserve(&mut self, r: &Request, _: &Guard) -> Result<PathBuf, Error> {
        self.check(Phase::Reserve)?;
        let stage = r.destination.with_extension("staging");
        // Reserve a private directory rather than an empty SQLite file:
        // VACUUM INTO must create the database itself.
        fs::create_dir(&stage).map_err(native)?;
        Ok(stage)
    }
    fn copy(&mut self, r: &Request, _: &Guard, s: &PathBuf) -> Result<(), Error> {
        let output = s.join("copy.db");
        if self.fail == Some(Phase::Copy) {
            fs::write(&output, b"partially copied artifact").map_err(native)?;
        }
        self.check(Phase::Copy)?;
        match r.strategy {
            Strategy::CheckpointCopy(_) => {
                fs::copy(&r.source, &output).map_err(native)?;
            }
            Strategy::ConsistentCopy => {
                if self.writer_during_copy {
                    // A committed WAL change must be included even though it is
                    // absent from the main file when an engine copy starts.
                    let writer = Connection::open(&r.source).map_err(native)?;
                    writer
                        .execute("INSERT INTO items VALUES(2,'wal')", [])
                        .map_err(native)?;
                }
                self.source
                    .execute("VACUUM INTO ?1", [output.to_str().unwrap()])
                    .map_err(native)?;
            }
        }
        Ok(())
    }
    fn verify(&mut self, _: &Request, s: &PathBuf) -> Result<Verification, Error> {
        self.check(Phase::Verify)?;
        let connection = Connection::open(s.join("copy.db")).map_err(native)?;
        let integrity: String = connection
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(native)?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(native)?;
        let rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .map_err(native)?;
        Ok(Verification {
            integrity: self.verification.integrity && integrity == "ok",
            schema: self.verification.schema && version == 3 && rows > 0,
        })
    }
    fn publish(&mut self, r: &Request, s: &PathBuf) -> Result<(), Error> {
        self.check(Phase::Publish)?;
        // Atomic no-replace creation in this same-filesystem private fixture.
        fs::hard_link(s.join("copy.db"), &r.destination).map_err(native)?;
        Ok(())
    }
    fn cleanup(&mut self, s: &PathBuf) -> Result<(), Error> {
        self.check(Phase::Cleanup)?;
        if self.cleanup_fail {
            return Err(Error::Injected(Phase::Cleanup));
        }
        fs::remove_dir_all(s).map_err(native)
    }
}
fn fixture(strategy: Strategy) -> (tempfile::TempDir, SqliteAdapter, Request) {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.db");
    let adapter = SqliteAdapter::new(&source);
    let request = Request {
        source,
        destination: directory.path().join("published.db"),
        strategy,
    };
    (directory, adapter, request)
}
#[test]
fn checkpoint_copy_requires_full_checkpoint_and_verifies_before_publication() {
    let (_dir, mut adapter, request) = fixture(Strategy::CheckpointCopy(CheckpointMode::Truncate));
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.outcome, Outcome::Published);
    assert_eq!(
        report.completed,
        vec![
            Phase::Acquire,
            Phase::Prepare,
            Phase::Reserve,
            Phase::Copy,
            Phase::Verify,
            Phase::Publish,
            Phase::Cleanup
        ]
    );
    assert!(report.checkpoint.unwrap().complete());
    assert!(!report.retained_staging);
    assert!(!adapter.guard.load(Ordering::SeqCst));
    let copied = Connection::open(request.destination).unwrap();
    assert_eq!(
        copied
            .query_row("SELECT value FROM items", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "before"
    );
}
#[test]
fn consistent_copy_includes_committed_wal_and_preserves_reader_snapshot() {
    let (_dir, mut adapter, request) = fixture(Strategy::ConsistentCopy);
    let reader = Connection::open(&request.source).unwrap();
    reader.execute_batch("BEGIN; SELECT * FROM items;").unwrap();
    adapter.writer_during_copy = true;
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.outcome, Outcome::Published);
    assert!(report.checkpoint.is_none());
    assert_eq!(
        reader
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    let copied = Connection::open(request.destination).unwrap();
    assert_eq!(
        copied
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    reader.execute_batch("ROLLBACK").unwrap();
}
#[test]
fn competing_reader_yields_busy_without_creating_output_then_succeeds() {
    let (_dir, mut adapter, request) = fixture(Strategy::CheckpointCopy(CheckpointMode::Truncate));
    let reader = Connection::open(&request.source).unwrap();
    reader.execute_batch("BEGIN; SELECT * FROM items;").unwrap();
    adapter
        .source
        .execute("INSERT INTO items VALUES(2,'after')", [])
        .unwrap();
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.outcome, Outcome::Busy);
    assert_eq!(report.failed_at, Some(Phase::Prepare));
    assert!(!request.destination.exists());
    assert!(!request.destination.with_extension("staging").exists());
    reader.execute_batch("ROLLBACK").unwrap();
    assert_eq!(run(&mut adapter, request).outcome, Outcome::Published);
}
#[test]
fn incomplete_passive_checkpoint_is_busy_and_invalid_nonwal_is_rejected() {
    for (o, outcome, rejection) in [
        (
            CheckpointObservation {
                busy: 0,
                log_pages: 4,
                checkpointed_pages: 2,
            },
            Outcome::Busy,
            None,
        ),
        (
            CheckpointObservation {
                busy: 0,
                log_pages: -1,
                checkpointed_pages: -1,
            },
            Outcome::Failed,
            Some(Rejection::InvalidCheckpoint),
        ),
        (
            CheckpointObservation {
                busy: 0,
                log_pages: 2,
                checkpointed_pages: 3,
            },
            Outcome::Failed,
            Some(Rejection::InvalidCheckpoint),
        ),
    ] {
        let (_dir, mut adapter, request) =
            fixture(Strategy::CheckpointCopy(CheckpointMode::Passive));
        adapter.prepare_override = Some(o);
        let report = run(&mut adapter, request.clone());
        assert_eq!(report.outcome, outcome);
        assert_eq!(report.rejection, rejection);
        assert!(!request.destination.exists());
    }
}
#[test]
fn errors_at_every_phase_preserve_source_and_cleanup_only_owned_staging() {
    for phase in [
        Phase::Acquire,
        Phase::Prepare,
        Phase::Reserve,
        Phase::Copy,
        Phase::Verify,
        Phase::Publish,
    ] {
        let (_dir, mut adapter, request) = fixture(Strategy::ConsistentCopy);
        adapter.fail = Some(phase);
        let report = run(&mut adapter, request.clone());
        assert_eq!(report.outcome, Outcome::Failed);
        assert_eq!(report.failed_at, Some(phase));
        assert_eq!(report.error, Some(Error::Injected(phase)));
        assert!(!report.retained_staging);
        assert!(!request.destination.exists());
        assert!(!request.destination.with_extension("staging").exists());
        assert!(!adapter.guard.load(Ordering::SeqCst));
        assert_eq!(
            adapter
                .source
                .query_row("SELECT COUNT(*) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
#[test]
fn failed_verification_never_publishes_and_cleanup_failure_retains_original() {
    let (_dir, mut adapter, request) = fixture(Strategy::ConsistentCopy);
    adapter.verification.integrity = false;
    let rejected = run(&mut adapter, request.clone());
    assert_eq!(rejected.rejection, Some(Rejection::VerificationFailed));
    assert!(!request.destination.exists());
    assert!(!rejected.retained_staging);
    adapter.verification.integrity = true;
    adapter.verification.schema = false;
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.rejection, Some(Rejection::VerificationFailed));
    assert_eq!(report.failed_at, Some(Phase::Verify));
    assert!(!request.destination.exists());
    assert!(!report.retained_staging);
    adapter.verification.schema = true;
    adapter.fail = Some(Phase::Copy);
    adapter.cleanup_fail = true;
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.error, Some(Error::Injected(Phase::Copy)));
    assert_eq!(report.cleanup_error, Some(Error::Injected(Phase::Cleanup)));
    assert!(report.retained_staging);
    assert!(request.destination.with_extension("staging").exists());
}
#[test]
fn collision_and_source_alias_never_overwrite_or_delete_other_files() {
    let (_dir, mut adapter, request) = fixture(Strategy::ConsistentCopy);
    fs::write(&request.destination, b"existing artifact").unwrap();
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.failed_at, Some(Phase::Publish));
    assert_eq!(
        fs::read(&request.destination).unwrap(),
        b"existing artifact"
    );
    assert!(!report.retained_staging);
    let mut alias = request.clone();
    alias.destination = alias.source.clone();
    let report = run(&mut adapter, alias);
    assert_eq!(report.rejection, Some(Rejection::InvalidDestination));
    let stage = request.destination.with_extension("staging");
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("other"), b"owned by someone else").unwrap();
    assert_eq!(run(&mut adapter, request).failed_at, Some(Phase::Reserve));
    assert!(stage.join("other").exists());
}
#[test]
fn per_file_reports_do_not_hide_partial_batch_failure() {
    let (_a, mut first, request) = fixture(Strategy::ConsistentCopy);
    let (_b, mut second, other) = fixture(Strategy::ConsistentCopy);
    second.fail = Some(Phase::Copy);
    let reports = [run(&mut first, request), run(&mut second, other)];
    assert_eq!(reports[0].outcome, Outcome::Published);
    assert_eq!(reports[1].outcome, Outcome::Failed);
}

#[test]
fn preparation_registry_reports_busy_invalid_and_errors_without_copying() {
    let paths = vec![
        PathBuf::from("ready"),
        PathBuf::from("busy"),
        PathBuf::from("nonwal"),
        PathBuf::from("failed"),
    ];
    let reports = prepare_checkpoints(paths, CheckpointMode::Truncate, |path, mode| {
        assert_eq!(mode.statement(), "PRAGMA wal_checkpoint(TRUNCATE)");
        match path.to_str().unwrap() {
            "ready" => Ok(CheckpointObservation {
                busy: 0,
                log_pages: 0,
                checkpointed_pages: 0,
            }),
            "busy" => Ok(CheckpointObservation {
                busy: 1,
                log_pages: 4,
                checkpointed_pages: 1,
            }),
            "nonwal" => Ok(CheckpointObservation {
                busy: 0,
                log_pages: -1,
                checkpointed_pages: -1,
            }),
            _ => Err(Error::Injected(Phase::Prepare)),
        }
    });
    assert_eq!(
        reports.iter().map(|r| r.status).collect::<Vec<_>>(),
        vec![
            CheckpointStatus::Prepared,
            CheckpointStatus::Busy,
            CheckpointStatus::Invalid,
            CheckpointStatus::Failed
        ]
    );
    assert_eq!(reports[3].error, Some(Error::Injected(Phase::Prepare)));
}
#[test]
fn publication_survives_cleanup_failure_and_reports_owned_staging() {
    let (_dir, mut adapter, request) = fixture(Strategy::ConsistentCopy);
    adapter.cleanup_fail = true;
    let report = run(&mut adapter, request.clone());
    assert_eq!(report.outcome, Outcome::Published);
    assert_eq!(report.failed_at, Some(Phase::Cleanup));
    assert_eq!(report.cleanup_error, Some(Error::Injected(Phase::Cleanup)));
    assert!(request.destination.exists());
    assert!(report.retained_staging);
}

struct AsyncFixture(SqliteAdapter);
impl AsyncAdapter for AsyncFixture {
    type Error = Error;
    type Guard = Guard;
    type Staging = PathBuf;
    async fn acquire(&mut self, r: &Request) -> Result<Guard, Error> {
        tokio::task::yield_now().await;
        Adapter::acquire(&mut self.0, r)
    }
    async fn prepare(
        &mut self,
        r: &Request,
        g: &Guard,
    ) -> Result<Option<CheckpointObservation>, Error> {
        tokio::task::yield_now().await;
        Adapter::prepare(&mut self.0, r, g)
    }
    async fn reserve(&mut self, r: &Request, g: &Guard) -> Result<PathBuf, Error> {
        Adapter::reserve(&mut self.0, r, g)
    }
    async fn copy(&mut self, r: &Request, g: &Guard, s: &PathBuf) -> Result<(), Error> {
        Adapter::copy(&mut self.0, r, g, s)
    }
    async fn verify(&mut self, r: &Request, s: &PathBuf) -> Result<Verification, Error> {
        Adapter::verify(&mut self.0, r, s)
    }
    async fn publish(&mut self, r: &Request, s: &PathBuf) -> Result<(), Error> {
        Adapter::publish(&mut self.0, r, s)
    }
    async fn cleanup(&mut self, s: &PathBuf) -> Result<(), Error> {
        tokio::task::yield_now().await;
        Adapter::cleanup(&mut self.0, s)
    }
}
#[tokio::test]
async fn async_adapter_matches_sync_phase_reports_and_preserves_cleanup() {
    for failure in [
        None,
        Some(Phase::Acquire),
        Some(Phase::Prepare),
        Some(Phase::Reserve),
        Some(Phase::Copy),
        Some(Phase::Verify),
        Some(Phase::Publish),
    ] {
        let (_dir, mut adapter, request) = fixture(Strategy::ConsistentCopy);
        adapter.fail = failure;
        let mut adapter = AsyncFixture(adapter);
        fn require_send(_: impl std::future::Future + Send) {}
        require_send(run_async(&mut adapter, request.clone()));
        let report = run_async(&mut adapter, request.clone()).await;
        assert_eq!(
            report.outcome,
            if failure.is_none() {
                Outcome::Published
            } else {
                Outcome::Failed
            }
        );
        assert_eq!(report.failed_at, failure);
        assert!(!report.retained_staging);
        assert_eq!(request.destination.exists(), failure.is_none());
        assert!(!adapter.0.guard.load(Ordering::SeqCst));
    }
}
