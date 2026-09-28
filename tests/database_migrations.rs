#![cfg(feature = "database-migrations")]

use simple_server::database::migrations::{
    CompletionState, MigrationEntry, MigrationError, MigrationObservation, MigrationPlan,
    MigrationVersion, RecordedMigration,
};

fn entry(version: u64, name: &str, digest: Option<&str>) -> MigrationEntry {
    MigrationEntry {
        version: MigrationVersion::Number(version),
        name: name.into(),
        digest: digest.map(str::to_owned),
    }
}

fn sample_manifest() -> MigrationPlan {
    MigrationPlan {
        namespace: "service/catalog".into(),
        entries: vec![
            entry(1, "initial", Some("sha:aaa")),
            entry(2, "users", Some("sha:bbb")),
            entry(3, "index", Some("sha:ccc")),
        ],
    }
}

fn record(entry: MigrationEntry) -> RecordedMigration {
    RecordedMigration {
        entry,
        state: CompletionState::Applied,
    }
}

fn observation(entries: Vec<RecordedMigration>) -> MigrationObservation {
    MigrationObservation {
        namespace: "service/catalog".into(),
        high_water_mark: entries.last().map(|row| row.entry.version.clone()),
        entries,
    }
}

#[test]
fn fresh_partial_and_fully_applied_histories_are_read_only_reports() {
    let plan = sample_manifest();
    let fresh = plan.inspect(&observation(vec![])).unwrap();
    assert!(fresh.applied.is_empty());
    assert_eq!(fresh.pending, plan.entries);
    assert_eq!(fresh.verified_digests, 0);

    let partially_applied = observation(vec![record(plan.entries[0].clone())]);
    let report = plan.inspect(&partially_applied).unwrap();
    assert_eq!(report.applied, plan.entries[..1]);
    assert_eq!(report.pending, plan.entries[1..]);
    assert_eq!(report.verified_digests, 1);
    assert_eq!(partially_applied.entries.len(), 1);

    let complete = observation(plan.entries.iter().cloned().map(record).collect());
    let report = plan.inspect(&complete).unwrap();
    assert_eq!(report.applied, plan.entries);
    assert!(report.pending.is_empty());
    assert_eq!(report.verified_digests, 3);
}

#[test]
fn version_only_ledger_does_not_claim_checksum_verification() {
    let mut plan = sample_manifest();
    for entry in &mut plan.entries {
        entry.digest = None;
    }
    let report = plan
        .inspect(&observation(vec![record(plan.entries[0].clone())]))
        .unwrap();
    assert_eq!(report.verified_digests, 0);

    let mut with_new_digest = plan.clone();
    with_new_digest.entries[0].digest = Some("newly-computed".into());
    assert_eq!(
        with_new_digest.inspect(&observation(vec![record(plan.entries[0].clone())])),
        Err(MigrationError::DigestAvailabilityMismatch { version: 1.into() })
    );
}

#[test]
fn text_versions_use_the_applications_sorted_filename_order() {
    let plan = MigrationPlan {
        namespace: "favzetto".into(),
        entries: vec![
            MigrationEntry {
                version: "001_init".into(),
                name: "001_init".into(),
                digest: None,
            },
            MigrationEntry {
                version: "002_users".into(),
                name: "002_users".into(),
                digest: None,
            },
        ],
    };
    let ledger = MigrationObservation {
        namespace: "favzetto".into(),
        entries: vec![record(plan.entries[0].clone())],
        high_water_mark: None,
    };
    let report = plan.inspect(&ledger).unwrap();
    assert_eq!(report.pending, plan.entries[1..]);
    assert_eq!(report.verified_digests, 0);
}

#[test]
fn rejects_duplicate_out_of_order_and_mixed_versions() {
    let mut manifest = sample_manifest();
    manifest.entries[1].version = 1.into();
    assert!(matches!(
        manifest.inspect(&observation(vec![])),
        Err(MigrationError::ManifestOrder { .. })
    ));
    manifest.entries[1].version = 0.into();
    assert!(matches!(
        manifest.inspect(&observation(vec![])),
        Err(MigrationError::ManifestOrder { .. })
    ));
    manifest.entries[1].version = "text".into();
    assert_eq!(
        manifest.inspect(&observation(vec![])),
        Err(MigrationError::MixedVersionKinds)
    );

    let plan = sample_manifest();
    let duplicate = observation(vec![
        record(plan.entries[0].clone()),
        record(plan.entries[0].clone()),
    ]);
    assert!(matches!(
        plan.inspect(&duplicate),
        Err(MigrationError::LedgerOrder { .. })
    ));
    let backwards = observation(vec![
        record(plan.entries[1].clone()),
        record(plan.entries[0].clone()),
    ]);
    assert!(matches!(
        plan.inspect(&backwards),
        Err(MigrationError::LedgerOrder { .. })
    ));
    let mixed = observation(vec![record(MigrationEntry {
        version: "1".into(),
        name: "initial".into(),
        digest: Some("sha:aaa".into()),
    })]);
    assert_eq!(plan.inspect(&mixed), Err(MigrationError::MixedVersionKinds));
}

#[test]
fn rejects_gaps_unknown_and_future_database_versions() {
    let plan = sample_manifest();
    let gap = observation(vec![
        record(plan.entries[0].clone()),
        record(plan.entries[2].clone()),
    ]);
    assert_eq!(
        plan.inspect(&gap),
        Err(MigrationError::HistoryGap {
            expected: 2.into(),
            actual: 3.into()
        })
    );
    let unknown = observation(vec![record(entry(0, "untracked", Some("x")))]);
    assert_eq!(
        plan.inspect(&unknown),
        Err(MigrationError::UnknownRecordedVersion { version: 0.into() })
    );
    let future = observation(vec![record(entry(4, "future", Some("x")))]);
    assert_eq!(
        plan.inspect(&future),
        Err(MigrationError::NewerDatabaseVersion { version: 4.into() })
    );
}

#[test]
fn rejects_dirty_renamed_or_modified_history() {
    let plan = sample_manifest();
    let mut dirty = record(plan.entries[0].clone());
    dirty.state = CompletionState::Dirty;
    assert_eq!(
        plan.inspect(&observation(vec![dirty])),
        Err(MigrationError::Dirty { version: 1.into() })
    );

    let mut renamed = record(plan.entries[0].clone());
    renamed.entry.name = "renamed".into();
    assert_eq!(
        plan.inspect(&observation(vec![renamed])),
        Err(MigrationError::NameMismatch {
            version: 1.into(),
            expected: "initial".into(),
            actual: "renamed".into(),
        })
    );

    let mut modified = record(plan.entries[0].clone());
    modified.entry.digest = Some("sha:changed".into());
    assert_eq!(
        plan.inspect(&observation(vec![modified])),
        Err(MigrationError::DigestMismatch {
            version: 1.into(),
            expected: "sha:aaa".into(),
            actual: "sha:changed".into(),
        })
    );
}

#[test]
fn rejects_high_water_disagreement_without_requiring_a_high_water_ledger() {
    let plan = sample_manifest();
    let mut ledger = observation(vec![record(plan.entries[0].clone())]);
    ledger.high_water_mark = Some(2.into());
    assert_eq!(
        plan.inspect(&ledger),
        Err(MigrationError::HighWaterMismatch {
            expected: Some(1.into()),
            actual: Some(2.into()),
        })
    );
    ledger.high_water_mark = None;
    assert!(plan.inspect(&ledger).is_ok());

    let ledger = MigrationObservation {
        high_water_mark: Some(1.into()),
        ..observation(vec![])
    };
    assert_eq!(
        plan.inspect(&ledger),
        Err(MigrationError::HighWaterMismatch {
            expected: None,
            actual: Some(1.into()),
        })
    );
}

#[test]
fn rejects_invalid_identity_and_namespace() {
    let mut plan = sample_manifest();
    plan.namespace = " ".into();
    assert_eq!(
        plan.inspect(&observation(vec![])),
        Err(MigrationError::EmptyNamespace)
    );
    plan.namespace = "other".into();
    assert!(matches!(
        plan.inspect(&observation(vec![])),
        Err(MigrationError::NamespaceMismatch { .. })
    ));

    let mut plan = sample_manifest();
    plan.entries[0].name = " ".into();
    assert_eq!(
        plan.inspect(&observation(vec![])),
        Err(MigrationError::EmptyName { version: 1.into() })
    );
    plan.entries[0].name = "initial".into();
    plan.entries[0].digest = Some("".into());
    assert_eq!(
        plan.inspect(&observation(vec![])),
        Err(MigrationError::EmptyDigest { version: 1.into() })
    );
    plan.entries[0].version = "".into();
    assert_eq!(
        plan.inspect(&observation(vec![])),
        Err(MigrationError::EmptyVersion)
    );
}
