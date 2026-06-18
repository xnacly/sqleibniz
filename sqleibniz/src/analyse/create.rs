use crate::{
    error::Error,
    parser::nodes::{Alter, CreateTable, CreateVirtualTable, Node},
    parser::nodes::{ColumnConstraint, ColumnDef, TableConstraint},
    types::{rules::Rule, storage::SqliteStorageClass},
};

#[derive(Clone, Copy)]
struct VirtualTableModule {
    name: &'static str,
    doc: &'static str,
    create_virtual_table: bool,
}

// https://www.sqlite.org/vtablist.html
const VIRTUAL_TABLE_MODULES: &[VirtualTableModule] = &[
    VirtualTableModule {
        name: "bytecode",
        doc: "https://www.sqlite.org/bytecodevtab.html",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "carray",
        doc: "https://www.sqlite.org/carray.html",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "closure",
        doc: "https://sqlite.org/src/file/ext/misc/closure.c",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "completion",
        doc: "https://www.sqlite.org/completion.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "csv",
        doc: "https://www.sqlite.org/csv.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "dbstat",
        doc: "https://www.sqlite.org/dbstat.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "files_of_checkin",
        doc: "https://fossil-scm.org/fossil/file/src/foci.c",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "fsdir",
        doc: "https://sqlite.org/src/file/ext/misc/fileio.c",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "fts3",
        doc: "https://www.sqlite.org/fts3.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "fts4",
        doc: "https://www.sqlite.org/fts3.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "fts5",
        doc: "https://www.sqlite.org/fts5.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "generate_series",
        doc: "https://www.sqlite.org/series.html",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "json_each",
        doc: "https://www.sqlite.org/json1.html#jeach",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "json_tree",
        doc: "https://www.sqlite.org/json1.html#jtree",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "pragma",
        doc: "https://www.sqlite.org/pragma.html#pragfunc",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "rtree",
        doc: "https://www.sqlite.org/rtree.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "spellfix1",
        doc: "https://www.sqlite.org/spellfix1.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "sqlite_btreeinfo",
        doc: "https://sqlite.org/src/file/ext/misc/btreeinfo.c",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "sqlite_dbpage",
        doc: "https://www.sqlite.org/dbpage.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "sqlite_memstat",
        doc: "https://www.sqlite.org/memstat.html",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "sqlite_stmt",
        doc: "https://www.sqlite.org/stmt.html",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "swarmvtab",
        doc: "https://www.sqlite.org/swarmvtab.html#overview",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "tables_used",
        doc: "https://www.sqlite.org/bytecodevtab.html",
        create_virtual_table: false,
    },
    VirtualTableModule {
        name: "tclvar",
        doc: "https://sqlite.org/src/file/src/test_tclvar.c",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "templatevtab",
        doc: "https://sqlite.org/src/file/ext/misc/templatevtab.c",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "unionvtab",
        doc: "https://www.sqlite.org/unionvtab.html",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "vfsstat",
        doc: "https://sqlite.org/src/file/ext/misc/vfsstat.c",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "vtablog",
        doc: "https://sqlite.org/src/file/ext/misc/vtablog.c",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "wholenumber",
        doc: "https://sqlite.org/src/file/ext/misc/wholenumber.c",
        create_virtual_table: true,
    },
    VirtualTableModule {
        name: "zipfile",
        doc: "https://www.sqlite.org/zipfile.html",
        create_virtual_table: true,
    },
];

pub fn create_table(file: &str, table: &CreateTable) -> Vec<Error> {
    let mut diagnostics = table
        .columns
        .iter()
        .flat_map(|column| column.analyse(file))
        .collect::<Vec<_>>();

    diagnostics.append(&mut nullable_primary_key_diagnostics(file, table));

    if table.strict {
        return diagnostics;
    }

    diagnostics.push(
        Error::new(
            file,
            table.location,
            Rule::Quirk,
            "Consider using a STRICT table",
            "SQLite tables use flexible typing by default. Add STRICT after the column list to enforce declared column types.",
        )
        .with_doc_url("https://www.sqlite.org/stricttables.html"),
    );

    diagnostics
}

pub fn create_virtual_table(file: &str, table: &CreateVirtualTable) -> Vec<Error> {
    let Some(module) = VIRTUAL_TABLE_MODULES
        .iter()
        .find(|module| module.name.eq_ignore_ascii_case(&table.module))
    else {
        // Applications can register their own virtual table modules, and SQLite's list is not
        // exhaustive. Unknown modules are valid syntax and should not be diagnosed here.
        return Vec::new();
    };

    if module.create_virtual_table {
        return Vec::new();
    }

    vec![
        Error::new(
            file,
            table.location,
            Rule::SqliteUnsupported,
            format!("`{}` is not documented for CREATE VIRTUAL TABLE", table.module),
            "SQLite documents this virtual table module as a table-valued function or built-in virtual table, not as a module for CREATE VIRTUAL TABLE.",
        )
        .with_doc_url(module.doc),
    ]
}

fn nullable_primary_key_diagnostics(file: &str, table: &CreateTable) -> Vec<Error> {
    if table.strict || table.without_rowid {
        return Vec::new();
    }

    let mut diagnostics = table
        .columns
        .iter()
        .filter(|column| is_nullable_column_primary_key(column))
        .map(|column| nullable_primary_key_error(file, column))
        .collect::<Vec<_>>();

    for constraint in &table.table_constraints {
        let TableConstraint::PrimaryKey { columns, .. } = constraint else {
            continue;
        };

        diagnostics.extend(
            columns
                .iter()
                .filter_map(|pk_column| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == pk_column.name)
                })
                .filter(|column| !column_has_not_null(column))
                .filter(|column| !is_integer_primary_key(column))
                .map(|column| nullable_primary_key_error(file, column)),
        );
    }

    diagnostics
}

fn is_nullable_column_primary_key(column: &ColumnDef) -> bool {
    column
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, ColumnConstraint::PrimaryKey { .. }))
        && !column_has_not_null(column)
        && !is_integer_primary_key(column)
}

fn column_has_not_null(column: &ColumnDef) -> bool {
    column
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, ColumnConstraint::NotNull { .. }))
}

fn is_integer_primary_key(column: &ColumnDef) -> bool {
    matches!(column.type_name, Some(SqliteStorageClass::Integer))
        && column
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, ColumnConstraint::PrimaryKey { .. }))
}

fn nullable_primary_key_error(file: &str, column: &ColumnDef) -> Error {
    Error::new(
        file,
        column.location,
        Rule::Quirk,
        format!("Primary key column `{}` may contain NULL", column.name),
        "SQLite allows NULL values in PRIMARY KEY columns unless the table is STRICT, WITHOUT ROWID, the column is INTEGER PRIMARY KEY, or the column is explicitly NOT NULL.",
    )
    .with_doc_url("https://www.sqlite.org/quirks.html#primary_keys_can_sometimes_contain_nulls")
}

pub fn alter(file: &str, alter: &Alter) -> Vec<Error> {
    alter
        .add_column
        .as_ref()
        .map(|column| column.analyse(file))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::{
        analyse::create::{alter, create_table, create_virtual_table},
        parser::nodes::{
            Alter, ColumnConstraint, ColumnDef, CreateTable, CreateVirtualTable, IndexedColumn,
            SchemaTableContainer, TableConstraint,
        },
        types::{rules::Rule, storage::SqliteStorageClass},
    };

    fn create_table_node(columns: Vec<ColumnDef>, strict: bool) -> CreateTable {
        CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            columns,
            vec![],
            strict,
            false,
        )
    }

    fn create_virtual_table_node(module: &str) -> CreateVirtualTable {
        CreateVirtualTable::new(
            false,
            false,
            SchemaTableContainer::Table("items".into()),
            module.into(),
            vec![],
        )
    }

    #[test]
    fn recommends_strict_for_non_strict_tables() {
        let table = create_table_node(
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![],
            )],
            false,
        );

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::Quirk);
        assert!(diagnostics[0].note.contains("Add STRICT"));
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/stricttables.html")
        );
    }

    #[test]
    fn accepts_strict_tables_without_column_diagnostics() {
        let table = create_table_node(
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![],
            )],
            true,
        );

        let diagnostics = create_table("test.sql", &table);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn accepts_known_create_virtual_table_modules() {
        for module in ["fts5", "rtree", "dbstat", "zipfile"] {
            let table = create_virtual_table_node(module);
            let diagnostics = create_virtual_table("test.sql", &table);

            assert!(
                diagnostics.is_empty(),
                "expected {module} to be accepted, got {diagnostics:?}"
            );
        }
    }

    #[test]
    fn accepts_unknown_create_virtual_table_modules() {
        let table = create_virtual_table_node("application_defined_module");
        let diagnostics = create_virtual_table("test.sql", &table);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_table_valued_function_modules_in_create_virtual_table() {
        let table = create_virtual_table_node("json_each");
        let diagnostics = create_virtual_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::SqliteUnsupported);
        assert!(
            diagnostics[0]
                .note
                .contains("not as a module for CREATE VIRTUAL TABLE")
        );
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/json1.html#jeach")
        );
    }

    #[test]
    fn includes_column_diagnostics_for_create_table_columns() {
        let table = create_table_node(vec![ColumnDef::new("name".into(), None, vec![])], true);

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::Quirk);
        assert!(
            diagnostics[0]
                .note
                .contains("SQLite allows columns without a declared type")
        );
    }

    #[test]
    fn includes_column_and_strict_diagnostics_for_non_strict_tables() {
        let table = create_table_node(vec![ColumnDef::new("name".into(), None, vec![])], false);

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .note
                .contains("SQLite allows columns without a declared type")
        }));
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.note.contains("Add STRICT"))
        );
    }

    #[test]
    fn reports_nullable_column_primary_key() {
        let table = create_table_node(
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            false,
        );

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 2);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.note.contains("PRIMARY KEY columns"))
            .unwrap();
        assert_eq!(diagnostic.rule, Rule::Quirk);
        assert_eq!(
            diagnostic.doc_url,
            Some("https://www.sqlite.org/quirks.html#primary_keys_can_sometimes_contain_nulls")
        );
    }

    #[test]
    fn accepts_integer_primary_key() {
        let table = create_table_node(
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            false,
        );

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].note.contains("Add STRICT"));
    }

    #[test]
    fn accepts_not_null_column_primary_key() {
        let table = create_table_node(
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Text),
                vec![
                    ColumnConstraint::PrimaryKey {
                        asc_desc: None,
                        on_conflict: None,
                        autoincrement: false,
                    },
                    ColumnConstraint::NotNull { on_conflict: None },
                ],
            )],
            true,
        );

        let diagnostics = create_table("test.sql", &table);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_nullable_table_primary_key_columns() {
        let table = CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![],
            )],
            vec![TableConstraint::PrimaryKey {
                columns: vec![IndexedColumn {
                    name: "id".into(),
                    collation: None,
                    order: None,
                }],
                on_conflict: None,
            }],
            false,
            false,
        );

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.note.contains("PRIMARY KEY columns"))
        );
    }

    #[test]
    fn accepts_nullable_primary_key_in_without_rowid_table() {
        let table = CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            vec![],
            false,
            true,
        );

        let diagnostics = create_table("test.sql", &table);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].note.contains("Add STRICT"));
    }

    #[test]
    fn includes_column_diagnostics_for_alter_add_column() {
        let alter_node = Alter::new(
            SchemaTableContainer::Table("users".into()),
            None,
            None,
            None,
            Some(ColumnDef::new("name".into(), None, vec![])),
            None,
        );

        let diagnostics = alter("test.sql", &alter_node);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::Quirk);
        assert!(
            diagnostics[0]
                .note
                .contains("SQLite allows columns without a declared type")
        );
    }

    #[test]
    fn accepts_alter_without_add_column() {
        let alter_node = Alter::new(
            SchemaTableContainer::Table("users".into()),
            Some("people".into()),
            None,
            None,
            None,
            None,
        );

        let diagnostics = alter("test.sql", &alter_node);

        assert!(diagnostics.is_empty());
    }
}
