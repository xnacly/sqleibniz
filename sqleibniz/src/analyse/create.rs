use crate::{
    error::Error,
    parser::nodes::{Alter, CreateTable, Node},
    parser::nodes::{ColumnConstraint, ColumnDef, TableConstraint},
    types::{rules::Rule, storage::SqliteStorageClass},
};

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
        analyse::create::{alter, create_table},
        parser::nodes::{
            Alter, ColumnConstraint, ColumnDef, CreateTable, IndexedColumn, SchemaTableContainer,
            TableConstraint,
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
