use rusqlite::Connection;
use simple_server::database::sqlite::schema::*;
use std::borrow::Cow;
fn id(s: &str) -> Identifier<'static> {
    Identifier::new(s.to_owned()).unwrap()
}
fn main() {
    // SAFETY: sqlite-vec exports the SQLite extension entry point; register once
    // in this standalone process before opening its sole SQLite connection.
    unsafe {
        assert_eq!(
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
                *const (),
                unsafe extern "C" fn(
                    *mut rusqlite::ffi::sqlite3,
                    *mut *mut std::ffi::c_char,
                    *const rusqlite::ffi::sqlite3_api_routines,
                ) -> std::ffi::c_int,
            >(
                sqlite_vec::sqlite3_vec_init as *const ()
            ))),
            0
        );
    }
    let conn = Connection::open_in_memory().unwrap();
    let snapshot = ExtendedSchemaSnapshot::new(SchemaSnapshot {
        namespace: "vec-canary".into(),
        database: id("main"),
        version: 0,
        tables: Cow::Borrowed(&[]),
    });
    for dimensions in [2, 3] {
        let options = CreationOptions {
            virtual_tables: vec![VirtualTableSpec {
                name: id(&format!("vec_emb_{dimensions}")),
                module: id("vec0"),
                arguments: Some(
                    VirtualTableArguments::trusted(format!(
                        "embedding_id TEXT PRIMARY KEY, embedding float[{dimensions}]"
                    ))
                    .unwrap(),
                ),
            }]
            .into(),
            ..Default::default()
        };
        let plan =
            create_extended_plan_with_options(&snapshot, CreationMode::IfNotExists, &options)
                .unwrap();
        conn.execute_batch(&plan.statements.join("\n")).unwrap();
        let vector = if dimensions == 2 {
            "[0.1,0.2]"
        } else {
            "[0.1,0.2,0.3]"
        };
        conn.execute(
            &format!(
                "INSERT INTO vec_emb_{dimensions}(embedding_id,embedding) VALUES('a',vec_f32(?1))"
            ),
            [vector],
        )
        .unwrap();
        conn.execute_batch(&plan.statements.join("\n")).unwrap();
        let found:String=conn.query_row(&format!("SELECT embedding_id FROM vec_emb_{dimensions} WHERE embedding MATCH vec_f32(?1) AND k=1"), [vector], |r|r.get(0)).unwrap();
        assert_eq!(found, "a");
        assert!(
            conn.execute(
                &format!("INSERT INTO vec_emb_{dimensions} VALUES('bad',vec_f32('[1]'))"),
                []
            )
            .is_err()
        );
    }
    println!(
        "sqlite-vec 0.1.6: generic vec0 creation, 2/3 dimensions, nearest-neighbor search, idempotence and dimension rejection pass"
    );
}
