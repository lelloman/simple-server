#![cfg(feature = "database-sqlite-schema")]

use rusqlite::Connection;
use simple_server::database::sqlite::schema::*;
use std::{borrow::Cow, collections::BTreeMap};

fn id(s: &str) -> Identifier<'static> {
    Identifier::new(s.to_owned()).unwrap()
}
fn sql(s: &str) -> SqlExpression<'static> {
    SqlExpression::trusted(s.to_owned()).unwrap()
}
fn col(name: &str, ty: &str) -> ColumnSpec<'static> {
    ColumnSpec {
        name: id(name),
        declared_type: ty.to_owned().into(),
        not_null: false,
        default: None,
    }
}
fn table(name: &str, columns: Vec<ColumnSpec<'static>>) -> TableSpec<'static> {
    TableSpec {
        name: id(name),
        columns: columns.into(),
        primary_key: Cow::Borrowed(&[]),
        unique_constraints: Cow::Borrowed(&[]),
        foreign_keys: Cow::Borrowed(&[]),
        indexes: Cow::Borrowed(&[]),
        unsupported: Cow::Borrowed(&[]),
    }
}
fn snapshot(tables: Vec<TableSpec<'static>>) -> SchemaSnapshot<'static> {
    SchemaSnapshot {
        namespace: "test/catalog".into(),
        database: id("main"),
        version: 3,
        tables: tables.into(),
    }
}
fn policy(scope: ValidationScope) -> ValidationPolicy {
    ValidationPolicy {
        scope,
        column_order: ColumnOrder::Ordered,
        declared_types: TypeComparison::Exact,
        expressions: ExpressionComparison::Exact,
    }
}
fn apply(conn: &Connection, s: &SchemaSnapshot<'_>) {
    for command in create_plan(s).unwrap().statements {
        conn.execute_batch(&command).unwrap();
    }
}
fn action(s: &str) -> rusqlite::Result<ForeignKeyAction> {
    match s {
        "NO ACTION" => Ok(ForeignKeyAction::NoAction),
        "RESTRICT" => Ok(ForeignKeyAction::Restrict),
        "SET NULL" => Ok(ForeignKeyAction::SetNull),
        "SET DEFAULT" => Ok(ForeignKeyAction::SetDefault),
        "CASCADE" => Ok(ForeignKeyAction::Cascade),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

// Test-only adapter. PRAGMAs cannot establish every DDL property. The caller
// explicitly lists tables whose entire DDL has been audited in these fixtures;
// this is not a general-purpose production parser. Unsupported expression-index
// terms/implicit FK target columns propagate errors rather than disappearing.
fn observe(
    conn: &Connection,
    database: &str,
    audited: &[&str],
) -> rusqlite::Result<SchemaObservation<'static>> {
    let db = id(database).quoted();
    let mut stmt = conn.prepare(&format!("SELECT type, name FROM {db}.sqlite_schema WHERE name NOT LIKE 'sqlite_%' AND type != 'index' ORDER BY name"))?;
    let objects = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut tables = vec![];
    let mut other = vec![];
    for (kind, name) in objects {
        if kind != "table" {
            other.push(SchemaObject {
                kind: kind.into(),
                name: id(&name),
            });
            continue;
        }
        let mut stmt = conn.prepare("SELECT name, type, \"notnull\", dflt_value, pk, hidden FROM pragma_table_xinfo(?1, ?2) ORDER BY cid")?;
        let rows = stmt
            .query_map([&name, database], |row| {
                Ok((
                    ColumnSpec {
                        name: id(&row.get::<_, String>(0)?),
                        declared_type: row.get::<_, String>(1)?.into(),
                        not_null: row.get::<_, i64>(2)? != 0,
                        default: row.get::<_, Option<String>>(3)?.map(|s| sql(&s)),
                    },
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut pk: Vec<_> = rows
            .iter()
            .filter(|(_, pk, _)| *pk != 0)
            .map(|(c, pk, _)| (*pk, c.name.clone()))
            .collect();
        pk.sort_by_key(|(position, _)| *position);
        let hidden = rows.iter().any(|(_, _, hidden)| *hidden != 0);
        let columns = rows.into_iter().map(|(c, _, _)| c).collect();
        let mut stmt = conn
            .prepare("SELECT name, \"unique\", origin, partial FROM pragma_index_list(?1, ?2)")?;
        let indices = stmt
            .query_map([&name, database], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut unique_constraints = vec![];
        let mut indexes = vec![];
        for (index_name, unique, origin, partial) in indices {
            let mut stmt = conn.prepare("SELECT name, coll, desc FROM pragma_index_xinfo(?1, ?2) WHERE key=1 ORDER BY seqno")?;
            let terms = stmt
                .query_map([&index_name, database], |row| {
                    Ok(IndexTerm {
                        column: id(&row.get::<_, String>(0)?),
                        collation: id(&row.get::<_, String>(1)?),
                        descending: row.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            match origin.as_str() {
                "pk" => {}
                "u" => unique_constraints.push(terms.iter().map(|t| t.column.clone()).collect()),
                "c" => {
                    let predicate = if partial {
                        let ddl: String = conn.query_row(
                            &format!(
                                "SELECT sql FROM {db}.sqlite_schema WHERE name=?1 AND type='index'"
                            ),
                            [&index_name],
                            |r| r.get(0),
                        )?;
                        // Only the exact fixture grammar is supported. Fail rather than
                        // silently claiming a missing predicate if parsing is unavailable.
                        Some(sql(ddl
                            .split_once(" WHERE ")
                            .ok_or(rusqlite::Error::InvalidQuery)?
                            .1
                            .trim_end_matches(';')))
                    } else {
                        None
                    };
                    indexes.push(IndexSpec {
                        name: id(&index_name),
                        terms: terms.into(),
                        unique,
                        predicate,
                    });
                }
                _ => return Err(rusqlite::Error::InvalidQuery),
            }
        }
        let mut stmt = conn.prepare("SELECT id, seq, \"table\", \"from\", \"to\", on_update, on_delete FROM pragma_foreign_key_list(?1, ?2) ORDER BY id, seq")?;
        let rows = stmt
            .query_map([&name, database], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut fks: BTreeMap<i64, ForeignKeySpec<'static>> = BTreeMap::new();
        for (n, parent, from, to, update, delete) in rows {
            let on_update = action(&update)?;
            let on_delete = action(&delete)?;
            let fk = fks.entry(n).or_insert_with(|| ForeignKeySpec {
                columns: vec![].into(),
                parent_table: id(&parent),
                parent_columns: vec![].into(),
                on_update,
                on_delete,
            });
            fk.columns.to_mut().push(id(&from));
            fk.parent_columns.to_mut().push(id(&to));
        }
        let ordinary_table = if audited.contains(&name.as_str()) && !hidden {
            Observed::Known(())
        } else {
            Observed::Unavailable(
                "DDL properties outside the audited ordinary fixture model".into(),
            )
        };
        tables.push(TableObservation {
            name: id(&name),
            columns: Observed::Known(columns),
            primary_key: Observed::Known(pk.into_iter().map(|(_, c)| c).collect()),
            unique_constraints: Observed::Known(unique_constraints),
            foreign_keys: Observed::Known(fks.into_values().collect()),
            indexes: Observed::Known(indexes),
            ordinary_table,
        });
    }
    Ok(SchemaObservation {
        namespace: "test/catalog".into(),
        database: id(database),
        tables: Observed::Known(tables),
        other_objects: Observed::Known(other),
    })
}

fn report(conn: &Connection, s: &SchemaSnapshot<'_>) -> SchemaReport {
    let audited: Vec<_> = s.tables.iter().map(|t| t.name.as_str()).collect();
    validate(
        s,
        &observe(conn, s.database.as_str(), &audited).unwrap(),
        policy(ValidationScope::Exact),
    )
    .unwrap()
}
fn fixture() -> SchemaSnapshot<'static> {
    let mut parent = table("parent", vec![col("tenant", "TEXT"), col("id", "INTEGER")]);
    parent.primary_key = vec![id("tenant"), id("id")].into();
    let mut child = table(
        "child",
        vec![
            col("tenant", "TEXT"),
            col("parent_id", "INTEGER"),
            col("payload", "BLOB"),
            col("score", "DOUBLE PRECISION"),
        ],
    );
    child.columns.to_mut()[0].not_null = true;
    child.columns.to_mut()[3].default = Some(sql("1 + 2"));
    child.unique_constraints = vec![vec![id("tenant"), id("parent_id")]].into();
    child.foreign_keys = vec![ForeignKeySpec {
        columns: vec![id("tenant"), id("parent_id")].into(),
        parent_table: id("parent"),
        parent_columns: vec![id("tenant"), id("id")].into(),
        on_update: ForeignKeyAction::Cascade,
        on_delete: ForeignKeyAction::Restrict,
    }]
    .into();
    child.indexes = vec![IndexSpec {
        name: id("child_score"),
        terms: vec![
            IndexTerm {
                column: id("score"),
                collation: id("BINARY"),
                descending: true,
            },
            IndexTerm {
                column: id("tenant"),
                collation: id("NOCASE"),
                descending: false,
            },
        ]
        .into(),
        unique: false,
        predicate: Some(sql("score > 0")),
    }]
    .into();
    snapshot(vec![parent, child])
}

#[test]
fn real_sqlite_round_trip_and_constraints_preserve_version_marker() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA user_version=99999;")
        .unwrap();
    let s = fixture();
    apply(&conn, &s);
    assert!(report(&conn, &s).is_match(), "{:?}", report(&conn, &s));
    let plan = create_plan(&s).unwrap();
    assert_eq!(plan, create_plan(&s).unwrap());
    assert_eq!(plan.statements.len(), 3);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        99999
    );
    conn.execute("INSERT INTO parent VALUES ('a',1)", [])
        .unwrap();
    conn.execute("INSERT INTO child(tenant,parent_id) VALUES ('a',1)", [])
        .unwrap();
    assert_eq!(
        conn.query_row("SELECT score FROM child", [], |r| r.get::<_, f64>(0))
            .unwrap(),
        3.0
    );
    assert!(
        conn.execute("INSERT INTO child(tenant,parent_id) VALUES ('a',1)", [])
            .is_err()
    );
    assert!(
        conn.execute("INSERT INTO child(tenant,parent_id) VALUES ('a',2)", [])
            .is_err()
    );
    assert!(conn.execute("DELETE FROM parent", []).is_err());
    conn.execute("UPDATE parent SET id=3", []).unwrap();
    assert_eq!(
        conn.query_row("SELECT parent_id FROM child", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn quoted_identifiers_and_attached_database_are_isolated() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("ATTACH ':memory:' AS \"other db\";")
        .unwrap();
    let mut t = table(
        "table\"; DROP TABLE x; --",
        vec![col("a'b\"c", "TEXT"), col("untyped", "")],
    );
    t.indexes = vec![IndexSpec {
        name: id("odd\"index"),
        terms: vec![IndexTerm {
            column: id("a'b\"c"),
            collation: id("BINARY"),
            descending: false,
        }]
        .into(),
        unique: true,
        predicate: None,
    }]
    .into();
    let mut s = snapshot(vec![t]);
    s.database = id("other db");
    apply(&conn, &s);
    assert!(report(&conn, &s).is_match());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM main.sqlite_schema", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn changed_columns_defaults_nullability_and_order_are_distinguished() {
    let conn = Connection::open_in_memory().unwrap();
    let s = snapshot(vec![table(
        "t",
        vec![col("a", "INTEGER"), col("b", "TEXT")],
    )]);
    conn.execute_batch("CREATE TABLE t(b TEXT DEFAULT 'changed', a TEXT NOT NULL);")
        .unwrap();
    let r = report(&conn, &s);
    for suffix in ["column_order", "type", "not_null", "default"] {
        assert!(
            r.differences.iter().any(|d| d.path.ends_with(suffix)),
            "{suffix}: {r:?}"
        );
    }
    assert!(!r.is_match());
}

#[test]
fn same_named_index_with_wrong_terms_uniqueness_and_predicate_fails() {
    let conn = Connection::open_in_memory().unwrap();
    let s = fixture();
    apply(&conn, &s);
    conn.execute_batch("DROP INDEX child_score; CREATE UNIQUE INDEX child_score ON child(tenant DESC, score) WHERE score < 0;").unwrap();
    let r = report(&conn, &s);
    for suffix in ["terms", "unique", "predicate"] {
        assert!(
            r.differences.iter().any(|d| d.path.ends_with(suffix)),
            "{r:?}"
        );
    }
    conn.execute_batch("DROP INDEX child_score").unwrap();
    assert!(
        report(&conn, &s)
            .differences
            .iter()
            .any(|d| d.kind == DifferenceKind::Missing && d.path.contains("child_score"))
    );
}

#[test]
fn wrong_composite_keys_and_foreign_key_actions_fail() {
    let conn = Connection::open_in_memory().unwrap();
    let s = fixture();
    apply(&conn, &s);
    let mut changed = s.clone();
    changed.tables.to_mut()[0].primary_key.to_mut().reverse();
    changed.tables.to_mut()[1].unique_constraints.to_mut()[0].reverse();
    changed.tables.to_mut()[1].foreign_keys.to_mut()[0].on_delete = ForeignKeyAction::Cascade;
    let r = report(&conn, &changed);
    for suffix in ["primary_key", "unique_constraints", "foreign_keys"] {
        assert!(
            r.differences.iter().any(|d| d.path.ends_with(suffix)),
            "{r:?}"
        );
    }
}

#[test]
fn exact_and_subset_have_explicit_extra_object_and_order_semantics() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t(extra INTEGER, a TEXT, b TEXT); CREATE TABLE extra_table(x); CREATE INDEX extra_index ON t(a);").unwrap();
    let s = snapshot(vec![table("t", vec![col("a", "TEXT"), col("b", "TEXT")])]);
    let obs = observe(&conn, "main", &["t", "extra_table"]).unwrap();
    assert!(
        !validate(&s, &obs, policy(ValidationScope::Exact))
            .unwrap()
            .is_match()
    );
    assert!(
        validate(&s, &obs, policy(ValidationScope::RequiredSubset))
            .unwrap()
            .is_match()
    );
    let mut swapped = s.clone();
    swapped.tables.to_mut()[0].columns.to_mut().reverse();
    assert!(
        !validate(&swapped, &obs, policy(ValidationScope::RequiredSubset))
            .unwrap()
            .is_match()
    );
    let mut unordered = policy(ValidationScope::RequiredSubset);
    unordered.column_order = ColumnOrder::ByName;
    assert!(validate(&swapped, &obs, unordered).unwrap().is_match());
}

#[test]
fn missing_tables_and_columns_are_not_healthy_or_repaired() {
    let conn = Connection::open_in_memory().unwrap();
    let s = fixture();
    assert!(
        report(&conn, &s)
            .differences
            .iter()
            .all(|d| d.kind == DifferenceKind::Missing)
    );
    conn.execute_batch("CREATE TABLE parent(tenant TEXT)")
        .unwrap();
    assert!(
        report(&conn, &s)
            .differences
            .iter()
            .any(|d| d.path.contains("column[\"id\"]"))
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM sqlite_schema", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn unknown_metadata_and_unsupported_objects_never_silently_pass() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE t(a TEXT CHECK(length(a)>0)); CREATE VIEW v AS SELECT a FROM t;",
    )
    .unwrap();
    let s = snapshot(vec![table("t", vec![col("a", "TEXT")])]);
    let obs = observe(&conn, "main", &[]).unwrap();
    let r = validate(&s, &obs, policy(ValidationScope::Exact)).unwrap();
    assert!(
        r.differences
            .iter()
            .any(|d| d.kind == DifferenceKind::Unverified)
    );
    assert!(r.differences.iter().any(|d| d.path.starts_with("view")));
    let mut unavailable = obs;
    unavailable.tables = Observed::Unavailable("adapter cannot read table metadata".into());
    assert!(
        !validate(&s, &unavailable, policy(ValidationScope::RequiredSubset))
            .unwrap()
            .is_match()
    );
    assert!(observe(&conn, "does_not_exist", &[]).is_err());
}

#[test]
fn unsupported_creation_and_invalid_definitions_fail_before_execution() {
    let mut s = snapshot(vec![table("t", vec![col("a", "TEXT")])]);
    s.tables.to_mut()[0].unsupported = vec!["CHECK constraint".into()].into();
    assert!(create_plan(&s).unwrap_err().reason.contains("unsupported"));
    s.tables.to_mut()[0].unsupported = vec![].into();
    s.tables.to_mut()[0].primary_key = vec![id("missing")].into();
    assert!(
        create_plan(&s)
            .unwrap_err()
            .reason
            .contains("unknown column")
    );
    s.tables.to_mut()[0].primary_key = vec![].into();
    s.tables.to_mut()[0].columns.to_mut().push(col("A", "TEXT"));
    assert!(create_plan(&s).unwrap_err().reason.contains("duplicate"));
    assert!(Identifier::new("").is_err());
    assert!(Identifier::new("x\0y").is_err());
    assert!(SqlExpression::trusted(" ").is_err());
}

#[test]
fn expression_normalization_is_explicit_and_does_not_rewrite_literals_or_parentheses() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t(a TEXT DEFAULT ' x ')")
        .unwrap();
    let mut s = snapshot(vec![table("t", vec![col("a", "TEXT")])]);
    s.tables.to_mut()[0].columns.to_mut()[0].default = Some(sql("  ' x ' \n"));
    let obs = observe(&conn, "main", &["t"]).unwrap();
    assert!(
        !validate(&s, &obs, policy(ValidationScope::Exact))
            .unwrap()
            .is_match()
    );
    let mut p = policy(ValidationScope::Exact);
    p.expressions = ExpressionComparison::TrimWhitespace;
    assert!(validate(&s, &obs, p).unwrap().is_match());
    for different in ["'x'", "(' x ')"] {
        s.tables.to_mut()[0].columns.to_mut()[0].default = Some(sql(different));
        assert!(!validate(&s, &obs, p).unwrap().is_match());
    }
}

#[test]
fn names_types_and_namespace_comparisons_are_explicit() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t(a INTEGER)").unwrap();
    let mut s = snapshot(vec![table("T", vec![col("A", "integer")])]);
    let obs = observe(&conn, "main", &["t"]).unwrap();
    assert!(
        !validate(&s, &obs, policy(ValidationScope::Exact))
            .unwrap()
            .is_match()
    );
    let mut p = policy(ValidationScope::Exact);
    p.declared_types = TypeComparison::AsciiCaseInsensitive;
    assert!(validate(&s, &obs, p).unwrap().is_match());
    s.namespace = "other/store".into();
    assert!(!validate(&s, &obs, p).unwrap().is_match());
    s.namespace = "test/catalog".into();
    s.database = id("other");
    assert!(!validate(&s, &obs, p).unwrap().is_match());
}

#[test]
fn transaction_failure_rolls_back_creation_and_application_marker() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA user_version=7").unwrap();
    {
        let tx = conn.transaction().unwrap();
        apply(&tx, &fixture());
        tx.execute_batch("PRAGMA user_version=8").unwrap();
        assert!(tx.execute_batch("INSERT INTO missing VALUES (1)").is_err());
        // Application owns rollback, not the creation plan.
    }
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM sqlite_schema", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    apply(&conn, &fixture());
    assert!(report(&conn, &fixture()).is_match());
}

#[test]
fn borrowed_descriptions_and_runtime_observations_can_be_combined() {
    let name = String::from("t");
    let ty = String::from("TEXT");
    let columns = [ColumnSpec {
        name: Identifier::new("a").unwrap(),
        declared_type: Cow::Borrowed(&ty),
        not_null: false,
        default: None,
    }];
    let tables = [TableSpec {
        name: Identifier::new(name.as_str()).unwrap(),
        columns: Cow::Borrowed(&columns),
        primary_key: Cow::Borrowed(&[]),
        unique_constraints: Cow::Borrowed(&[]),
        foreign_keys: Cow::Borrowed(&[]),
        indexes: Cow::Borrowed(&[]),
        unsupported: Cow::Borrowed(&[]),
    }];
    let s = SchemaSnapshot {
        namespace: Cow::Borrowed("test/catalog"),
        database: Identifier::new("main").unwrap(),
        version: 1,
        tables: Cow::Borrowed(&tables),
    };
    let conn = Connection::open_in_memory().unwrap();
    apply(&conn, &s);
    assert!(report(&conn, &s).is_match());
}

#[test]
fn every_unavailable_property_and_duplicate_observation_is_visible() {
    let conn = Connection::open_in_memory().unwrap();
    let s = fixture();
    apply(&conn, &s);
    let mut obs = observe(&conn, "main", &["parent", "child"]).unwrap();
    let Observed::Known(tables) = &mut obs.tables else {
        panic!()
    };
    let child = tables.iter_mut().find(|t| t.name == id("child")).unwrap();
    child.columns = Observed::Unavailable("columns".into());
    child.primary_key = Observed::Unavailable("pk".into());
    child.unique_constraints = Observed::Unavailable("unique".into());
    child.foreign_keys = Observed::Unavailable("fk".into());
    child.indexes = Observed::Unavailable("index expressions".into());
    child.ordinary_table = Observed::Unavailable("extended table properties".into());
    let r = validate(&s, &obs, policy(ValidationScope::Exact)).unwrap();
    assert_eq!(r.differences.len(), 6);
    assert!(
        r.differences
            .iter()
            .all(|d| d.kind == DifferenceKind::Unverified)
    );
    let Observed::Known(tables) = &mut obs.tables else {
        panic!()
    };
    tables.push(tables[0].clone());
    assert!(validate(&s, &obs, policy(ValidationScope::Exact)).is_err());
}

#[test]
fn subset_extras_are_reported_without_claiming_their_validation() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t(a TEXT, extra TEXT); CREATE VIEW v AS SELECT * FROM t;")
        .unwrap();
    let s = snapshot(vec![table("t", vec![col("a", "TEXT")])]);
    let obs = observe(&conn, "main", &["t"]).unwrap();
    let r = validate(&s, &obs, policy(ValidationScope::RequiredSubset)).unwrap();
    assert!(r.is_match());
    assert_eq!(r.outside_scope.len(), 2);
    assert!(r.outside_scope.iter().any(|p| p.starts_with("view")));
    assert!(r.outside_scope.iter().any(|p| p.contains("extra")));
}

#[test]
fn generated_and_virtual_tables_are_not_accepted_as_ordinary_tables() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE generated(a INTEGER, b INTEGER GENERATED ALWAYS AS (a+1) STORED); CREATE VIRTUAL TABLE virtual_table USING fts5(body);").unwrap();
    let s = snapshot(vec![
        table("generated", vec![col("a", "INTEGER"), col("b", "INTEGER")]),
        table("virtual_table", vec![col("body", "")]),
    ]);
    let obs = observe(&conn, "main", &[]).unwrap();
    let r = validate(&s, &obs, policy(ValidationScope::RequiredSubset)).unwrap();
    assert!(!r.is_match());
    assert!(
        r.differences
            .iter()
            .filter(|d| d.path.ends_with("ordinary_table"))
            .count()
            >= 2
    );
}

#[test]
fn schema_and_index_names_are_case_insensitively_unique_and_keys_validated() {
    let mut s = fixture();
    s.tables.to_mut()[1].indexes.to_mut()[0].name = id("PARENT");
    assert!(create_plan(&s).is_err());
    s = fixture();
    s.tables.to_mut()[0].name = id("sqlite_reserved");
    assert!(create_plan(&s).is_err());
    s = fixture();
    s.tables.to_mut()[1].foreign_keys.to_mut()[0].parent_columns = vec![id("id")].into();
    assert!(create_plan(&s).is_err());
    s = fixture();
    s.tables.to_mut()[1].unique_constraints = vec![vec![]].into();
    assert!(create_plan(&s).is_err());
    s = fixture();
    s.tables.to_mut()[0].columns = vec![].into();
    assert!(create_plan(&s).is_err());
}

#[test]
fn file_database_upgrade_failure_and_restart_preserve_existing_records() {
    struct TempDatabase(std::path::PathBuf);
    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let file = TempDatabase(std::env::temp_dir().join(format!(
            "simple-server-schema-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    let mut conn = Connection::open(&file.0).unwrap();
    conn.execute_batch("CREATE TABLE t(id INTEGER PRIMARY KEY, body TEXT); INSERT INTO t VALUES(1,'keep'); PRAGMA user_version=100000;").unwrap();
    let mut t = table(
        "t",
        vec![
            col("id", "INTEGER"),
            col("body", "TEXT"),
            col("revision", "INTEGER"),
        ],
    );
    t.primary_key = vec![id("id")].into();
    t.columns.to_mut()[2].default = Some(sql("1"));
    let s = snapshot(vec![t]);
    assert!(!report(&conn, &s).is_match());
    {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(
            "ALTER TABLE t ADD COLUMN revision INTEGER DEFAULT 1; PRAGMA user_version=100001;",
        )
        .unwrap();
        assert!(report(&tx, &s).is_match());
        assert!(tx.execute_batch("INSERT INTO missing VALUES(1)").is_err());
    }
    drop(conn);
    let conn = Connection::open(&file.0).unwrap();
    assert!(!report(&conn, &s).is_match());
    assert_eq!(
        conn.query_row("SELECT body FROM t WHERE id=1", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        100000
    );
    conn.execute_batch("BEGIN IMMEDIATE; ALTER TABLE t ADD COLUMN revision INTEGER DEFAULT 1; PRAGMA user_version=100001; COMMIT;").unwrap();
    drop(conn);
    let conn = Connection::open(&file.0).unwrap();
    assert!(report(&conn, &s).is_match());
    assert_eq!(
        conn.query_row("SELECT body, revision FROM t", [], |r| Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?
        )))
        .unwrap(),
        ("keep".into(), 1)
    );
}
