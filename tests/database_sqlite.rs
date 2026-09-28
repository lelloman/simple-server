#![cfg(feature = "database-sqlite")]

use std::time::Duration;

use rusqlite::Connection;
use simple_server::database::sqlite::{
    ConnectionObservation, ConnectionPolicy, JournalMode, PolicyError, Synchronous,
};

fn observe(conn: &Connection) -> ConnectionObservation {
    ConnectionObservation {
        foreign_keys: conn
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
            .unwrap()
            != 0,
        busy_timeout: Duration::from_millis(
            conn.pragma_query_value(None, "busy_timeout", |row| row.get::<_, u64>(0))
                .unwrap(),
        ),
        journal_mode: conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap(),
        synchronous: conn
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap(),
        wal_autocheckpoint: conn
            .pragma_query_value(None, "wal_autocheckpoint", |row| row.get(0))
            .unwrap(),
    }
}

#[test]
fn policy_applies_and_verifies_only_selected_settings() {
    let conn = Connection::open_in_memory().unwrap();
    let policy = ConnectionPolicy::new()
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(2))
        .synchronous(Synchronous::Normal)
        .wal_autocheckpoint(0);

    for command in policy.commands().unwrap() {
        conn.execute_batch(&command).unwrap();
    }
    assert_eq!(
        conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    policy.verify(&observe(&conn)).unwrap();
    assert!(
        policy
            .commands()
            .unwrap()
            .iter()
            .all(|sql| !sql.contains("journal_mode"))
    );

    conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
    assert_eq!(
        policy.verify(&observe(&conn)),
        Err(PolicyError::Mismatch {
            setting: "foreign_keys",
            expected: "true".into(),
            actual: "false".into(),
        })
    );
}

#[test]
fn journal_mode_is_checked_against_effective_mode() {
    let conn = Connection::open_in_memory().unwrap();
    let policy = ConnectionPolicy::new().journal_mode(JournalMode::Wal);
    for command in policy.commands().unwrap() {
        conn.execute_batch(&command).unwrap();
    }
    assert!(matches!(
        policy.verify(&observe(&conn)),
        Err(PolicyError::Mismatch {
            setting: "journal_mode",
            ..
        })
    ));
}

#[test]
fn oversized_timeout_is_rejected_and_empty_policy_is_noop() {
    assert!(ConnectionPolicy::new().commands().unwrap().is_empty());
    let policy = ConnectionPolicy::new().busy_timeout(Duration::MAX);
    assert_eq!(policy.commands(), Err(PolicyError::BusyTimeoutTooLarge));
    let policy = ConnectionPolicy::new().wal_autocheckpoint(u32::MAX);
    assert_eq!(
        policy.commands(),
        Err(PolicyError::WalAutocheckpointTooLarge)
    );
}
