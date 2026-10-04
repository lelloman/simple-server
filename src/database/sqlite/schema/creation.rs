//! Optional creation descriptions beyond ordinary table/index definitions.
//! This API generates trusted application DDL; it makes no validation claim.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoIncrementSpec<'a> {
    pub table: Identifier<'a>,
    pub column: Identifier<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fts5Column<'a> {
    pub name: Identifier<'a>,
    pub indexed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fts5Content<'a> {
    Internal,
    External {
        table: Identifier<'a>,
        rowid: Identifier<'a>,
    },
    Contentless,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fts5TableSpec<'a> {
    pub name: Identifier<'a>,
    pub columns: Cow<'a, [Fts5Column<'a>]>,
    pub content: Fts5Content<'a>,
    /// FTS5 tokenizer configuration, encoded as an SQL string literal.
    /// SQLite's FTS5 parser checks its syntax; this is not an SQL fragment.
    pub tokenizer: Option<Cow<'a, str>>,
    /// Token prefix lengths, in positive Unicode characters.
    pub prefixes: Cow<'a, [u32]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerTiming {
    Before,
    After,
    InsteadOf,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriggerEvent<'a> {
    Insert,
    Delete,
    /// Empty columns means every update, otherwise UPDATE OF these columns.
    Update {
        columns: Cow<'a, [Identifier<'a>]>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriggerSpec<'a> {
    pub name: Identifier<'a>,
    /// May target an application-owned table or view not in this creation plan.
    pub table: Identifier<'a>,
    pub timing: TriggerTiming,
    pub event: TriggerEvent<'a>,
    pub when: Option<SqlExpression<'a>>,
    /// Trusted application-authored trigger body, including statement terminators.
    /// No parser, sanitizer or automatic FTS synchronization policy is implied.
    /// The caller owns body syntax and semantics; newlines preserve SQL comments.
    pub body: SqlExpression<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedSchemaSnapshot<'a> {
    pub schema: SchemaSnapshot<'a>,
    pub auto_increment: Cow<'a, [AutoIncrementSpec<'a>]>,
    pub fts5_tables: Cow<'a, [Fts5TableSpec<'a>]>,
    pub triggers: Cow<'a, [TriggerSpec<'a>]>,
}
impl<'a> ExtendedSchemaSnapshot<'a> {
    pub fn new(schema: SchemaSnapshot<'a>) -> Self {
        Self {
            schema,
            auto_increment: Cow::Borrowed(&[]),
            fts5_tables: Cow::Borrowed(&[]),
            triggers: Cow::Borrowed(&[]),
        }
    }
}
/// Named or unnamed CHECK; an optional column selects column-level placement.
/// SQL expressions are trusted, unparsed and evaluated by SQLite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckConstraint<'a> {
    pub name: Option<Identifier<'a>>,
    pub column: Option<Identifier<'a>>,
    pub expression: SqlExpression<'a>,
}
impl CheckConstraint<'_> {
    pub(super) fn sql(&self) -> String {
        let prefix = self
            .name
            .as_ref()
            .map(|name| format!("CONSTRAINT {} ", name.quoted()))
            .unwrap_or_default();
        format!("{prefix}CHECK ({}\n)", self.expression.as_str())
    }
}

/// Creation-only additions for a declared ordinary table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableCreationOptions<'a> {
    pub table: Identifier<'a>,
    /// SQLite STRICT typing (requires SQLite 3.37 or newer).
    pub strict: bool,
    pub checks: Cow<'a, [CheckConstraint<'a>]>,
}

/// Trusted module argument text, not SQL values or identifiers.
/// No parser or sanitizer is implied. Never populate from untrusted input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualTableArguments<'a>(Cow<'a, str>);
impl<'a> VirtualTableArguments<'a> {
    pub fn trusted(arguments: impl Into<Cow<'a, str>>) -> Result<Self, DefinitionError> {
        let arguments = arguments.into();
        if arguments.trim().is_empty() || arguments.contains('\0') {
            return Err(invalid("virtual table arguments", "empty or contains NUL"));
        }
        Ok(Self(arguments))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Generic virtual-table creation. The caller loads/registers the module.
/// Arguments, shadow objects, syntax and runtime behavior belong to that module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualTableSpec<'a> {
    pub name: Identifier<'a>,
    pub module: Identifier<'a>,
    /// None emits no parentheses, for modules requiring no arguments.
    pub arguments: Option<VirtualTableArguments<'a>>,
}

/// Additive options; existing snapshot struct literals remain source-compatible.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CreationOptions<'a> {
    pub tables: Cow<'a, [TableCreationOptions<'a>]>,
    pub virtual_tables: Cow<'a, [VirtualTableSpec<'a>]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreationMode {
    Create,
    /// SQLite's existing-object no-op. This does not validate or repair objects.
    IfNotExists,
}
impl CreationMode {
    pub(super) fn clause(self) -> &'static str {
        match self {
            Self::Create => "",
            Self::IfNotExists => "IF NOT EXISTS ",
        }
    }
}
fn literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Return all statements or a definition error before the caller executes SQL.
/// Order: ordinary tables, indexes, FTS5 tables, triggers. No transaction, marker,
/// rebuild, backfill, PRAGMA or driver dependency is included. Bodies/expressions
/// remain trusted SQL; SQLite is the final authority for their syntax/semantics.
pub fn create_extended_plan(
    snapshot: &ExtendedSchemaSnapshot<'_>,
    mode: CreationMode,
) -> Result<CreationPlan, DefinitionError> {
    create_extended_plan_with_options(snapshot, mode, &CreationOptions::default())
}

/// Extended creation with STRICT/CHECK and generic module options.
/// All descriptors are checked before SQL is returned. Trusted SQL syntax,
/// module availability and shadow-object collisions remain engine-owned.
pub fn create_extended_plan_with_options(
    snapshot: &ExtendedSchemaSnapshot<'_>,
    mode: CreationMode,
    options: &CreationOptions<'_>,
) -> Result<CreationPlan, DefinitionError> {
    let schema = &snapshot.schema;
    validate_definition(schema)?;
    unique_names(
        "table creation options",
        options.tables.iter().map(|t| t.table.as_str()),
    )?;
    for option in options.tables.iter() {
        let table = schema
            .tables
            .iter()
            .find(|t| t.name == option.table)
            .ok_or_else(|| invalid("table creation options", "table not declared in snapshot"))?;
        if option.strict
            && table.columns.iter().any(|c| {
                !["INT", "INTEGER", "REAL", "TEXT", "BLOB", "ANY"]
                    .iter()
                    .any(|kind| c.declared_type.eq_ignore_ascii_case(kind))
            })
        {
            return Err(invalid(
                "strict table",
                "columns require INT, INTEGER, REAL, TEXT, BLOB or ANY",
            ));
        }
        unique_names(
            "check constraint names",
            option
                .checks
                .iter()
                .filter_map(|c| c.name.as_ref().map(Identifier::as_str)),
        )?;
        for check in option.checks.iter() {
            if check
                .column
                .as_ref()
                .is_some_and(|c| !table.columns.iter().any(|known| known.name == *c))
            {
                return Err(invalid("check constraint", "column not declared in table"));
            }
        }
    }
    for table in options.virtual_tables.iter() {
        if table
            .name
            .as_str()
            .to_ascii_lowercase()
            .starts_with("sqlite_")
        {
            return Err(invalid("virtual table", "reserved table name"));
        }
    }
    unique_names(
        "extended schema objects",
        schema
            .tables
            .iter()
            .flat_map(|t| {
                std::iter::once(t.name.as_str()).chain(t.indexes.iter().map(|i| i.name.as_str()))
            })
            .chain(snapshot.fts5_tables.iter().map(|t| t.name.as_str()))
            .chain(options.virtual_tables.iter().map(|t| t.name.as_str())),
    )?;
    // SQLite gives triggers a separate name namespace from tables/indexes.
    unique_names(
        "triggers",
        snapshot.triggers.iter().map(|t| t.name.as_str()),
    )?;
    unique_names(
        "autoincrement tables",
        snapshot.auto_increment.iter().map(|t| t.table.as_str()),
    )?;
    for option in snapshot.auto_increment.iter() {
        let table = schema
            .tables
            .iter()
            .find(|t| t.name == option.table)
            .ok_or_else(|| invalid("autoincrement", "table not declared in snapshot"))?;
        let column = table
            .columns
            .iter()
            .find(|c| c.name == option.column)
            .ok_or_else(|| invalid("autoincrement", "column not declared in table"))?;
        if table.primary_key.as_ref() != [option.column.clone()]
            || !column.declared_type.eq_ignore_ascii_case("INTEGER")
        {
            return Err(invalid(
                "autoincrement",
                "requires the sole INTEGER primary key",
            ));
        }
    }
    for table in snapshot.fts5_tables.iter() {
        if table
            .name
            .as_str()
            .to_ascii_lowercase()
            .starts_with("sqlite_")
            || table.columns.is_empty()
        {
            return Err(invalid("fts5", "reserved name or empty column list"));
        }
        unique_names(
            "fts5 columns",
            table.columns.iter().map(|c| c.name.as_str()),
        )?;
        if table.columns.iter().any(|c| {
            c.name.as_str().eq_ignore_ascii_case("rank")
                || c.name.as_str().eq_ignore_ascii_case("rowid")
        }) {
            return Err(invalid("fts5", "rank and rowid are reserved column names"));
        }
        if table
            .tokenizer
            .as_ref()
            .is_some_and(|s| s.trim().is_empty() || s.contains('\0'))
        {
            return Err(invalid("fts5 tokenizer", "empty or contains NUL"));
        }
        let mut prefixes = BTreeSet::new();
        if table
            .prefixes
            .iter()
            .any(|n| *n == 0 || !prefixes.insert(n))
        {
            return Err(invalid("fts5 prefixes", "zero or duplicate prefix length"));
        }
        // Reserve generated shadow names; external/contentless modes have no
        // _content table. Reject explicit collisions before returning any plan.
        for suffix in ["data", "idx", "content", "docsize", "config"] {
            if suffix == "content" && !matches!(table.content, Fts5Content::Internal) {
                continue;
            }
            let shadow = format!("{}_{suffix}", table.name.as_str());
            if schema.tables.iter().any(|t| {
                t.name.as_str().eq_ignore_ascii_case(&shadow)
                    || t.indexes
                        .iter()
                        .any(|i| i.name.as_str().eq_ignore_ascii_case(&shadow))
            }) || snapshot
                .fts5_tables
                .iter()
                .any(|t| t.name.as_str().eq_ignore_ascii_case(&shadow))
                || options
                    .virtual_tables
                    .iter()
                    .any(|t| t.name.as_str().eq_ignore_ascii_case(&shadow))
            {
                return Err(invalid(
                    "fts5 shadow tables",
                    format!("explicit object collision: {shadow}"),
                ));
            }
        }
    }
    for trigger in snapshot.triggers.iter() {
        if trigger
            .name
            .as_str()
            .to_ascii_lowercase()
            .starts_with("sqlite_")
            || trigger
                .body
                .as_str()
                .trim()
                .trim_end_matches(';')
                .trim()
                .is_empty()
        {
            return Err(invalid("trigger", "reserved name or empty body"));
        }
        if let TriggerEvent::Update { columns } = &trigger.event {
            unique_names(
                "trigger update columns",
                columns.iter().map(Identifier::as_str),
            )?;
            if let Some(table) = schema.tables.iter().find(|t| t.name == trigger.table)
                && columns
                    .iter()
                    .any(|c| !table.columns.iter().any(|known| known.name == *c))
            {
                return Err(invalid(
                    "trigger",
                    "UPDATE OF column not in declared target table",
                ));
            }
        }
    }
    let mut plan = create_ordinary_plan(
        schema,
        &snapshot.auto_increment,
        &options.tables,
        mode,
        true,
    )?;
    for table in snapshot.fts5_tables.iter() {
        let mut args = table
            .columns
            .iter()
            .map(|c| {
                format!(
                    "{}{}",
                    c.name.quoted(),
                    if c.indexed { "" } else { " UNINDEXED" }
                )
            })
            .collect::<Vec<_>>();
        match &table.content {
            Fts5Content::Internal => {}
            Fts5Content::Contentless => args.push("content=''".into()),
            Fts5Content::External { table, rowid } => {
                args.push(format!("content={}", literal(table.as_str())));
                args.push(format!("content_rowid={}", literal(rowid.as_str())));
            }
        }
        if let Some(tokenizer) = &table.tokenizer {
            args.push(format!("tokenize={}", literal(tokenizer)));
        }
        if !table.prefixes.is_empty() {
            args.push(format!(
                "prefix={}",
                literal(
                    &table
                        .prefixes
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            ));
        }
        plan.statements.push(format!(
            "CREATE VIRTUAL TABLE {}{}.{} USING fts5({});",
            mode.clause(),
            schema.database.quoted(),
            table.name.quoted(),
            args.join(", ")
        ));
    }
    for table in options.virtual_tables.iter() {
        let arguments = table
            .arguments
            .as_ref()
            .map(|args| format!("({}\n)", args.as_str()))
            .unwrap_or_default();
        plan.statements.push(format!(
            "CREATE VIRTUAL TABLE {}{}.{} USING {}{arguments};",
            mode.clause(),
            schema.database.quoted(),
            table.name.quoted(),
            table.module.quoted()
        ));
    }
    for trigger in snapshot.triggers.iter() {
        let timing = match trigger.timing {
            TriggerTiming::Before => "BEFORE",
            TriggerTiming::After => "AFTER",
            TriggerTiming::InsteadOf => "INSTEAD OF",
        };
        let event = match &trigger.event {
            TriggerEvent::Insert => "INSERT".into(),
            TriggerEvent::Delete => "DELETE".into(),
            TriggerEvent::Update { columns } if columns.is_empty() => "UPDATE".into(),
            TriggerEvent::Update { columns } => format!("UPDATE OF {}", identifiers(columns)),
        };
        let when = trigger
            .when
            .as_ref()
            .map(|s| format!(" WHEN {}", s.as_str()))
            .unwrap_or_default();
        let body = trigger.body.as_str();
        plan.statements.push(format!(
            "CREATE TRIGGER {}{}.{} {timing} {event} ON {}{when}\nBEGIN\n{body}\nEND;",
            mode.clause(),
            schema.database.quoted(),
            trigger.name.quoted(),
            trigger.table.quoted()
        ));
    }
    Ok(plan)
}
