use crate::{
    analyse::{Analyse, AnalysisContext, Column, RelationKind},
    error::{Error, Location},
    parser::nodes::{
        Alter, CreateTable, CreateTableAs, CreateView, CreateVirtualTable, SchemaTableContainer,
    },
    parser::nodes::{ColumnConstraint, ColumnDef, TableConstraint},
    types::{rules::Rule, storage::SqliteStorageClass},
};

mod virtual_table;

pub fn create_table(file: &str, context: &mut AnalysisContext, table: &CreateTable) -> Vec<Error> {
    let mut diagnostics = duplicate_relation_diagnostic(
        file,
        context,
        &table.name,
        table.if_not_exists,
        table.location,
        "https://www.sqlite.org/lang_createtable.html",
    );

    context.define_relation_with_columns(
        &table.name,
        RelationKind::Table,
        table.columns.iter().map(Column::from).collect(),
    );

    diagnostics.extend(
        table
            .columns
            .iter()
            .flat_map(|column| column.analyse(file, context)),
    );

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

pub fn create_table_as(
    file: &str,
    context: &mut AnalysisContext,
    table: &CreateTableAs,
) -> Vec<Error> {
    let mut diagnostics = table.select.analyse(file, context);
    diagnostics.extend(duplicate_relation_diagnostic(
        file,
        context,
        &table.name,
        table.if_not_exists,
        table.location,
        "https://www.sqlite.org/lang_createtable.html",
    ));
    context.define_relation(&table.name, RelationKind::Table);
    diagnostics
}

pub fn create_view(file: &str, context: &mut AnalysisContext, view: &CreateView) -> Vec<Error> {
    let mut diagnostics = view.select.analyse(file, context);
    diagnostics.extend(duplicate_relation_diagnostic(
        file,
        context,
        &view.name,
        view.if_not_exists,
        view.location,
        "https://www.sqlite.org/lang_createview.html",
    ));
    context.define_relation(&view.name, RelationKind::View);
    diagnostics
}

pub fn create_virtual_table(
    file: &str,
    context: &mut AnalysisContext,
    table: &CreateVirtualTable,
) -> Vec<Error> {
    let mut diagnostics = duplicate_relation_diagnostic(
        file,
        context,
        &table.name,
        table.if_not_exists,
        table.location,
        "https://www.sqlite.org/lang_createvtab.html",
    );

    context.define_relation(&table.name, RelationKind::VirtualTable);

    let Some(module) = virtual_table::module(&table.module) else {
        // Applications can register their own virtual table modules, and SQLite's list is not
        // exhaustive. Unknown modules are valid syntax and should not be diagnosed here.
        return diagnostics;
    };

    if module.create_virtual_table {
        return diagnostics;
    }

    diagnostics.push(
        Error::new(
            file,
            table.location,
            Rule::SqliteUnsupported,
            format!("`{}` is not documented for CREATE VIRTUAL TABLE", table.module),
            "SQLite documents this virtual table module as a table-valued function or built-in virtual table, not as a module for CREATE VIRTUAL TABLE.",
        )
        .with_doc_url(module.doc),
    );

    diagnostics
}

fn duplicate_relation_diagnostic(
    file: &str,
    context: &AnalysisContext,
    name: &SchemaTableContainer,
    if_not_exists: bool,
    location: Location,
    doc_url: &'static str,
) -> Vec<Error> {
    if if_not_exists || !context.contains_relation(name) {
        return Vec::new();
    }

    vec![
        Error::new(
            file,
            location,
            Rule::DuplicateRelation,
            format!("Relation `{}` is defined more than once", relation_display(name)),
            "SQLite rejects duplicate table, view, and virtual table names unless the statement uses IF NOT EXISTS.",
        )
        .with_doc_url(doc_url),
    ]
}

fn relation_display(name: &SchemaTableContainer) -> String {
    match name {
        SchemaTableContainer::Table(table) => table.clone(),
        SchemaTableContainer::SchemaAndTable { schema, table } => format!("{schema}.{table}"),
    }
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

pub fn alter(file: &str, context: &mut AnalysisContext, alter: &Alter) -> Vec<Error> {
    alter
        .add_column
        .as_ref()
        .map(|column| column.analyse(file, context))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::{
        analyse::{
            AnalysisContext, Column, RelationKind,
            create::{alter, create_table, create_virtual_table},
        },
        parser::nodes::{
            Alter, ColumnConstraint, ColumnDef, CreateTable, CreateTableAs, CreateView,
            CreateVirtualTable, IndexedColumn, ResultColumn, SchemaTableContainer, Select,
            SelectQuantifier, TableConstraint,
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

    fn typed_id_column() -> ColumnDef {
        ColumnDef::new("id".into(), Some(SqliteStorageClass::Integer), vec![])
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

    fn empty_select() -> Select {
        Select::new(
            None::<SelectQuantifier>,
            vec![ResultColumn::Star],
            vec![],
            None,
            vec![],
            None,
            vec![],
            None,
        )
    }

    fn create_table_diagnostics(
        table: &CreateTable,
    ) -> (Vec<crate::error::Error>, AnalysisContext) {
        let mut context = AnalysisContext::default();
        let diagnostics = create_table("test.sql", &mut context, table);
        (diagnostics, context)
    }

    fn create_virtual_table_diagnostics(
        table: &CreateVirtualTable,
    ) -> (Vec<crate::error::Error>, AnalysisContext) {
        let mut context = AnalysisContext::default();
        let diagnostics = create_virtual_table("test.sql", &mut context, table);
        (diagnostics, context)
    }

    fn alter_diagnostics(alter_node: &Alter) -> Vec<crate::error::Error> {
        let mut context = AnalysisContext::default();
        alter("test.sql", &mut context, alter_node)
    }

    #[test]
    fn recommends_strict_for_non_strict_tables() {
        let table = create_table_node(vec![typed_id_column()], false);

        let (diagnostics, context) = create_table_diagnostics(&table);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("users".into()))
                .unwrap()
                .kind,
            RelationKind::Table
        );
        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("users".into()))
                .unwrap()
                .columns,
            vec![Column {
                name: "id".into(),
                type_name: Some(SqliteStorageClass::Integer),
            }]
        );
        assert_eq!(diagnostics[0].rule, Rule::Quirk);
        assert!(diagnostics[0].note.contains("Add STRICT"));
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/stricttables.html")
        );
    }

    #[test]
    fn accepts_strict_tables_without_column_diagnostics() {
        let table = create_table_node(vec![typed_id_column()], true);

        let (diagnostics, _) = create_table_diagnostics(&table);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_duplicate_create_table_relations() {
        let first = create_table_node(vec![typed_id_column()], true);
        let second = create_table_node(vec![typed_id_column()], true);
        let mut context = AnalysisContext::default();

        assert!(create_table("test.sql", &mut context, &first).is_empty());
        let diagnostics = create_table("test.sql", &mut context, &second);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::DuplicateRelation);
        assert!(diagnostics[0].msg.contains("defined more than once"));
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/lang_createtable.html")
        );
    }

    #[test]
    fn accepts_duplicate_create_table_relations_with_if_not_exists() {
        let first = create_table_node(vec![typed_id_column()], true);
        let second = CreateTable::new(
            false,
            true,
            SchemaTableContainer::Table("users".into()),
            vec![typed_id_column()],
            vec![],
            true,
            false,
        );
        let mut context = AnalysisContext::default();

        assert!(create_table("test.sql", &mut context, &first).is_empty());
        let diagnostics = create_table("test.sql", &mut context, &second);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_duplicate_schema_qualified_relations_case_insensitively() {
        let first = CreateTable::new(
            false,
            false,
            SchemaTableContainer::SchemaAndTable {
                schema: "Main".into(),
                table: "Users".into(),
            },
            vec![typed_id_column()],
            vec![],
            true,
            false,
        );
        let second = CreateTable::new(
            false,
            false,
            SchemaTableContainer::SchemaAndTable {
                schema: "main".into(),
                table: "users".into(),
            },
            vec![typed_id_column()],
            vec![],
            true,
            false,
        );
        let mut context = AnalysisContext::default();

        assert!(create_table("test.sql", &mut context, &first).is_empty());
        let diagnostics = create_table("test.sql", &mut context, &second);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::DuplicateRelation);
        assert!(diagnostics[0].msg.contains("main.users"));
    }

    #[test]
    fn accepts_known_create_virtual_table_modules() {
        for module in ["fts5", "rtree", "dbstat", "zipfile"] {
            let table = create_virtual_table_node(module);
            let (diagnostics, context) = create_virtual_table_diagnostics(&table);

            assert!(
                diagnostics.is_empty(),
                "expected {module} to be accepted, got {diagnostics:?}"
            );
            assert_eq!(
                context
                    .relation(&SchemaTableContainer::Table("items".into()))
                    .unwrap()
                    .kind,
                RelationKind::VirtualTable
            );
        }
    }

    #[test]
    fn accepts_unknown_create_virtual_table_modules() {
        let table = create_virtual_table_node("application_defined_module");
        let (diagnostics, context) = create_virtual_table_diagnostics(&table);

        assert!(diagnostics.is_empty());
        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("items".into()))
                .unwrap()
                .kind,
            RelationKind::VirtualTable
        );
    }

    #[test]
    fn reports_table_valued_function_modules_in_create_virtual_table() {
        let table = create_virtual_table_node("json_each");
        let (diagnostics, _) = create_virtual_table_diagnostics(&table);

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
    fn reports_duplicate_create_virtual_table_relations() {
        let first = create_virtual_table_node("fts5");
        let second = create_virtual_table_node("fts5");
        let mut context = AnalysisContext::default();

        assert!(create_virtual_table("test.sql", &mut context, &first).is_empty());
        let diagnostics = create_virtual_table("test.sql", &mut context, &second);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::DuplicateRelation);
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/lang_createvtab.html")
        );
    }

    #[test]
    fn includes_column_diagnostics_for_create_table_columns() {
        let table = create_table_node(vec![ColumnDef::new("name".into(), None, vec![])], true);

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let (diagnostics, _) = create_table_diagnostics(&table);

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

        let diagnostics = alter_diagnostics(&alter_node);

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

        let diagnostics = alter_diagnostics(&alter_node);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn registers_create_table_as_relations() {
        let table = CreateTableAs::new(
            false,
            false,
            SchemaTableContainer::Table("snapshot".into()),
            Box::new(empty_select()),
        );
        let mut context = AnalysisContext::default();

        let diagnostics = super::create_table_as("test.sql", &mut context, &table);

        assert!(diagnostics.is_empty());
        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("snapshot".into()))
                .unwrap()
                .kind,
            RelationKind::Table
        );
    }

    #[test]
    fn reports_duplicate_create_table_as_relations() {
        let first = create_table_node(vec![typed_id_column()], true);
        let second = CreateTableAs::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            Box::new(empty_select()),
        );
        let mut context = AnalysisContext::default();

        assert!(create_table("test.sql", &mut context, &first).is_empty());
        let diagnostics = super::create_table_as("test.sql", &mut context, &second);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::DuplicateRelation);
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/lang_createtable.html")
        );
    }

    #[test]
    fn registers_create_view_relations() {
        let view = CreateView::new(
            false,
            false,
            SchemaTableContainer::Table("active_users".into()),
            vec![],
            Box::new(empty_select()),
        );
        let mut context = AnalysisContext::default();

        let diagnostics = super::create_view("test.sql", &mut context, &view);

        assert!(diagnostics.is_empty());
        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("active_users".into()))
                .unwrap()
                .kind,
            RelationKind::View
        );
    }

    #[test]
    fn reports_duplicate_create_view_relations() {
        let first = create_table_node(vec![typed_id_column()], true);
        let second = CreateView::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![],
            Box::new(empty_select()),
        );
        let mut context = AnalysisContext::default();

        assert!(create_table("test.sql", &mut context, &first).is_empty());
        let diagnostics = super::create_view("test.sql", &mut context, &second);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::DuplicateRelation);
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/lang_createview.html")
        );
    }
}
