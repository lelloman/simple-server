#![cfg(feature = "database-sqlite-schema")]
use rusqlite::Connection;
use simple_server::database::sqlite::schema::*;
use std::borrow::Cow;
fn id(s: &str) -> Identifier<'static> {
    Identifier::new(s.to_owned()).unwrap()
}
fn sql(s: &str) -> SqlExpression<'static> {
    SqlExpression::trusted(s.to_owned()).unwrap()
}
fn regular() -> TableSpec<'static> {
    TableSpec {
        name: id("docs"),
        columns: vec![
            ColumnSpec {
                name: id("id"),
                declared_type: "INTEGER".into(),
                not_null: false,
                default: None,
            },
            ColumnSpec {
                name: id("title"),
                declared_type: "TEXT".into(),
                not_null: true,
                default: None,
            },
        ]
        .into(),
        primary_key: vec![id("id")].into(),
        unique_constraints: Cow::Borrowed(&[]),
        foreign_keys: Cow::Borrowed(&[]),
        indexes: Cow::Borrowed(&[]),
        unsupported: Cow::Borrowed(&[]),
    }
}
fn schema() -> ExtendedSchemaSnapshot<'static> {
    ExtendedSchemaSnapshot::new(SchemaSnapshot {
        namespace: "tests/fts".into(),
        database: id("main"),
        version: 9,
        tables: vec![regular()].into(),
    })
}
fn fts() -> Fts5TableSpec<'static> {
    Fts5TableSpec {
        name: id("docs_fts"),
        columns: vec![Fts5Column {
            name: id("title"),
            indexed: true,
        }]
        .into(),
        content: Fts5Content::External {
            table: id("docs"),
            rowid: id("id"),
        },
        tokenizer: None,
        prefixes: Cow::Borrowed(&[]),
    }
}
fn synced() -> ExtendedSchemaSnapshot<'static> {
    let mut s = schema();
    s.auto_increment = vec![AutoIncrementSpec {
        table: id("docs"),
        column: id("id"),
    }]
    .into();
    s.fts5_tables = vec![fts()].into();
    s.triggers = vec![
        TriggerSpec {
            name: id("docs_ai"),
            table: id("docs"),
            timing: TriggerTiming::After,
            event: TriggerEvent::Insert,
            when: None,
            body: sql("INSERT INTO docs_fts(rowid,title) VALUES(NEW.id,NEW.title);"),
        },
        TriggerSpec {
            name: id("docs_ad"),
            table: id("docs"),
            timing: TriggerTiming::After,
            event: TriggerEvent::Delete,
            when: None,
            body: sql("INSERT INTO docs_fts(docs_fts,rowid,title) VALUES('delete',OLD.id,OLD.title);"),
        },
        TriggerSpec {
            name: id("docs_au"),
            table: id("docs"),
            timing: TriggerTiming::After,
            event: TriggerEvent::Update {
                columns: Cow::Borrowed(&[]),
            },
            when: None,
            body: sql("INSERT INTO docs_fts(docs_fts,rowid,title) VALUES('delete',OLD.id,OLD.title);\nINSERT INTO docs_fts(rowid,title) VALUES(NEW.id,NEW.title);"),
        },
    ]
    .into();
    s
}
fn apply(c: &Connection, s: &ExtendedSchemaSnapshot<'_>, mode: CreationMode) {
    for cmd in create_extended_plan(s, mode).unwrap().statements {
        c.execute_batch(&cmd).unwrap();
    }
}
fn hits(c: &Connection, q: &str) -> i64 {
    c.query_row(
        "SELECT count(*) FROM docs_fts WHERE docs_fts MATCH ?1",
        [q],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn autoincrement_and_fts_triggers_follow_insert_update_delete_and_rollback() {
    let c = Connection::open_in_memory().unwrap();
    let s = synced();
    apply(&c, &s, CreationMode::Create);
    c.execute("INSERT INTO docs(title) VALUES('alpha')", [])
        .unwrap();
    let first = c.last_insert_rowid();
    assert_eq!(hits(&c, "alpha"), 1);
    c.execute("UPDATE docs SET title='beta'", []).unwrap();
    assert_eq!(hits(&c, "alpha"), 0);
    assert_eq!(hits(&c, "beta"), 1);
    c.execute_batch("BEGIN; INSERT INTO docs(title) VALUES('rolledback'); ROLLBACK;")
        .unwrap();
    assert_eq!(hits(&c, "rolledback"), 0);
    c.execute("DELETE FROM docs", []).unwrap();
    assert_eq!(hits(&c, "beta"), 0);
    c.execute("INSERT INTO docs(title) VALUES('gamma')", [])
        .unwrap();
    assert!(c.last_insert_rowid() > first);
    c.execute(
        "INSERT INTO docs_fts(docs_fts,rank) VALUES('integrity-check',1)",
        [],
    )
    .unwrap();
}
#[test]
fn idempotence_does_not_backfill_rebuild_or_change_version_marker() {
    let c = Connection::open_in_memory().unwrap();
    apply(&c, &schema(), CreationMode::Create);
    c.execute("INSERT INTO docs(title) VALUES('old')", [])
        .unwrap();
    c.pragma_update(None, "user_version", 77).unwrap();
    let s = synced();
    apply(&c, &s, CreationMode::IfNotExists);
    assert_eq!(hits(&c, "old"), 0);
    c.execute("INSERT INTO docs_fts(docs_fts) VALUES('rebuild')", [])
        .unwrap();
    assert_eq!(hits(&c, "old"), 1);
    apply(&c, &s, CreationMode::IfNotExists);
    assert_eq!(hits(&c, "old"), 1);
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        77
    );
    assert!(
        create_extended_plan(&s, CreationMode::Create)
            .unwrap()
            .statements
            .iter()
            .all(|s| !s.contains("IF NOT EXISTS"))
    );
}
#[test]
fn internal_and_contentless_fts_options_execute_and_quote_literals() {
    for content in [Fts5Content::Internal, Fts5Content::Contentless] {
        let c = Connection::open_in_memory().unwrap();
        let mut s = schema();
        let mut t = fts();
        t.content = content;
        t.tokenizer = Some("unicode61 remove_diacritics 2".into());
        t.prefixes = vec![2, 3].into();
        t.columns.to_mut().push(Fts5Column {
            name: id("extra"),
            indexed: false,
        });
        s.fts5_tables = vec![t].into();
        apply(&c, &s, CreationMode::Create);
        c.execute(
            "INSERT INTO docs_fts(rowid,title,extra) VALUES(1,'café','secret')",
            [],
        )
        .unwrap();
        assert_eq!(hits(&c, "cafe"), 1);
        assert_eq!(hits(&c, "caf*"), 1);
        assert_eq!(hits(&c, "secret"), 0);
    }
}
#[test]
fn invalid_extended_definitions_fail_before_returning_a_plan() {
    let mut s = synced();
    s.auto_increment.to_mut()[0].column = id("title");
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.schema.tables.to_mut()[0].columns.to_mut()[0].declared_type = "INT".into();
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    let duplicate = s.auto_increment[0].clone();
    s.auto_increment.to_mut().push(duplicate);
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.fts5_tables.to_mut()[0].name = id("DOCS");
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.fts5_tables.to_mut()[0].columns.to_mut()[0].name = id("rank");
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.fts5_tables.to_mut()[0].prefixes = vec![0].into();
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.fts5_tables.to_mut()[0].prefixes = vec![2, 2].into();
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.fts5_tables.to_mut()[0].tokenizer = Some("bad\0tokenizer".into());
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.schema.tables.to_mut().push(TableSpec {
        name: id("docs_fts_data"),
        ..regular()
    });
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.triggers.to_mut()[0].body = sql(";");
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.triggers.to_mut()[0].body = sql(";;;");
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
    let mut s = synced();
    s.triggers.to_mut()[0].event = TriggerEvent::Update {
        columns: vec![id("missing")].into(),
    };
    assert!(create_extended_plan(&s, CreationMode::Create).is_err());
}
#[test]
fn strict_quoted_index_columns_do_not_become_dqs_string_literals() {
    let c = Connection::open_in_memory().unwrap();
    c.execute("CREATE TABLE docs(id INTEGER PRIMARY KEY)", [])
        .unwrap();
    let mut s = schema();
    s.schema.tables.to_mut()[0].indexes = vec![IndexSpec {
        name: id("title_index"),
        terms: vec![IndexTerm {
            column: id("title"),
            collation: id("BINARY"),
            descending: false,
        }]
        .into(),
        unique: false,
        predicate: None,
    }]
    .into();
    let cmds = create_extended_plan(&s, CreationMode::IfNotExists)
        .unwrap()
        .statements;
    c.execute_batch(&cmds[0]).unwrap();
    assert!(c.execute_batch(&cmds[1]).is_err());
    c.execute("CREATE INDEX title_index ON docs(id)", [])
        .unwrap();
    c.execute_batch(&cmds[1]).unwrap();
}
#[test]
fn trigger_update_of_when_comments_and_external_views_execute() {
    let c = Connection::open_in_memory().unwrap();
    let mut s = schema();
    s.triggers = vec![TriggerSpec {
        name: id("change"),
        table: id("docs"),
        timing: TriggerTiming::After,
        event: TriggerEvent::Update {
            columns: vec![id("title")].into(),
        },
        when: Some(sql("NEW.title='blocked' -- predicate comment")),
        body: sql("SELECT RAISE(ABORT,'blocked'); -- body comment"),
    }]
    .into();
    apply(&c, &s, CreationMode::Create);
    c.execute("INSERT INTO docs(title) VALUES('good')", [])
        .unwrap();
    assert!(c.execute("UPDATE docs SET title='blocked'", []).is_err());
    c.execute("UPDATE docs SET id=2", []).unwrap();
    c.execute("CREATE VIEW editable AS SELECT id,title FROM docs", [])
        .unwrap();
    let mut view = ExtendedSchemaSnapshot::new(SchemaSnapshot {
        tables: Cow::Borrowed(&[]),
        ..schema().schema
    });
    view.triggers = vec![TriggerSpec {
        name: id("edit"),
        table: id("editable"),
        timing: TriggerTiming::InsteadOf,
        event: TriggerEvent::Insert,
        when: None,
        body: sql("INSERT INTO docs(title) VALUES(NEW.title);"),
    }]
    .into();
    apply(&c, &view, CreationMode::Create);
    c.execute("INSERT INTO editable(title) VALUES('via_view')", [])
        .unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM docs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}
#[test]
fn attached_schema_and_file_restart_keep_fts_and_sequence() {
    struct TempDatabase(std::path::PathBuf);
    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let file = TempDatabase(std::env::temp_dir().join(format!(
            "simple-server-fts5-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    let path = &file.0;
    let c = Connection::open_in_memory().unwrap();
    c.execute("ATTACH ?1 AS archive", [path.to_str().unwrap()])
        .unwrap();
    let mut s = synced();
    s.schema.database = id("archive");
    apply(&c, &s, CreationMode::Create);
    c.execute("INSERT INTO archive.docs(title) VALUES('persisted')", [])
        .unwrap();
    c.execute("DELETE FROM archive.docs", []).unwrap();
    drop(c);
    let c = Connection::open(path).unwrap();
    c.execute("INSERT INTO docs(title) VALUES('restarted')", [])
        .unwrap();
    assert_eq!(c.last_insert_rowid(), 2);
    assert_eq!(hits(&c, "restarted"), 1);
}
#[test]
fn failed_application_transaction_rolls_back_extended_schema_and_records() {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE keep(value TEXT); INSERT INTO keep VALUES('old'); PRAGMA user_version=5; BEGIN;").unwrap();
    apply(&c, &synced(), CreationMode::Create);
    assert!(c.execute("CREATE TABLE keep(value TEXT)", []).is_err());
    c.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name LIKE 'docs%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT value FROM keep", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "old"
    );
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
}

#[test]
fn quoted_content_identifiers_and_tokenizer_literals_cannot_escape_into_sql() {
    let c = Connection::open_in_memory().unwrap();
    let mut s = schema();
    s.schema.tables.to_mut()[0].name = id("docs'quoted");
    let mut t = fts();
    t.name = id("fts\"quoted");
    t.content = Fts5Content::External {
        table: id("docs'quoted"),
        rowid: id("id"),
    };
    s.fts5_tables = vec![t].into();
    apply(&c, &s, CreationMode::Create);
    c.execute("INSERT INTO \"docs'quoted\"(title) VALUES('safe')", [])
        .unwrap();
    c.execute(
        "INSERT INTO \"fts\"\"quoted\"(\"fts\"\"quoted\") VALUES('rebuild')",
        [],
    )
    .unwrap();
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM \"fts\"\"quoted\" WHERE \"fts\"\"quoted\" MATCH 'safe'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let c = Connection::open_in_memory().unwrap();
    let mut s = schema();
    let mut t = fts();
    t.tokenizer = Some("unicode61'); DROP TABLE docs; --".into());
    s.fts5_tables = vec![t].into();
    let plan = create_extended_plan(&s, CreationMode::Create).unwrap();
    c.execute_batch(&plan.statements[0]).unwrap();
    assert!(c.execute_batch(&plan.statements[1]).is_err());
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name='docs'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

fn strict_options() -> CreationOptions<'static> {
    CreationOptions {
        tables: vec![TableCreationOptions {
            table: id("docs"),
            strict: true,
            checks: vec![
                CheckConstraint {
                    name: Some(id("positive id")),
                    column: Some(id("id")),
                    expression: sql("id > 0 -- retain comment"),
                },
                CheckConstraint {
                    name: None,
                    column: None,
                    expression: sql("length(title) BETWEEN 1 AND 8"),
                },
            ]
            .into(),
        }]
        .into(),
        virtual_tables: Cow::Borrowed(&[]),
    }
}

#[test]
fn strict_checks_enforce_types_constraints_defaults_and_rollback() {
    let conn = Connection::open_in_memory().unwrap();
    let mut s = schema();
    s.auto_increment = vec![AutoIncrementSpec {
        table: id("docs"),
        column: id("id"),
    }]
    .into();
    let options = strict_options();
    let plan = create_extended_plan_with_options(&s, CreationMode::Create, &options).unwrap();
    for statement in &plan.statements {
        conn.execute_batch(statement).unwrap();
    }
    let strict: i64 = conn
        .query_row(
            "SELECT strict FROM pragma_table_list WHERE name='docs'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(strict, 1);
    conn.execute("INSERT INTO docs(title) VALUES('hello')", [])
        .unwrap();
    assert!(
        conn.execute("INSERT INTO docs VALUES('invalid', 'hello')", [])
            .is_err()
    );
    assert!(
        conn.execute("INSERT INTO docs VALUES(-1, 'hello')", [])
            .is_err()
    );
    assert!(
        conn.execute("INSERT INTO docs(title) VALUES('too long text')", [])
            .is_err()
    );
    conn.execute_batch("BEGIN; INSERT INTO docs(title) VALUES('rollback'); ROLLBACK;")
        .unwrap();
    conn.execute("INSERT INTO docs(title) VALUES('last')", [])
        .unwrap();
    assert_eq!(conn.last_insert_rowid(), 2);
    for statement in create_extended_plan_with_options(&s, CreationMode::IfNotExists, &options)
        .unwrap()
        .statements
    {
        conn.execute_batch(&statement).unwrap();
    }
    assert_eq!(
        conn.query_row("SELECT count(*) FROM docs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn strict_any_and_non_integer_primary_key_preserve_sqlite_semantics() {
    let conn = Connection::open_in_memory().unwrap();
    let mut s = schema();
    let mut table = regular();
    table.columns.to_mut()[0].declared_type = "INT".into();
    table.columns.to_mut()[1].declared_type = "ANY".into();
    s.schema.tables = vec![table].into();
    let options = CreationOptions {
        tables: vec![TableCreationOptions {
            table: id("docs"),
            strict: true,
            checks: Cow::Borrowed(&[]),
        }]
        .into(),
        ..Default::default()
    };
    let plan = create_extended_plan_with_options(&s, CreationMode::Create, &options).unwrap();
    conn.execute_batch(&plan.statements.join("\n")).unwrap();
    assert!(
        conn.execute("INSERT INTO docs VALUES(NULL,'000123')", [])
            .is_err()
    );
    conn.execute("INSERT INTO docs VALUES(1,'000123')", [])
        .unwrap();
    assert_eq!(
        conn.query_row("SELECT typeof(title)||':'||title FROM docs", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "text:000123"
    );
    assert!(
        conn.execute("INSERT INTO docs VALUES('text','x')", [])
            .is_err()
    );
    assert_eq!(
        conn.query_row(
            "SELECT \"notnull\" FROM pragma_table_info('docs') WHERE name='id'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn invalid_strict_check_and_virtual_definitions_fail_before_execution() {
    let s = schema();
    for kind in ["", "VARCHAR", "BIGINT", " INTEGER "] {
        let mut invalid = s.clone();
        invalid.schema.tables.to_mut()[0].columns.to_mut()[0].declared_type =
            kind.to_owned().into();
        assert!(
            create_extended_plan_with_options(&invalid, CreationMode::Create, &strict_options())
                .is_err()
        );
    }
    let mut options = strict_options();
    options.tables.to_mut()[0].table = id("missing");
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
    options = strict_options();
    options.tables.to_mut()[0].checks.to_mut()[0].column = Some(id("missing"));
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
    options = strict_options();
    let duplicate = options.tables[0].clone();
    options.tables.to_mut().push(duplicate);
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
    options = CreationOptions::default();
    options.virtual_tables = vec![VirtualTableSpec {
        name: id("docs"),
        module: id("fts5"),
        arguments: Some(VirtualTableArguments::trusted("title").unwrap()),
    }]
    .into();
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
    options.virtual_tables.to_mut()[0].name = id("sqlite_reserved");
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
    for args in ["", "  ", "x\0y"] {
        assert!(VirtualTableArguments::trusted(args).is_err());
    }
}

#[test]
fn generic_modules_preserve_search_spatial_queries_and_attached_database() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("ATTACH ':memory:' AS aux;").unwrap();
    let mut s = schema();
    s.schema.database = id("aux");
    s.schema.tables = Cow::Borrowed(&[]);
    let options = CreationOptions {
        virtual_tables: vec![
            VirtualTableSpec {
                name: id("generic search"),
                module: id("fts5"),
                arguments: Some(
                    VirtualTableArguments::trusted("title, tokenize='unicode61' -- comment")
                        .unwrap(),
                ),
            },
            VirtualTableSpec {
                name: id("bounds"),
                module: id("rtree"),
                arguments: Some(
                    VirtualTableArguments::trusted("id, min_x, max_x, min_y, max_y").unwrap(),
                ),
            },
        ]
        .into(),
        ..Default::default()
    };
    let plan = create_extended_plan_with_options(&s, CreationMode::Create, &options).unwrap();
    conn.execute_batch(&plan.statements.join("\n")).unwrap();
    conn.execute_batch("INSERT INTO aux.\"generic search\" VALUES('hello world'); INSERT INTO aux.bounds VALUES(1,0,10,0,10);").unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM aux.\"generic search\" WHERE \"generic search\" MATCH 'hello'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT id FROM aux.bounds WHERE min_x<=5 AND max_x>=5",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let plan = create_extended_plan_with_options(&s, CreationMode::IfNotExists, &options).unwrap();
    conn.execute_batch(&plan.statements.join("\n")).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM aux.bounds", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(
        !conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM main.sqlite_master WHERE name='bounds')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
    );
}

#[test]
fn unavailable_module_and_bad_check_leave_transaction_ownership_to_caller() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE retained(x); INSERT INTO retained VALUES(1); PRAGMA user_version=42; BEGIN;",
    )
    .unwrap();
    let options = CreationOptions {
        virtual_tables: vec![VirtualTableSpec {
            name: id("vectors"),
            module: id("missing module\"; DROP TABLE retained;--"),
            arguments: None,
        }]
        .into(),
        ..Default::default()
    };
    let plan =
        create_extended_plan_with_options(&schema(), CreationMode::Create, &options).unwrap();
    conn.execute_batch(&plan.statements[0]).unwrap();
    assert!(conn.execute_batch(&plan.statements[1]).is_err());
    conn.execute_batch("ROLLBACK;").unwrap();
    assert_eq!(
        conn.query_row("SELECT x FROM retained", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(
        !conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='docs')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        42
    );
    let mut options = strict_options();
    options.tables.to_mut()[0].checks.to_mut()[0].expression = sql("unknown_column > 0");
    let plan =
        create_extended_plan_with_options(&schema(), CreationMode::Create, &options).unwrap();
    assert!(conn.execute_batch(&plan.statements.join("\n")).is_err());
}

#[test]
fn creation_options_do_not_change_legacy_sql_and_catch_known_shadow_collisions() {
    let mut s = schema();
    assert_eq!(
        create_extended_plan(&s, CreationMode::Create).unwrap(),
        create_extended_plan_with_options(&s, CreationMode::Create, &CreationOptions::default())
            .unwrap()
    );
    s.fts5_tables = vec![fts()].into();
    let options = CreationOptions {
        virtual_tables: vec![VirtualTableSpec {
            name: id("docs_fts_data"),
            module: id("rtree"),
            arguments: None,
        }]
        .into(),
        ..Default::default()
    };
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
}

#[test]
fn strict_runner_ledger_matches_legacy_constraints_and_file_restart() {
    struct TempDatabase(std::path::PathBuf);
    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!(
        "simple-server-strict-ledger-{}-{unique}.sqlite",
        std::process::id()
    );
    let path = TempDatabase(std::env::temp_dir().join(name));
    let legacy = Connection::open_in_memory().unwrap();
    legacy.execute_batch("CREATE TABLE runner_migration(version INTEGER PRIMARY KEY NOT NULL,name TEXT NOT NULL,checksum TEXT NOT NULL,dirty INTEGER NOT NULL DEFAULT 0 CHECK(dirty IN(0,1)),applied_at INTEGER NOT NULL DEFAULT(unixepoch())) STRICT;").unwrap();
    let table = TableSpec {
        name: id("runner_migration"),
        columns: vec![
            ColumnSpec {
                name: id("version"),
                declared_type: "INTEGER".into(),
                not_null: true,
                default: None,
            },
            ColumnSpec {
                name: id("name"),
                declared_type: "TEXT".into(),
                not_null: true,
                default: None,
            },
            ColumnSpec {
                name: id("checksum"),
                declared_type: "TEXT".into(),
                not_null: true,
                default: None,
            },
            ColumnSpec {
                name: id("dirty"),
                declared_type: "INTEGER".into(),
                not_null: true,
                default: Some(sql("0")),
            },
            ColumnSpec {
                name: id("applied_at"),
                declared_type: "INTEGER".into(),
                not_null: true,
                default: Some(sql("unixepoch()")),
            },
        ]
        .into(),
        primary_key: vec![id("version")].into(),
        unique_constraints: Cow::Borrowed(&[]),
        foreign_keys: Cow::Borrowed(&[]),
        indexes: Cow::Borrowed(&[]),
        unsupported: Cow::Borrowed(&[]),
    };
    let mut s = schema();
    s.schema.tables = vec![table].into();
    let options = CreationOptions {
        tables: vec![TableCreationOptions {
            table: id("runner_migration"),
            strict: true,
            checks: vec![CheckConstraint {
                name: None,
                column: Some(id("dirty")),
                expression: sql("dirty IN (0,1)"),
            }]
            .into(),
        }]
        .into(),
        ..Default::default()
    };
    let plan = create_extended_plan_with_options(&s, CreationMode::IfNotExists, &options).unwrap();
    {
        let shared = Connection::open(&path.0).unwrap();
        shared.execute_batch(&plan.statements.join("\n")).unwrap();
        let metadata = |conn: &Connection| {
            conn.prepare("SELECT name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('runner_migration') ORDER BY cid").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,i64>(4)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap()
        };
        assert_eq!(metadata(&legacy), metadata(&shared));
        for conn in [&legacy, &shared] {
            conn.execute_batch("INSERT INTO runner_migration(version,name,checksum) VALUES(1,'one','abc'); PRAGMA user_version=99;").unwrap();
            assert_eq!(
                conn.query_row("SELECT dirty FROM runner_migration", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            for bad in [
                "UPDATE runner_migration SET dirty=2",
                "UPDATE runner_migration SET dirty='not an integer'",
                "UPDATE runner_migration SET dirty=NULL",
                "INSERT INTO runner_migration(version,name,checksum) VALUES(1,'duplicate','x')",
            ] {
                assert!(conn.execute_batch(bad).is_err());
            }
            conn.execute_batch("BEGIN; UPDATE runner_migration SET dirty=1; ROLLBACK;")
                .unwrap();
        }
    }
    let reopened = Connection::open(&path.0).unwrap();
    reopened.execute_batch(&plan.statements.join("\n")).unwrap();
    assert_eq!(
        reopened
            .query_row("SELECT name||':'||dirty FROM runner_migration", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "one:0"
    );
    assert_eq!(
        reopened
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        99
    );
    assert!(
        reopened
            .execute_batch("UPDATE runner_migration SET dirty=2")
            .is_err()
    );
}

#[test]
fn checks_keep_sqlite_null_behavior_and_named_constraints() {
    let conn = Connection::open_in_memory().unwrap();
    let mut s = schema();
    s.schema.tables.to_mut()[0].columns.to_mut()[1].not_null = false;
    let mut options = strict_options();
    options.tables.to_mut()[0].strict = false;
    let plan = create_extended_plan_with_options(&s, CreationMode::Create, &options).unwrap();
    conn.execute_batch(&plan.statements.join("\n")).unwrap();
    conn.execute_batch("INSERT INTO docs VALUES(1,NULL)")
        .unwrap(); // NULL CHECK results are accepted by SQLite.
    assert!(
        conn.execute_batch("INSERT INTO docs VALUES(-1,'ok')")
            .unwrap_err()
            .to_string()
            .contains("positive id")
    );
    let duplicate = options.tables[0].checks[0].clone();
    options.tables.to_mut()[0].checks.to_mut().push(duplicate);
    assert!(create_extended_plan_with_options(&s, CreationMode::Create, &options).is_err());
}
