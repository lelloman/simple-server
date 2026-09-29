//! Driver-neutral SQLite schema descriptions and read-only comparison.
//!
//! Adapters supply observations under an application-owned consistency boundary.
//! Never turn metadata query failures into empty collections: return the original
//! driver error instead. Unknown capabilities use [`Observed::Unavailable`](crate::database::sqlite::schema::Observed::Unavailable).
//! This module does not open connections, execute SQL or write version markers.

use std::{borrow::Cow, collections::BTreeSet, fmt};

/// An SQLite identifier, quoted independently of trusted SQL expressions.
#[derive(Clone, Debug)]
pub struct Identifier<'a>(Cow<'a, str>);

impl<'a> Identifier<'a> {
    pub fn new(value: impl Into<Cow<'a, str>>) -> Result<Self, DefinitionError> {
        let value = value.into();
        if value.is_empty() || value.contains('\0') {
            return Err(invalid("identifier", "empty or contains NUL"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn quoted(&self) -> String {
        format!("\"{}\"", self.0.replace('"', "\"\""))
    }
}

// SQLite identifiers use ASCII case-insensitive matching. Preserve spelling for
// diagnostics and generated SQL; do not case-fold expressions or Unicode text.
impl PartialEq for Identifier<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}
impl Eq for Identifier<'_> {}

/// Trusted application-authored SQL, never implicitly converted from user input.
/// This is not an SQL parser or sanitizer. The caller owns syntax and semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlExpression<'a>(Cow<'a, str>);
impl<'a> SqlExpression<'a> {
    pub fn trusted(sql: impl Into<Cow<'a, str>>) -> Result<Self, DefinitionError> {
        let sql = sql.into();
        if sql.trim().is_empty() || sql.contains('\0') {
            return Err(invalid("expression", "empty or contains NUL"));
        }
        Ok(Self(sql))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSpec<'a> {
    pub name: Identifier<'a>,
    /// Exact declared type text, not a Rust type or inferred SQLite affinity.
    /// Empty text represents an untyped column. Generation quotes nonempty types.
    pub declared_type: Cow<'a, str>,
    /// The metadata NOT NULL flag, not inferred non-nullability from a key.
    pub not_null: bool,
    pub default: Option<SqlExpression<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForeignKeyAction {
    NoAction,
    Restrict,
    SetNull,
    SetDefault,
    Cascade,
}
impl ForeignKeyAction {
    fn sql(self) -> &'static str {
        match self {
            Self::NoAction => "NO ACTION",
            Self::Restrict => "RESTRICT",
            Self::SetNull => "SET NULL",
            Self::SetDefault => "SET DEFAULT",
            Self::Cascade => "CASCADE",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignKeySpec<'a> {
    pub columns: Cow<'a, [Identifier<'a>]>,
    pub parent_table: Identifier<'a>,
    pub parent_columns: Cow<'a, [Identifier<'a>]>,
    pub on_update: ForeignKeyAction,
    pub on_delete: ForeignKeyAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexTerm<'a> {
    pub column: Identifier<'a>,
    /// Specify BINARY explicitly for SQLite's ordinary default collation.
    pub collation: Identifier<'a>,
    pub descending: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexSpec<'a> {
    pub name: Identifier<'a>,
    pub terms: Cow<'a, [IndexTerm<'a>]>,
    pub unique: bool,
    pub predicate: Option<SqlExpression<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableSpec<'a> {
    pub name: Identifier<'a>,
    pub columns: Cow<'a, [ColumnSpec<'a>]>,
    /// Ordered primary-key column names. Empty means no primary key.
    pub primary_key: Cow<'a, [Identifier<'a>]>,
    /// Ordered columns within each unique constraint; constraint order is immaterial.
    pub unique_constraints: Cow<'a, [Vec<Identifier<'a>>]>,
    pub foreign_keys: Cow<'a, [ForeignKeySpec<'a>]>,
    /// Explicit CREATE INDEX objects; exclude SQLite's implicit constraint indexes.
    pub indexes: Cow<'a, [IndexSpec<'a>]>,
    /// Requirements this version cannot describe (e.g. triggers or CHECK clauses).
    /// Validation reports them as unverified; creation rejects them before SQL.
    pub unsupported: Cow<'a, [String]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaSnapshot<'a> {
    /// Application identity, distinct from the attached SQLite database name.
    pub namespace: Cow<'a, str>,
    pub database: Identifier<'a>,
    /// Logical application version, never automatically persisted or inferred.
    pub version: u64,
    pub tables: Cow<'a, [TableSpec<'a>]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionError {
    pub path: String,
    pub reason: String,
}
impl fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.reason)
    }
}
impl std::error::Error for DefinitionError {}
fn invalid(path: impl Into<String>, reason: impl Into<String>) -> DefinitionError {
    DefinitionError {
        path: path.into(),
        reason: reason.into(),
    }
}

/// A value known to the adapter, or a property it cannot inspect.
/// Query failures must be returned as errors by the adapter, not represented here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observed<T> {
    Known(T),
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableObservation<'a> {
    pub name: Identifier<'a>,
    pub columns: Observed<Vec<ColumnSpec<'a>>>,
    pub primary_key: Observed<Vec<Identifier<'a>>>,
    pub unique_constraints: Observed<Vec<Vec<Identifier<'a>>>>,
    pub foreign_keys: Observed<Vec<ForeignKeySpec<'a>>>,
    pub indexes: Observed<Vec<IndexSpec<'a>>>,
    /// Known(()) asserts an ordinary rowid table with no additional properties
    /// outside the represented model. Metadata PRAGMAs alone may not establish
    /// this: the adapter must inspect DDL or explicitly report unavailable.
    /// Generated columns, STRICT/WITHOUT ROWID, checks, non-default column
    /// collations, conflict clauses, AUTOINCREMENT and FK deferral require this
    /// capability to be Unavailable until the adapter can model them faithfully.
    pub ordinary_table: Observed<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaObject<'a> {
    pub kind: Cow<'a, str>,
    pub name: Identifier<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaObservation<'a> {
    pub namespace: Cow<'a, str>,
    pub database: Identifier<'a>,
    /// Complete ordinary table inventory, not just tables in the expected snapshot.
    pub tables: Observed<Vec<TableObservation<'a>>>,
    /// Views, triggers, virtual tables and other objects outside the ordinary model.
    /// Exclude SQLite internal objects, but not application-owned unknown objects.
    pub other_objects: Observed<Vec<SchemaObject<'a>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationScope {
    Exact,
    RequiredSubset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnOrder {
    Ordered,
    ByName,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeComparison {
    Exact,
    AsciiCaseInsensitive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionComparison {
    Exact,
    TrimWhitespace,
}

/// No default policy: consumers must select their comparison semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationPolicy {
    pub scope: ValidationScope,
    pub column_order: ColumnOrder,
    pub declared_types: TypeComparison,
    /// Only trims outer ASCII whitespace; never changes parentheses or literals.
    pub expressions: ExpressionComparison,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DifferenceKind {
    Missing,
    Unexpected,
    Mismatch,
    Unverified,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaDifference {
    pub path: String,
    pub kind: DifferenceKind,
    pub expected: Option<String>,
    pub observed: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaReport {
    pub namespace: String,
    pub database: String,
    pub snapshot_version: u64,
    pub policy: ValidationPolicy,
    /// Property paths that were compared, not a claim about row data or integrity.
    pub checked: Vec<String>,
    pub differences: Vec<SchemaDifference>,
    /// Extra objects/properties explicitly permitted by RequiredSubset.
    /// They were inventoried, not structurally verified.
    pub outside_scope: Vec<String>,
}
impl SchemaReport {
    /// False for unavailable/unsupported checks as well as concrete differences.
    /// A true subset result verifies only the requested requirements.
    pub fn is_match(&self) -> bool {
        self.differences.is_empty()
    }

    fn difference(
        &mut self,
        path: &str,
        kind: DifferenceKind,
        expected: Option<String>,
        observed: Option<String>,
    ) {
        self.differences.push(SchemaDifference {
            path: path.into(),
            kind,
            expected,
            observed,
        });
    }
    fn compare<T: fmt::Debug + PartialEq>(&mut self, path: &str, expected: &T, actual: &T) {
        self.checked.push(path.into());
        if expected != actual {
            self.difference(
                path,
                DifferenceKind::Mismatch,
                Some(format!("{expected:?}")),
                Some(format!("{actual:?}")),
            );
        }
    }
    fn known<'a, T>(&mut self, path: &str, value: &'a Observed<T>) -> Option<&'a T> {
        match value {
            Observed::Known(v) => Some(v),
            Observed::Unavailable(reason) => {
                self.difference(path, DifferenceKind::Unverified, None, Some(reason.clone()));
                None
            }
        }
    }
}

fn unique_names<'a>(
    path: &str,
    names: impl IntoIterator<Item = &'a str>,
) -> Result<(), DefinitionError> {
    let mut seen = BTreeSet::new();
    for name in names {
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(invalid(path, format!("duplicate name {name:?}")));
        }
    }
    Ok(())
}

fn validate_definition(snapshot: &SchemaSnapshot<'_>) -> Result<(), DefinitionError> {
    if snapshot.namespace.trim().is_empty() || snapshot.namespace.contains('\0') {
        return Err(invalid("namespace", "empty or contains NUL"));
    }
    unique_names(
        "schema objects",
        snapshot.tables.iter().flat_map(|t| {
            std::iter::once(t.name.as_str()).chain(t.indexes.iter().map(|i| i.name.as_str()))
        }),
    )?;
    for table in snapshot.tables.iter() {
        let path = format!("table[{:?}]", table.name.as_str());
        if table
            .name
            .as_str()
            .to_ascii_lowercase()
            .starts_with("sqlite_")
        {
            return Err(invalid(&path, "reserved SQLite object name"));
        }
        if table.columns.is_empty() {
            return Err(invalid(&path, "table has no columns"));
        }
        unique_names(&path, table.columns.iter().map(|c| c.name.as_str()))?;
        for c in table.columns.iter() {
            if c.declared_type.contains('\0') {
                return Err(invalid(&path, "type contains NUL"));
            }
        }
        let references = |names: &[Identifier<'_>]| -> Result<(), DefinitionError> {
            unique_names(&path, names.iter().map(Identifier::as_str))?;
            for name in names {
                if !table.columns.iter().any(|c| c.name == *name) {
                    return Err(invalid(
                        &path,
                        format!("unknown column {:?}", name.as_str()),
                    ));
                }
            }
            Ok(())
        };
        references(&table.primary_key)?;
        for columns in table.unique_constraints.iter() {
            if columns.is_empty() {
                return Err(invalid(&path, "empty unique constraint"));
            }
            references(columns)?;
        }
        for fk in table.foreign_keys.iter() {
            if fk.columns.is_empty() || fk.columns.len() != fk.parent_columns.len() {
                return Err(invalid(
                    &path,
                    "foreign key requires matching nonempty column lists",
                ));
            }
            references(&fk.columns)?;
            unique_names(&path, fk.parent_columns.iter().map(Identifier::as_str))?;
        }
        for idx in table.indexes.iter() {
            if idx
                .name
                .as_str()
                .to_ascii_lowercase()
                .starts_with("sqlite_")
                || idx.terms.is_empty()
            {
                return Err(invalid(&path, "reserved index name or empty index"));
            }
            for term in idx.terms.iter() {
                references(std::slice::from_ref(&term.column))?;
            }
        }
        for (i, columns) in table.unique_constraints.iter().enumerate() {
            if table.unique_constraints[..i].contains(columns) {
                return Err(invalid(&path, "duplicate unique constraint"));
            }
        }
        for (i, fk) in table.foreign_keys.iter().enumerate() {
            if table.foreign_keys[..i].contains(fk) {
                return Err(invalid(&path, "duplicate foreign key"));
            }
        }
    }
    Ok(())
}

/// Deterministic statements in input table/index order. No transaction, PRAGMA,
/// marker write, IF NOT EXISTS, repair or destructive migration is included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationPlan {
    pub statements: Vec<String>,
}
fn identifiers(names: &[Identifier<'_>]) -> String {
    names
        .iter()
        .map(Identifier::quoted)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn create_plan(snapshot: &SchemaSnapshot<'_>) -> Result<CreationPlan, DefinitionError> {
    validate_definition(snapshot)?;
    for t in snapshot.tables.iter() {
        if !t.unsupported.is_empty() {
            return Err(invalid(
                t.name.as_str(),
                format!("unsupported definitions: {:?}", t.unsupported),
            ));
        }
    }
    let mut statements = Vec::new();
    for t in snapshot.tables.iter() {
        let mut parts = Vec::new();
        for c in t.columns.iter() {
            let mut part = c.name.quoted();
            if !c.declared_type.is_empty() {
                part.push_str(&format!(" \"{}\"", c.declared_type.replace('"', "\"\"")));
            }
            if c.not_null {
                part.push_str(" NOT NULL");
            }
            if let Some(default) = &c.default {
                part.push_str(&format!(" DEFAULT ({})", default.as_str()));
            }
            parts.push(part);
        }
        if !t.primary_key.is_empty() {
            parts.push(format!("PRIMARY KEY ({})", identifiers(&t.primary_key)));
        }
        for columns in t.unique_constraints.iter() {
            parts.push(format!("UNIQUE ({})", identifiers(columns)));
        }
        for fk in t.foreign_keys.iter() {
            parts.push(format!(
                "FOREIGN KEY ({}) REFERENCES {} ({}) ON UPDATE {} ON DELETE {}",
                identifiers(&fk.columns),
                fk.parent_table.quoted(),
                identifiers(&fk.parent_columns),
                fk.on_update.sql(),
                fk.on_delete.sql()
            ));
        }
        statements.push(format!(
            "CREATE TABLE {}.{} ({});",
            snapshot.database.quoted(),
            t.name.quoted(),
            parts.join(", ")
        ));
    }
    for t in snapshot.tables.iter() {
        for idx in t.indexes.iter() {
            let terms = idx
                .terms
                .iter()
                .map(|term| {
                    format!(
                        "{} COLLATE {} {}",
                        term.column.quoted(),
                        term.collation.quoted(),
                        if term.descending { "DESC" } else { "ASC" }
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            statements.push(format!(
                "CREATE {}INDEX {}.{} ON {} ({}){};",
                if idx.unique { "UNIQUE " } else { "" },
                snapshot.database.quoted(),
                idx.name.quoted(),
                t.name.quoted(),
                terms,
                idx.predicate
                    .as_ref()
                    .map(|p| format!(" WHERE {}", p.as_str()))
                    .unwrap_or_default()
            ));
        }
    }
    Ok(CreationPlan { statements })
}

fn compare_set<T: PartialEq + fmt::Debug>(
    report: &mut SchemaReport,
    path: &str,
    expected: &[T],
    actual: &[T],
) {
    report.checked.push(path.into());
    // Multiset matching does not let one observed item satisfy two requirements.
    let mut used = vec![false; actual.len()];
    for e in expected {
        if let Some(i) = actual
            .iter()
            .enumerate()
            .position(|(i, a)| !used[i] && a == e)
        {
            used[i] = true;
        } else {
            report.difference(path, DifferenceKind::Missing, Some(format!("{e:?}")), None);
        }
    }
    for (i, a) in actual.iter().enumerate() {
        if !used[i] {
            if report.policy.scope == ValidationScope::Exact {
                report.difference(
                    path,
                    DifferenceKind::Unexpected,
                    None,
                    Some(format!("{a:?}")),
                );
            } else {
                report.outside_scope.push(format!("{path}: {a:?}"));
            }
        }
    }
}

fn expression<'a>(
    value: &'a Option<SqlExpression<'_>>,
    comparison: ExpressionComparison,
) -> Option<&'a str> {
    value.as_ref().map(|e| match comparison {
        ExpressionComparison::Exact => e.as_str(),
        ExpressionComparison::TrimWhitespace => {
            e.as_str().trim_matches(|c: char| c.is_ascii_whitespace())
        }
    })
}

/// Compare a snapshot with adapter-supplied metadata. Invalid definitions and
/// ambiguous duplicate observations are errors; differences are a typed report.
/// The snapshot version labels the report, not proof of ledger compatibility.
pub fn validate(
    snapshot: &SchemaSnapshot<'_>,
    observed: &SchemaObservation<'_>,
    policy: ValidationPolicy,
) -> Result<SchemaReport, DefinitionError> {
    validate_definition(snapshot)?;
    let mut report = SchemaReport {
        namespace: snapshot.namespace.to_string(),
        database: snapshot.database.as_str().into(),
        snapshot_version: snapshot.version,
        policy,
        checked: vec![],
        differences: vec![],
        outside_scope: vec![],
    };
    report.compare(
        "namespace",
        &snapshot.namespace.as_ref(),
        &observed.namespace.as_ref(),
    );
    report.compare("database", &snapshot.database, &observed.database);
    if let Some(objects) = report.known("other_objects", &observed.other_objects) {
        report.checked.push("other_objects".into());
        for object in objects {
            let path = format!("{}[{:?}]", object.kind, object.name.as_str());
            if policy.scope == ValidationScope::Exact {
                report.difference(
                    &path,
                    DifferenceKind::Unexpected,
                    None,
                    Some("outside structured coverage".into()),
                );
            } else {
                report.outside_scope.push(path);
            }
        }
    }
    let Some(tables) = report.known("tables", &observed.tables) else {
        return Ok(report);
    };
    unique_names("observed tables", tables.iter().map(|t| t.name.as_str()))?;
    report.checked.push("tables".into());
    for expected in snapshot.tables.iter() {
        let path = format!("table[{:?}]", expected.name.as_str());
        for unsupported in expected.unsupported.iter() {
            report.difference(
                &path,
                DifferenceKind::Unverified,
                Some(unsupported.clone()),
                None,
            );
        }
        let Some(actual) = tables.iter().find(|t| t.name == expected.name) else {
            report.difference(
                &path,
                DifferenceKind::Missing,
                Some(expected.name.as_str().into()),
                None,
            );
            continue;
        };
        if report
            .known(&format!("{path}.ordinary_table"), &actual.ordinary_table)
            .is_some()
        {
            report.checked.push(format!("{path}.ordinary_table"));
        }
        if let Some(columns) = report.known(&format!("{path}.columns"), &actual.columns) {
            unique_names(&path, columns.iter().map(|c| c.name.as_str()))?;
            if policy.column_order == ColumnOrder::Ordered {
                let expected_names: Vec<_> = expected.columns.iter().map(|c| &c.name).collect();
                let actual_names: Vec<_> = columns
                    .iter()
                    .filter(|c| {
                        policy.scope == ValidationScope::Exact
                            || expected.columns.iter().any(|e| e.name == c.name)
                    })
                    .map(|c| &c.name)
                    .collect();
                report.compare(
                    &format!("{path}.column_order"),
                    &expected_names,
                    &actual_names,
                );
            }
            for e in expected.columns.iter() {
                let cp = format!("{path}.column[{:?}]", e.name.as_str());
                if let Some(a) = columns.iter().find(|a| a.name == e.name) {
                    let normalize_type = |s: &str| match policy.declared_types {
                        TypeComparison::Exact => s.to_owned(),
                        TypeComparison::AsciiCaseInsensitive => s.to_ascii_uppercase(),
                    };
                    report.compare(
                        &format!("{cp}.type"),
                        &normalize_type(&e.declared_type),
                        &normalize_type(&a.declared_type),
                    );
                    report.compare(&format!("{cp}.not_null"), &e.not_null, &a.not_null);
                    report.compare(
                        &format!("{cp}.default"),
                        &expression(&e.default, policy.expressions),
                        &expression(&a.default, policy.expressions),
                    );
                } else {
                    report.difference(
                        &cp,
                        DifferenceKind::Missing,
                        Some(e.name.as_str().into()),
                        None,
                    );
                }
            }
            if policy.scope == ValidationScope::Exact {
                for a in columns {
                    if !expected.columns.iter().any(|e| e.name == a.name) {
                        report.difference(
                            &format!("{path}.column[{:?}]", a.name.as_str()),
                            DifferenceKind::Unexpected,
                            None,
                            Some(a.name.as_str().into()),
                        );
                    }
                }
            }
        }
        if let Some(pk) = report.known(&format!("{path}.primary_key"), &actual.primary_key) {
            report.compare(
                &format!("{path}.primary_key"),
                &expected.primary_key.as_ref(),
                &pk.as_slice(),
            );
        }
        if let Some(uniques) = report.known(
            &format!("{path}.unique_constraints"),
            &actual.unique_constraints,
        ) {
            compare_set(
                &mut report,
                &format!("{path}.unique_constraints"),
                &expected.unique_constraints,
                uniques,
            );
        }
        if let Some(fks) = report.known(&format!("{path}.foreign_keys"), &actual.foreign_keys) {
            compare_set(
                &mut report,
                &format!("{path}.foreign_keys"),
                &expected
                    .foreign_keys
                    .iter()
                    .map(foreign_key_identity)
                    .collect::<Vec<_>>(),
                &fks.iter().map(foreign_key_identity).collect::<Vec<_>>(),
            );
        }
        if let Some(indexes) = report.known(&format!("{path}.indexes"), &actual.indexes) {
            unique_names(&path, indexes.iter().map(|i| i.name.as_str()))?;
            for e in expected.indexes.iter() {
                let ip = format!("{path}.index[{:?}]", e.name.as_str());
                if let Some(a) = indexes.iter().find(|i| i.name == e.name) {
                    report.compare(
                        &format!("{ip}.terms"),
                        &index_terms(&e.terms),
                        &index_terms(&a.terms),
                    );
                    report.compare(&format!("{ip}.unique"), &e.unique, &a.unique);
                    report.compare(
                        &format!("{ip}.predicate"),
                        &expression(&e.predicate, policy.expressions),
                        &expression(&a.predicate, policy.expressions),
                    );
                } else {
                    report.difference(
                        &ip,
                        DifferenceKind::Missing,
                        Some(e.name.as_str().into()),
                        None,
                    );
                }
            }
            if policy.scope == ValidationScope::Exact {
                for a in indexes {
                    if !expected.indexes.iter().any(|e| e.name == a.name) {
                        report.difference(
                            &format!("{path}.index[{:?}]", a.name.as_str()),
                            DifferenceKind::Unexpected,
                            None,
                            Some(a.name.as_str().into()),
                        );
                    }
                }
            }
        }
    }
    if policy.scope == ValidationScope::RequiredSubset {
        for actual in tables {
            let path = format!("table[{:?}]", actual.name.as_str());
            if let Some(expected) = snapshot.tables.iter().find(|t| t.name == actual.name) {
                if let Observed::Known(columns) = &actual.columns {
                    for c in columns {
                        if !expected.columns.iter().any(|e| e.name == c.name) {
                            report
                                .outside_scope
                                .push(format!("{path}.column[{:?}]", c.name.as_str()));
                        }
                    }
                }
                if let Observed::Known(indexes) = &actual.indexes {
                    for i in indexes {
                        if !expected.indexes.iter().any(|e| e.name == i.name) {
                            report
                                .outside_scope
                                .push(format!("{path}.index[{:?}]", i.name.as_str()));
                        }
                    }
                }
            } else {
                report.outside_scope.push(path);
            }
        }
    }
    if policy.scope == ValidationScope::Exact {
        for actual in tables {
            if !snapshot.tables.iter().any(|t| t.name == actual.name) {
                report.difference(
                    &format!("table[{:?}]", actual.name.as_str()),
                    DifferenceKind::Unexpected,
                    None,
                    Some(actual.name.as_str().into()),
                );
            }
        }
    }
    Ok(report)
}

fn index_terms(terms: &[IndexTerm<'_>]) -> Vec<(String, String, bool)> {
    terms
        .iter()
        .map(|term| {
            (
                term.column.as_str().to_ascii_lowercase(),
                term.collation.as_str().to_ascii_lowercase(),
                term.descending,
            )
        })
        .collect()
}

#[derive(Debug, PartialEq)]
struct ForeignKeyIdentity {
    columns: Vec<String>,
    parent_table: String,
    parent_columns: Vec<String>,
    on_update: ForeignKeyAction,
    on_delete: ForeignKeyAction,
}
fn foreign_key_identity(fk: &ForeignKeySpec<'_>) -> ForeignKeyIdentity {
    ForeignKeyIdentity {
        columns: fk
            .columns
            .iter()
            .map(|c| c.as_str().to_ascii_lowercase())
            .collect(),
        parent_table: fk.parent_table.as_str().to_ascii_lowercase(),
        parent_columns: fk
            .parent_columns
            .iter()
            .map(|c| c.as_str().to_ascii_lowercase())
            .collect(),
        on_update: fk.on_update,
        on_delete: fk.on_delete,
    }
}
