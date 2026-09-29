//! Build an inspectable schema plan without opening a database or HTTP server.
use simple_server::database::sqlite::schema::{
    ColumnSpec, Identifier, SchemaSnapshot, SqlExpression, TableSpec, create_plan,
};
use std::borrow::Cow;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let columns = [
        ColumnSpec {
            name: Identifier::new("id")?,
            declared_type: Cow::Borrowed("INTEGER"),
            not_null: false,
            default: None,
        },
        ColumnSpec {
            name: Identifier::new("title")?,
            declared_type: Cow::Borrowed("TEXT"),
            not_null: true,
            default: Some(SqlExpression::trusted("'untitled'")?),
        },
    ];
    let keys = [Identifier::new("id")?];
    let tables = [TableSpec {
        name: Identifier::new("items")?,
        columns: Cow::Borrowed(&columns),
        primary_key: Cow::Borrowed(&keys),
        unique_constraints: Cow::Borrowed(&[]),
        foreign_keys: Cow::Borrowed(&[]),
        indexes: Cow::Borrowed(&[]),
        unsupported: Cow::Borrowed(&[]),
    }];
    let schema = SchemaSnapshot {
        namespace: Cow::Borrowed("example/catalog"),
        database: Identifier::new("main")?,
        version: 1,
        tables: Cow::Borrowed(&tables),
    };
    for statement in create_plan(&schema)?.statements {
        println!("{statement}");
    }
    Ok(())
}
