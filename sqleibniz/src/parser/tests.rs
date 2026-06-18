#[allow(unused_macros)]
macro_rules! test_group_pass_assert {
    ($group_name:ident,$($ident:ident:$input:literal=$expected:expr),*) => {
    mod $group_name {
        #[allow(unused_imports)]
        use super::*;
        #[allow(unused_imports)]
        use crate::{lexer, parser::Parser, parser::nodes::*, types::*, types::storage::*};

        $(
            #[test]
            fn $ident() {
                let input = $input.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&input, "parser_test_pass");
                let toks = l.run();
                assert_eq!(l.errors.len(), 0);

                let mut parser = Parser::new(toks, "parser_test_pass");
                let ast = parser.parse();
                assert_eq!(parser.errors.len(), 0);

                let serialized_ast = serde_json::to_string(
                    &ast.into_iter()
                        .map(|n| n.as_serializable())
                        .collect::<Vec<_>>(),
                ).unwrap();
                let serialized_expected = serde_json::to_string(
                    &$expected.into_iter()
                        .map(|n| n.as_serializable())
                        .collect::<Vec<_>>(),
                    )
                .unwrap();
                pretty_assertions::assert_eq!(serialized_expected, serialized_ast);
            }
        )*
        }
    };
}

#[cfg(test)]
mod should_pass {
    use crate::{
        parser::nodes::{
            Alter, ColumnConstraint, ColumnDef, Delete, Expr, Insert, InsertSource, LimitOffset,
            OrderingTerm, QualifiedTableIndex, QualifiedTableName, ResultColumn,
            SchemaTableContainer, Select, Update, UpdateAssignment,
        },
        types::{Keyword, Token, Type, storage::SqliteStorageClass},
    };

    fn alter_check(expr: Expr) -> Vec<Alter> {
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable {
                schema: "schema".into(),
                table: "table_name".into(),
            },
            None,
            None,
            None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Check(expr)],
            )),
            None,
        )]
    }

    fn lit(ttype: Type) -> Expr {
        Expr::new(
            Some(Token::new(ttype)),
            None,
            None,
            None,
            None,
            None,
            None,
            vec![],
        )
    }

    fn num(value: f64) -> Expr {
        lit(Type::Number(value))
    }

    fn string(value: &str) -> Expr {
        lit(Type::String(value.into()))
    }

    fn col(name: &str) -> Expr {
        Expr::new(
            None,
            None,
            None,
            None,
            Some(name.into()),
            None,
            None,
            vec![],
        )
    }

    fn op(operator: &str, arguments: Vec<Expr>) -> Expr {
        Expr::new(
            None,
            None,
            None,
            None,
            None,
            None,
            Some(operator.into()),
            arguments,
        )
    }

    fn qtable(name: SchemaTableContainer) -> QualifiedTableName {
        QualifiedTableName {
            name,
            alias: None,
            index: None,
        }
    }

    fn delete(
        target: QualifiedTableName,
        where_expr: Option<Expr>,
        returning: Vec<ResultColumn>,
        order_by: Vec<OrderingTerm>,
        limit: Option<LimitOffset>,
    ) -> Vec<Delete> {
        vec![Delete::new(target, where_expr, returning, order_by, limit)]
    }

    fn insert(
        conflict: Option<Keyword>,
        target: SchemaTableContainer,
        columns: Vec<String>,
        source: InsertSource,
        returning: Vec<ResultColumn>,
    ) -> Vec<Insert> {
        vec![Insert::new(conflict, target, columns, source, returning)]
    }

    fn update(
        conflict: Option<Keyword>,
        target: QualifiedTableName,
        assignments: Vec<UpdateAssignment>,
        where_expr: Option<Expr>,
        returning: Vec<ResultColumn>,
        order_by: Vec<OrderingTerm>,
        limit: Option<LimitOffset>,
    ) -> Vec<Update> {
        vec![Update::new(
            conflict,
            target,
            assignments,
            where_expr,
            returning,
            order_by,
            limit,
        )]
    }

    fn assignment(columns: Vec<&str>, expr: Expr) -> UpdateAssignment {
        UpdateAssignment {
            columns: columns.into_iter().map(String::from).collect(),
            expr,
        }
    }

    fn select(
        columns: Vec<ResultColumn>,
        from: Vec<SchemaTableContainer>,
        where_expr: Option<Expr>,
        order_by: Vec<OrderingTerm>,
        limit: Option<LimitOffset>,
    ) -> Vec<Select> {
        vec![Select::new(columns, from, where_expr, order_by, limit)]
    }

    test_group_pass_assert! {
        sqleibniz_instructions,
        expect: r"
    -- @sqleibniz::expect
    VACUUM 25;
    -- the above is skipped
    EXPLAIN VACUUM;
        "=vec![Explain::new(Box::new(Vacuum::new(None, None)))],

        expect_with_semicolons_in_comment: r"
    -- @sqleibniz::expect lets skip this error;;;;;;;;
    VACUUM 25;
    EXPLAIN VACUUM;
        "=vec![Explain::new(Box::new(Vacuum::new(None, None)))]
    }

    test_group_pass_assert! {
        sql_stmt_prefix,
        explain: r#"EXPLAIN VACUUM;"#=vec![Explain::new(Box::new(Vacuum::new(None, None)))],
        explain_query_plan: r#"EXPLAIN QUERY PLAN VACUUM;"#=vec![Explain::new(Box::new(Vacuum::new(None, None)))]
    }

    test_group_pass_assert! {
        vacuum,
        vacuum_first_path: r#"VACUUM;"#=vec![Vacuum::new(None, None)],
        vacuum_second_path: r#"VACUUM schema_name;"#=vec![
            Vacuum::new(
                Some(Token::new(Type::Ident("schema_name".into()))),
                None,
            )
        ],
        vacuum_third_path: r#"VACUUM INTO 'filename';"#=vec![
            Vacuum::new(
                None,
                Some(Token::new(Type::String("filename".into()))),
            )
        ],
        vacuum_full_path: r#"VACUUM schema_name INTO 'filename';"#=vec![
            Vacuum::new(
                Some(Token::new(Type::Ident("schema_name".into()))),
                Some(Token::new(Type::String("filename".into()))),
            )
        ]
    }

    test_group_pass_assert! {
        begin_stmt,
        begin: r#"BEGIN;"#=vec![Begin::new(None)],
        begin_transaction: r#"BEGIN TRANSACTION;"#=vec![Begin::new(None)],
        begin_deferred: r#"BEGIN DEFERRED;"#=vec![Begin::new(Some(Keyword::DEFERRED))],
        begin_immediate: r#"BEGIN IMMEDIATE;"#=vec![Begin::new(Some(Keyword::IMMEDIATE))],
        begin_exclusive: r#"BEGIN EXCLUSIVE;"#=vec![Begin::new(Some(Keyword::EXCLUSIVE))],

        begin_deferred_transaction: r"BEGIN DEFERRED TRANSACTION;"=vec![Begin::new(Some(Keyword::DEFERRED))],
        begin_immediate_transaction: r"BEGIN IMMEDIATE TRANSACTION;"=vec![Begin::new(Some(Keyword::IMMEDIATE))],
        begin_exclusive_transaction: r"BEGIN EXCLUSIVE TRANSACTION;"=vec![Begin::new(Some(Keyword::EXCLUSIVE))]
    }

    test_group_pass_assert! {
        commit_stmt,
        commit:            r"COMMIT;"=vec![Commit::new()],
        end:               r"END;"=vec![Commit::new()],
        commit_transaction:r"COMMIT TRANSACTION;"=vec![Commit::new()],
        end_transaction:   r"END TRANSACTION;"=vec![Commit::new()]
    }

    test_group_pass_assert! {
        rollback_stmt,

        rollback:r"ROLLBACK;"=vec![Rollback::new(None)],
        rollback_to_save_point:r"ROLLBACK TO save_point;"=vec![Rollback::new(Some("save_point".into()))],
        rollback_to_savepoint_save_point:r"ROLLBACK TO SAVEPOINT save_point;"=vec![Rollback::new(Some("save_point".into()))],
        rollback_transaction:r"ROLLBACK TRANSACTION;"=vec![Rollback::new(None)],
        rollback_transaction_to_save_point:r"ROLLBACK TRANSACTION TO save_point;"=vec![Rollback::new(Some("save_point".into()))],
        rollback_transaction_to_savepoint_save_point:r"ROLLBACK TRANSACTION TO SAVEPOINT save_point;"=vec![Rollback::new(Some("save_point".into()))]
    }

    test_group_pass_assert! {
        detach_stmt,

        detach_schema_name:r"DETACH schema_name;"=vec![Detach::new("schema_name".into())],
        detach_database_schema_name:r"DETACH DATABASE schema_name;"=vec![Detach::new("schema_name".into())]
    }

    test_group_pass_assert! {
        analyze_stmt,

        analyze:r"ANALYZE;"=vec![Analyze::new(None)],
        analyze_schema_name:r"ANALYZE schema_name;"=vec![
            Analyze::new(
                Some(SchemaTableContainer::Table("schema_name".into())),
            ),
        ],
        analyze_index_or_table_name:r"ANALYZE index_or_table_name;"=vec![
            Analyze::new(
                Some(SchemaTableContainer::Table("index_or_table_name".into()))
            )
        ],
        analyze_schema_name_with_subtable:r"ANALYZE schema_name.index_or_table_name;"=vec![
            Analyze::new(
                Some(SchemaTableContainer::SchemaAndTable {
                    schema: "schema_name".into(),
                    table: "index_or_table_name".into(),
                })
            )
        ]
    }

    test_group_pass_assert! {
        delete_stmt,

        delete_from_table:
        r"DELETE FROM users;"=delete(
            qtable(SchemaTableContainer::Table("users".into())),
            None,
            vec![],
            vec![],
            None,
        ),

        delete_from_schema_table:
        r"DELETE FROM main.users;"=delete(
            qtable(SchemaTableContainer::SchemaAndTable { schema: "main".into(), table: "users".into() }),
            None,
            vec![],
            vec![],
            None,
        ),

        delete_where:
        r"DELETE FROM users WHERE id = 1;"=delete(
            qtable(SchemaTableContainer::Table("users".into())),
            Some(op("=", vec![col("id"), num(1.0)])),
            vec![],
            vec![],
            None,
        ),

        delete_alias_indexed:
        r"DELETE FROM users AS u INDEXED BY idx_users_id WHERE u.id = 1;"=delete(
            QualifiedTableName {
                name: SchemaTableContainer::Table("users".into()),
                alias: Some("u".into()),
                index: Some(QualifiedTableIndex::IndexedBy("idx_users_id".into())),
            },
            Some(op("=", vec![
                Expr::new(None, None, None, Some("u".into()), Some("id".into()), None, None, vec![]),
                num(1.0),
            ])),
            vec![],
            vec![],
            None,
        ),

        delete_not_indexed:
        r"DELETE FROM users u NOT INDEXED;"=delete(
            QualifiedTableName {
                name: SchemaTableContainer::Table("users".into()),
                alias: Some("u".into()),
                index: Some(QualifiedTableIndex::NotIndexed),
            },
            None,
            vec![],
            vec![],
            None,
        ),

        delete_returning:
        r"DELETE FROM users WHERE id = 1 RETURNING *, users.*, id AS deleted_id, name;"=delete(
            qtable(SchemaTableContainer::Table("users".into())),
            Some(op("=", vec![col("id"), num(1.0)])),
            vec![
                ResultColumn::Star,
                ResultColumn::TableStar("users".into()),
                ResultColumn::Expr { expr: col("id"), alias: Some("deleted_id".into()) },
                ResultColumn::Expr { expr: col("name"), alias: None },
            ],
            vec![],
            None,
        ),

        delete_order_limit:
        r"DELETE FROM logs WHERE created_at < 10 ORDER BY created_at DESC NULLS LAST, id ASC LIMIT 5 OFFSET 2;"=delete(
            qtable(SchemaTableContainer::Table("logs".into())),
            Some(op("<", vec![col("created_at"), num(10.0)])),
            vec![],
            vec![
                OrderingTerm { expr: col("created_at"), order: Some(Keyword::DESC), nulls: Some(Keyword::LAST) },
                OrderingTerm { expr: col("id"), order: Some(Keyword::ASC), nulls: None },
            ],
            Some(LimitOffset { limit: num(5.0), offset: Some(num(2.0)) }),
        ),

        delete_limit_comma_offset:
        r"DELETE FROM logs LIMIT 5, 2;"=delete(
            qtable(SchemaTableContainer::Table("logs".into())),
            None,
            vec![],
            vec![],
            Some(LimitOffset { limit: num(5.0), offset: Some(num(2.0)) }),
        )
    }

    test_group_pass_assert! {
        insert_stmt,

        insert_default_values:
        r"INSERT INTO users DEFAULT VALUES;"=insert(
            None,
            SchemaTableContainer::Table("users".into()),
            vec![],
            InsertSource::DefaultValues,
            vec![],
        ),

        insert_values:
        r"INSERT INTO users VALUES (1, 'Ada');"=insert(
            None,
            SchemaTableContainer::Table("users".into()),
            vec![],
            InsertSource::Values(vec![vec![num(1.0), string("Ada")]]),
            vec![],
        ),

        insert_column_list_values:
        r"INSERT INTO main.users (id, name) VALUES (1, 'Ada');"=insert(
            None,
            SchemaTableContainer::SchemaAndTable { schema: "main".into(), table: "users".into() },
            vec!["id".into(), "name".into()],
            InsertSource::Values(vec![vec![num(1.0), string("Ada")]]),
            vec![],
        ),

        insert_multi_row_values:
        r"INSERT INTO users (id, name) VALUES (1, 'Ada'), (2, 'Grace');"=insert(
            None,
            SchemaTableContainer::Table("users".into()),
            vec!["id".into(), "name".into()],
            InsertSource::Values(vec![
                vec![num(1.0), string("Ada")],
                vec![num(2.0), string("Grace")],
            ]),
            vec![],
        ),

        insert_or_ignore_returning:
        r"INSERT OR IGNORE INTO users (id) VALUES (1) RETURNING *, id AS inserted_id;"=insert(
            Some(Keyword::IGNORE),
            SchemaTableContainer::Table("users".into()),
            vec!["id".into()],
            InsertSource::Values(vec![vec![num(1.0)]]),
            vec![
                ResultColumn::Star,
                ResultColumn::Expr { expr: col("id"), alias: Some("inserted_id".into()) },
            ],
        )
    }

    test_group_pass_assert! {
        update_stmt,

        update_single_assignment:
        r"UPDATE users SET name = 'Ada';"=update(
            None,
            qtable(SchemaTableContainer::Table("users".into())),
            vec![assignment(vec!["name"], string("Ada"))],
            None,
            vec![],
            vec![],
            None,
        ),

        update_or_fail_schema_table_where:
        r"UPDATE OR FAIL main.users SET name = 'Ada' WHERE id = 1;"=update(
            Some(Keyword::FAIL),
            qtable(SchemaTableContainer::SchemaAndTable { schema: "main".into(), table: "users".into() }),
            vec![assignment(vec!["name"], string("Ada"))],
            Some(op("=", vec![col("id"), num(1.0)])),
            vec![],
            vec![],
            None,
        ),

        update_alias_not_indexed:
        r"UPDATE users AS u NOT INDEXED SET name = 'Ada' WHERE u.id = 1;"=update(
            None,
            QualifiedTableName {
                name: SchemaTableContainer::Table("users".into()),
                alias: Some("u".into()),
                index: Some(QualifiedTableIndex::NotIndexed),
            },
            vec![assignment(vec!["name"], string("Ada"))],
            Some(op("=", vec![
                Expr::new(None, None, None, Some("u".into()), Some("id".into()), None, None, vec![]),
                num(1.0),
            ])),
            vec![],
            vec![],
            None,
        ),

        update_indexed_multi_assignment:
        r"UPDATE users INDEXED BY idx_users_id SET name = 'Ada', active = true;"=update(
            None,
            QualifiedTableName {
                name: SchemaTableContainer::Table("users".into()),
                alias: None,
                index: Some(QualifiedTableIndex::IndexedBy("idx_users_id".into())),
            },
            vec![
                assignment(vec!["name"], string("Ada")),
                assignment(vec!["active"], lit(Type::Boolean(true))),
            ],
            None,
            vec![],
            vec![],
            None,
        ),

        update_column_list_assignment:
        r"UPDATE users SET (name, email) = user_defaults(id);"=update(
            None,
            qtable(SchemaTableContainer::Table("users".into())),
            vec![assignment(
                vec!["name", "email"],
                Expr::new(None, None, None, None, None, Some("user_defaults".into()), None, vec![col("id")]),
            )],
            None,
            vec![],
            vec![],
            None,
        ),

        update_returning_order_limit:
        r"UPDATE users SET active = false WHERE last_seen < 10 RETURNING *, id AS updated_id ORDER BY last_seen DESC LIMIT 5 OFFSET 2;"=update(
            None,
            qtable(SchemaTableContainer::Table("users".into())),
            vec![assignment(vec!["active"], lit(Type::Boolean(false)))],
            Some(op("<", vec![col("last_seen"), num(10.0)])),
            vec![
                ResultColumn::Star,
                ResultColumn::Expr { expr: col("id"), alias: Some("updated_id".into()) },
            ],
            vec![OrderingTerm { expr: col("last_seen"), order: Some(Keyword::DESC), nulls: None }],
            Some(LimitOffset { limit: num(5.0), offset: Some(num(2.0)) }),
        )
    }

    test_group_pass_assert! {
        select_stmt,

        select_literal:
        r"SELECT 1;"=select(
            vec![ResultColumn::Expr { expr: num(1.0), alias: None }],
            vec![],
            None,
            vec![],
            None,
        ),

        select_columns_from_table_where:
        r"SELECT id, name FROM users WHERE active = true;"=select(
            vec![
                ResultColumn::Expr { expr: col("id"), alias: None },
                ResultColumn::Expr { expr: col("name"), alias: None },
            ],
            vec![SchemaTableContainer::Table("users".into())],
            Some(op("=", vec![col("active"), lit(Type::Boolean(true))])),
            vec![],
            None,
        ),

        select_star_and_table_star:
        r"SELECT *, users.* FROM main.users;"=select(
            vec![ResultColumn::Star, ResultColumn::TableStar("users".into())],
            vec![SchemaTableContainer::SchemaAndTable { schema: "main".into(), table: "users".into() }],
            None,
            vec![],
            None,
        ),

        select_expr_alias_order_limit:
        r"SELECT id AS user_id, lower(name) AS normalized FROM users ORDER BY id DESC LIMIT 10 OFFSET 5;"=select(
            vec![
                ResultColumn::Expr { expr: col("id"), alias: Some("user_id".into()) },
                ResultColumn::Expr {
                    expr: Expr::new(None, None, None, None, None, Some("lower".into()), None, vec![col("name")]),
                    alias: Some("normalized".into()),
                },
            ],
            vec![SchemaTableContainer::Table("users".into())],
            None,
            vec![OrderingTerm { expr: col("id"), order: Some(Keyword::DESC), nulls: None }],
            Some(LimitOffset { limit: num(10.0), offset: Some(num(5.0)) }),
        ),

        select_multiple_from_tables:
        r"SELECT id FROM users, archived_users;"=select(
            vec![ResultColumn::Expr { expr: col("id"), alias: None }],
            vec![
                SchemaTableContainer::Table("users".into()),
                SchemaTableContainer::Table("archived_users".into()),
            ],
            None,
            vec![],
            None,
        )
    }

    test_group_pass_assert! {
        drop_stmt,

        drop_index_index_name:r"DROP INDEX index_name;"=vec![Drop::new(false, Keyword::INDEX, SchemaTableContainer::Table("index_name".into()))],
        drop_index_if_exists_schema_name_index_name:r"DROP INDEX IF EXISTS schema_name.index_name;"=vec![
            Drop::new(true, Keyword::INDEX, SchemaTableContainer::SchemaAndTable{ schema: "schema_name".into(), table: "index_name".into(), })
        ],
        drop_table_table_name:r"DROP TABLE table_name;"=vec![Drop::new(false, Keyword::TABLE, SchemaTableContainer::Table("table_name".into()))],
        drop_table_if_exists_schema_name_table_name:r"DROP TABLE IF EXISTS schema_name.table_name;"=vec![
            Drop::new(true, Keyword::TABLE, SchemaTableContainer::SchemaAndTable{ schema: "schema_name".into(), table: "table_name".into(), })
        ],
        drop_trigger_trigger_name:r"DROP TRIGGER trigger_name;"=vec![Drop::new(false, Keyword::TRIGGER, SchemaTableContainer::Table("trigger_name".into()))],
        drop_trigger_if_exists_schema_name_trigger_name:r"DROP TRIGGER IF EXISTS schema_name.trigger_name;"=vec![
            Drop::new(true, Keyword::TRIGGER, SchemaTableContainer::SchemaAndTable{ schema: "schema_name".into(), table: "trigger_name".into(), })
        ],
        drop_view_view_name:r"DROP VIEW view_name;"=vec![
            Drop::new(false, Keyword::VIEW, SchemaTableContainer::Table("view_name".into()))
        ],
        drop_view_if_exists_schema_name_view_name:r"DROP VIEW IF EXISTS schema_name.view_name;"=vec![
            Drop::new(true, Keyword::VIEW, SchemaTableContainer::SchemaAndTable{ schema: "schema_name".into(), table: "view_name".into(), })
        ]
    }

    test_group_pass_assert! {
        savepoint_stmt,

        savepoint_savepoint_name:r"SAVEPOINT savepoint_name;"=vec![Savepoint::new("savepoint_name".into())]
    }

    test_group_pass_assert! {
        release_stmt,

        release_savepoint_savepoint_name:r"RELEASE SAVEPOINT savepoint_name;"=vec![Release::new("savepoint_name".into())],
        release_savepoint_name:r"RELEASE savepoint_name;"=vec![Release::new("savepoint_name".into())]
    }

    test_group_pass_assert! {
        attach_stmt,

        attach:r"ATTACH 'database.db' AS db;"=vec![
            Attach::new(
                "db".into(),
                Expr::new(
                    Some(Token::new(Type::String("database.db".into()))),
                    None,
                    None,
                    None,
                    None,
                    None, None, vec![],
                )
            ),
        ],
        attach_database:r"ATTACH DATABASE 'database.db' AS db;"=vec![
            Attach::new(
                "db".into(),
                Expr::new(
                    Some(Token::new(Type::String("database.db".into()))),
                    None,
                    None,
                    None,
                    None,
                    None, None, vec![],
                )
            ),
        ]
    }

    test_group_pass_assert! {
        reindex_stmt,

        reindex:r"REINDEX;"=vec![Reindex::new(None)],
        reindex_collation_name:r"REINDEX collation_name;"=vec![Reindex::new(Some(SchemaTableContainer::Table("collation_name".into())))],
        reindex_schema_name_table_name:r"REINDEX schema_name.table_name;"=vec![Reindex::new(Some(SchemaTableContainer::SchemaAndTable { schema: "schema_name".into(), table: "table_name".into() }))]
    }

    test_group_pass_assert! {
        create_table_stmt,

        create_table_single_column:
        r"CREATE TABLE users (id INTEGER);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("id".into(), Some(SqliteStorageClass::Integer), vec![])],
            vec![],
            false,
            false,
        )],

        create_table_column_without_type:
        r"CREATE TABLE users (name);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("name".into(), None, vec![])],
            vec![],
            false,
            false,
        )],

        create_temp_table_if_not_exists:
        r"CREATE TEMP TABLE IF NOT EXISTS main.users (id INTEGER PRIMARY KEY, name TEXT);"=
        vec![CreateTable::new(
            true,
            true,
            SchemaTableContainer::SchemaAndTable {
                schema: "main".into(),
                table: "users".into(),
            },
            vec![
                ColumnDef::new(
                    "id".into(),
                    Some(SqliteStorageClass::Integer),
                    vec![ColumnConstraint::PrimaryKey {
                        asc_desc: None,
                        on_conflict: None,
                        autoincrement: false,
                    }],
                ),
                ColumnDef::new("name".into(), Some(SqliteStorageClass::Text), vec![]),
            ],
            vec![],
            false,
            false,
        )],

        create_table_foreign_key_column:
        r"CREATE TABLE users (team_id INTEGER REFERENCES teams);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "team_id".into(),
                Some(SqliteStorageClass::Integer),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "teams".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })],
            )],
            vec![],
            false,
            false,
        )],

        create_table_strict:
        r"CREATE TABLE users (id INTEGER, metadata ANY) STRICT;"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![
                ColumnDef::new("id".into(), Some(SqliteStorageClass::Integer), vec![]),
                ColumnDef::new("metadata".into(), Some(SqliteStorageClass::Any), vec![]),
            ],
            vec![],
            true,
            false,
        )],

        create_table_without_rowid:
        r"CREATE TABLE users (id INTEGER PRIMARY KEY) WITHOUT ROWID;"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            vec![],
            false,
            true,
        )],

        create_table_strict_without_rowid:
        r"CREATE TABLE users (id INTEGER PRIMARY KEY) WITHOUT ROWID, STRICT;"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            vec![],
            true,
            true,
        )],

        create_table_strict_then_without_rowid:
        r"CREATE TABLE users (id INTEGER PRIMARY KEY) STRICT, WITHOUT ROWID;"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            vec![],
            true,
            true,
        )],

        create_table_duplicate_strict_options:
        r"CREATE TABLE users (id INTEGER) STRICT, STRICT;"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("id".into(), Some(SqliteStorageClass::Integer), vec![])],
            vec![],
            true,
            false,
        )],

        create_table_duplicate_without_rowid_options:
        r"CREATE TABLE users (id INTEGER PRIMARY KEY) WITHOUT ROWID, WITHOUT ROWID;"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new(
                "id".into(),
                Some(SqliteStorageClass::Integer),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )],
            vec![],
            false,
            true,
        )],

        create_table_primary_key_constraint:
        r"CREATE TABLE users (id INTEGER, team_id INTEGER, PRIMARY KEY (id, team_id) ON CONFLICT REPLACE);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![
                ColumnDef::new("id".into(), Some(SqliteStorageClass::Integer), vec![]),
                ColumnDef::new("team_id".into(), Some(SqliteStorageClass::Integer), vec![]),
            ],
            vec![TableConstraint::PrimaryKey {
                columns: vec![
                    IndexedColumn { name: "id".into(), collation: None, order: None },
                    IndexedColumn { name: "team_id".into(), collation: None, order: None },
                ],
                on_conflict: Some(Keyword::REPLACE),
            }],
            false,
            false,
        )],

        create_table_named_primary_key_constraint_with_order:
        r"CREATE TABLE users (id INTEGER, CONSTRAINT pk_users PRIMARY KEY (id DESC));"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("id".into(), Some(SqliteStorageClass::Integer), vec![])],
            vec![TableConstraint::PrimaryKey {
                columns: vec![IndexedColumn {
                    name: "id".into(),
                    collation: None,
                    order: Some(Keyword::DESC),
                }],
                on_conflict: None,
            }],
            false,
            false,
        )],

        create_table_unique_constraint:
        r"CREATE TABLE users (email TEXT, CONSTRAINT unique_email UNIQUE (email COLLATE nocase) ON CONFLICT IGNORE);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("email".into(), Some(SqliteStorageClass::Text), vec![])],
            vec![TableConstraint::Unique {
                columns: vec![IndexedColumn {
                    name: "email".into(),
                    collation: Some("nocase".into()),
                    order: None,
                }],
                on_conflict: Some(Keyword::IGNORE),
            }],
            false,
            false,
        )],

        create_table_check_constraint:
        r"CREATE TABLE users (name TEXT, CHECK ('literal'));"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("name".into(), Some(SqliteStorageClass::Text), vec![])],
            vec![TableConstraint::Check(Expr::new(
                Some(Token::new(Type::String("literal".into()))),
                None, None, None, None, None, None, vec![],
            ))],
            false,
            false,
        )],

        create_table_named_check_constraint:
        r"CREATE TABLE users (name TEXT, CONSTRAINT check_name CHECK ('literal'));"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("name".into(), Some(SqliteStorageClass::Text), vec![])],
            vec![TableConstraint::Check(Expr::new(
                Some(Token::new(Type::String("literal".into()))),
                None, None, None, None, None, None, vec![],
            ))],
            false,
            false,
        )],

        create_table_foreign_key_constraint:
        r"CREATE TABLE users (team_id INTEGER, FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("team_id".into(), Some(SqliteStorageClass::Integer), vec![])],
            vec![TableConstraint::ForeignKey {
                columns: vec!["team_id".into()],
                foreign_key_clause: ForeignKeyClause {
                    foreign_table: "teams".into(),
                    references_columns: vec!["id".into()],
                    on_delete: Some(ForeignKeyAction::Cascade),
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                },
            }],
            false,
            false,
        )],

        create_table_named_foreign_key_constraint:
        r"CREATE TABLE users (team_id INTEGER, CONSTRAINT fk_team FOREIGN KEY (team_id) REFERENCES teams);"=
        vec![CreateTable::new(
            false,
            false,
            SchemaTableContainer::Table("users".into()),
            vec![ColumnDef::new("team_id".into(), Some(SqliteStorageClass::Integer), vec![])],
            vec![TableConstraint::ForeignKey {
                columns: vec!["team_id".into()],
                foreign_key_clause: ForeignKeyClause {
                    foreign_table: "teams".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                },
            }],
            false,
            false,
        )]
    }

    test_group_pass_assert! {
        create_index_stmt,

        create_index_single_column:
        r"CREATE INDEX idx_users_id ON users (id);"=
        vec![CreateIndex::new(
            false,
            false,
            SchemaTableContainer::Table("idx_users_id".into()),
            "users".into(),
            vec![IndexedColumn {
                name: "id".into(),
                collation: None,
                order: None,
            }],
        )],

        create_unique_index_if_not_exists:
        r"CREATE UNIQUE INDEX IF NOT EXISTS main.idx_users_email ON users (email);"=
        vec![CreateIndex::new(
            true,
            true,
            SchemaTableContainer::SchemaAndTable {
                schema: "main".into(),
                table: "idx_users_email".into(),
            },
            "users".into(),
            vec![IndexedColumn {
                name: "email".into(),
                collation: None,
                order: None,
            }],
        )],

        create_index_with_indexed_column_modifiers:
        r"CREATE INDEX idx_users_name ON users (name COLLATE nocase DESC, id ASC);"=
        vec![CreateIndex::new(
            false,
            false,
            SchemaTableContainer::Table("idx_users_name".into()),
            "users".into(),
            vec![
                IndexedColumn {
                    name: "name".into(),
                    collation: Some("nocase".into()),
                    order: Some(Keyword::DESC),
                },
                IndexedColumn {
                    name: "id".into(),
                    collation: None,
                    order: Some(Keyword::ASC),
                },
            ],
        )]
    }

    test_group_pass_assert! {
        create_trigger_stmt,

        create_trigger_after_insert:
        r"CREATE TRIGGER user_ai AFTER INSERT ON users BEGIN SELECT 1; END;"=
        vec![CreateTrigger::new(
            false,
            false,
            SchemaTableContainer::Table("user_ai".into()),
            Some(TriggerTiming::After),
            TriggerEvent::Insert,
            "users".into(),
            false,
            false,
            vec![TriggerBodyStmt::Select],
        )],

        create_trigger_insert_without_timing:
        r"CREATE TRIGGER user_i INSERT ON users BEGIN SELECT 1; END;"=
        vec![CreateTrigger::new(
            false,
            false,
            SchemaTableContainer::Table("user_i".into()),
            None,
            TriggerEvent::Insert,
            "users".into(),
            false,
            false,
            vec![TriggerBodyStmt::Select],
        )],

        create_trigger_update_without_columns:
        r"CREATE TRIGGER user_au AFTER UPDATE ON users BEGIN UPDATE users SET name = new.name; END;"=
        vec![CreateTrigger::new(
            false,
            false,
            SchemaTableContainer::Table("user_au".into()),
            Some(TriggerTiming::After),
            TriggerEvent::Update { columns: vec![] },
            "users".into(),
            false,
            false,
            vec![TriggerBodyStmt::Update],
        )],

        create_temp_trigger_if_not_exists_instead_of_update:
        r"CREATE TEMP TRIGGER IF NOT EXISTS temp.user_update INSTEAD OF UPDATE OF name, email ON users FOR EACH ROW WHEN old_name BEGIN UPDATE users SET name = new.name; END;"=
        vec![CreateTrigger::new(
            true,
            true,
            SchemaTableContainer::SchemaAndTable {
                schema: "temp".into(),
                table: "user_update".into(),
            },
            Some(TriggerTiming::InsteadOf),
            TriggerEvent::Update {
                columns: vec!["name".into(), "email".into()],
            },
            "users".into(),
            true,
            true,
            vec![TriggerBodyStmt::Update],
        )],

        create_trigger_multiple_body_statements:
        r"CREATE TRIGGER user_ad BEFORE DELETE ON users BEGIN INSERT INTO audit VALUES (old.id); DELETE FROM sessions WHERE user_id = old.id; END;"=
        vec![CreateTrigger::new(
            false,
            false,
            SchemaTableContainer::Table("user_ad".into()),
            Some(TriggerTiming::Before),
            TriggerEvent::Delete,
            "users".into(),
            false,
            false,
            vec![TriggerBodyStmt::Insert, TriggerBodyStmt::Delete],
        )]
    }

    test_group_pass_assert! {
        alter_stmt,

        alter_rename_to: r"ALTER TABLE schema.table_name RENAME TO new_table;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                Some("new_table".into()),
                None,
                None,
                None,
                None,
            ),
        ],

        alter_rename_column_to: r"ALTER TABLE schema.table_name RENAME COLUMN old_column_name TO new_column_name;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                Some("old_column_name".into()),
                Some("new_column_name".into()),
                None,
                None,
            ),
        ],
        alter_rename_column_to_without_column_keyword: r"ALTER TABLE schema.table_name RENAME old_column_name TO new_column_name;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                Some("old_column_name".into()),
                Some("new_column_name".into()),
                None,
                None,
            ),
        ],

        alter_add: r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                None,
                None,
                Some(ColumnDef::new("column_name".into(), Some(SqliteStorageClass::Text), vec![])),
                None,
            ),
        ],
        alter_add_without_column_keyword: r"ALTER TABLE schema.table_name ADD column_name TEXT;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                None,
                None,
                Some(ColumnDef::new("column_name".into(), Some(SqliteStorageClass::Text), vec![])),
                None,
            ),
        ],
        alter_add_column_without_type: r"ALTER TABLE schema.table_name ADD COLUMN column_name;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                None,
                None,
                Some(ColumnDef::new("column_name".into(), None, vec![])),
                None,
            ),
        ],

        alter_drop_column: r"ALTER TABLE schema.table_name DROP COLUMN column_name;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                None,
                None,
                None,
                Some("column_name".into()),
            ),
        ],
        alter_drop_column_without_column_keyword: r"ALTER TABLE schema.table_name DROP column_name;"=vec![
            Alter::new(
                SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
                None,
                None,
                None,
                None,
                Some("column_name".into()),
            ),
        ]
    }

    test_group_pass_assert! {
        column_constraint_primary_key,

        primary_key_no_order:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT PRIMARY KEY;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )),
            None,
        )],

        primary_key_asc:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT PRIMARY KEY ASC;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: Some(Keyword::ASC),
                    on_conflict: None,
                    autoincrement: false,
                }],
            )),
            None,
        )],

        named_primary_key:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CONSTRAINT pk PRIMARY KEY;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: None,
                    on_conflict: None,
                    autoincrement: false,
                }],
            )),
            None,
        )],

        primary_key_desc_conflict_replace_autoincrement:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT PRIMARY KEY DESC ON CONFLICT REPLACE AUTOINCREMENT;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::PrimaryKey {
                    asc_desc: Some(Keyword::DESC),
                    on_conflict: Some(Keyword::REPLACE),
                    autoincrement: true,
                }],
            )),
            None,
        )]
    }

    test_group_pass_assert! {
        column_constraint_not_null_unique,

        not_null_no_conflict:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT NOT NULL;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::NotNull { on_conflict: None }],
            )),
            None,
        )],

        unique_conflict_replace:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT UNIQUE ON CONFLICT REPLACE;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Unique {
                    on_conflict: Some(Keyword::REPLACE),
                }],
            )),
            None,
        )]
    }

    test_group_pass_assert! {
        column_constraint_misc,

        check_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK ('literal string lol');"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Check(
                    Expr::new(
                        Some(Token::new(Type::String("literal string lol".into()))),
                        None, None, None, None, None, None, vec![])
                )],
            )),
            None,
        )],

        check_column_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (column_name);"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Check(
                    Expr::new(
                        None,
                        None,
                        None,
                        None,
                        Some("column_name".into()),
                        None, None, vec![],
                    )
                )],
            )),
            None,
        )],

        check_qualified_column_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (app.users.email);"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Check(
                    Expr::new(
                        None,
                        None,
                        Some("app".into()),
                        Some("users".into()),
                        Some("email".into()),
                        None, None, vec![],
                    )
                )],
            )),
            None,
        )],

        check_function_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (length(trim(column_name)));"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Check(
                    Expr::new(
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some("length".into()),
                        None,
                        vec![Expr::new(
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some("trim".into()),
                            None,
                            vec![Expr::new(
                                None,
                                None,
                                None,
                                None,
                                Some("column_name".into()),
                                None, None, vec![],
                            )],
                        )],
                    )
                )],
            )),
            None,
        )],

        default_literal:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT DEFAULT 'literal';"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Default {
                    expr: None,
                    literal: Some(Literal {
                        location: crate::error::Location::new(0, 0, 0),
                        value: Token::new(Type::String("literal".into())),
                    }),
                }],
            )),
            None,
        )],

        default_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT DEFAULT ('literal');"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Default {
                    expr: Some(Expr::new(
                        Some(Token::new(Type::String("literal".into()))),
                        None, None, None, None, None, None, vec![])),
                    literal: None,
                }],
            )),
            None,
        )],

        collate:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT COLLATE collation_name;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Collate("collation_name".into())],
            )),
            None,
        )]
    }

    test_group_pass_assert! {
        expr_operators,

        binary_comparison:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (age > 0);"=
        alter_check(op(">", vec![col("age"), num(0.0)])),

        logical_precedence:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (age > 0 AND age < 130 OR admin = true);"=
        alter_check(op("OR", vec![
            op("AND", vec![
                op(">", vec![col("age"), num(0.0)]),
                op("<", vec![col("age"), num(130.0)]),
            ]),
            op("=", vec![col("admin"), lit(Type::Boolean(true))]),
        ])),

        arithmetic_precedence:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (price + tax * 2 == total);"=
        alter_check(op("==", vec![
            op("+", vec![
                col("price"),
                op("*", vec![col("tax"), num(2.0)]),
            ]),
            col("total"),
        ])),

        unary_not_low_precedence:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (NOT age > 0);"=
        alter_check(op("NOT", vec![op(">", vec![col("age"), num(0.0)])])),

        unary_numeric_high_precedence:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (-age < 0);"=
        alter_check(op("<", vec![op("-", vec![col("age")]), num(0.0)])),

        bitwise_and_shift:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (~flags & 3 << 1);"=
        alter_check(op("<<", vec![
            op("&", vec![op("~", vec![col("flags")]), num(3.0)]),
            num(1.0),
        ])),

        concat_and_json_extract:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (doc ->> 'name' || suffix);"=
        alter_check(op("||", vec![
            op("->>", vec![col("doc"), string("name")]),
            col("suffix"),
        ])),

        is_not:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (deleted_at IS NOT NULL);"=
        alter_check(op("IS NOT", vec![col("deleted_at"), lit(Type::Keyword(Keyword::NULL))])),

        distinct_from:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (a IS DISTINCT FROM b);"=
        alter_check(op("IS DISTINCT FROM", vec![col("a"), col("b")])),

        not_distinct_from:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (a IS NOT DISTINCT FROM b);"=
        alter_check(op("IS NOT DISTINCT FROM", vec![col("a"), col("b")])),

        between:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (age BETWEEN 18 AND 130);"=
        alter_check(op("BETWEEN", vec![col("age"), num(18.0), num(130.0)])),

        not_between:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (age NOT BETWEEN 18 AND 130);"=
        alter_check(op("NOT BETWEEN", vec![col("age"), num(18.0), num(130.0)])),

        in_list:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (status IN ('new', 'done'));"=
        alter_check(op("IN", vec![col("status"), string("new"), string("done")])),

        not_in_empty_list:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (status NOT IN ());"=
        alter_check(op("NOT IN", vec![col("status")])),

        like_escape:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (name NOT LIKE 'a%' ESCAPE '\\');"=
        alter_check(op("ESCAPE", vec![
            op("NOT LIKE", vec![col("name"), string("a%")]),
            string("\\\\"),
        ])),

        glob_match_regexp:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (name GLOB 'a*' AND body MATCH 'needle' AND email REGEXP '.+@.+');"=
        alter_check(op("AND", vec![
            op("AND", vec![
                op("GLOB", vec![col("name"), string("a*")]),
                op("MATCH", vec![col("body"), string("needle")]),
            ]),
            op("REGEXP", vec![col("email"), string(".+@.+")]),
        ])),

        collate_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (name COLLATE nocase = 'x');"=
        alter_check(op("=", vec![
            op("COLLATE", vec![col("name"), col("nocase")]),
            string("x"),
        ])),

        postfix_null_checks:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT CHECK (name NOTNULL OR alias NOT NULL OR deleted_at ISNULL);"=
        alter_check(op("OR", vec![
            op("OR", vec![
                op("NOTNULL", vec![col("name")]),
                op("NOT NULL", vec![col("alias")]),
            ]),
            op("ISNULL", vec![col("deleted_at")]),
        ]))
    }

    test_group_pass_assert! {
        column_constraint_generated,

        generated_stored:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT GENERATED ALWAYS AS ('literal') STORED;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Generated {
                    expr: Expr::new(
                        Some(Token::new(Type::String("literal".into()))),
                        None, None, None, None, None, None, vec![]),
                    stored_virtual: Some(Keyword::STORED),
                }],
            )),
            None,
        )],

        generated_virtual:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT GENERATED ALWAYS AS ('literal') VIRTUAL;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::Generated {
                    expr: Expr::new(
                        Some(Token::new(Type::String("literal".into()))),
                        None, None, None, None, None, None, vec![]),
                    stored_virtual: Some(Keyword::VIRTUAL),
                }],
            )),
            None,
        )],

        as_expr:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT AS ('literal');"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::As{
                    stored_virtual: None,
                    expr: Expr::new(
                        Some(Token::new(Type::String("literal".into()))),
                        None, None, None, None, None, None, vec![])
                }],
            )),
            None,
        )]
    }

    test_group_pass_assert! {
        foreign_key_clause,

        references_on_delete_set_null:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT REFERENCES foreign_table ON DELETE SET NULL;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "foreign_table".into(),
                    references_columns: vec![],
                    on_delete: Some(ForeignKeyAction::SetNull),
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })],
            )),
            None,
        )],

        references_columns_on_update_match_deferrable:
        r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT REFERENCES foreign_table (foreign_column) ON UPDATE CASCADE MATCH FULL DEFERRABLE INITIALLY DEFERRED;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new(
                "column_name".into(),
                Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "foreign_table".into(),
                    references_columns: vec!["foreign_column".into()],
                    on_delete: None,
                    on_update: Some(ForeignKeyAction::Cascade),
                    match_type: Some(ForeignKeyMatch::Full),
                    deferrable: true,
                    initially_deferred: true,
                })],
            )),
            None,
        )]
    }

    test_group_pass_assert! {
        pragma,

        query:"PRAGMA database_list;"=vec![Pragma::new(SchemaTableContainer::Table("database_list".into()), PragmaInvocation::Query)],
        assignment:"PRAGMA schema.cache_size = 5;"=vec![
            Pragma::new(
                SchemaTableContainer::SchemaAndTable{
                    schema: "schema".into(),
                    table: "cache_size".into(),
                },
                PragmaInvocation::Assign { value: Token::new(Type::Number(5.0)) }
            )],
        assign_keyword:"PRAGMA schema.locking_mode = EXCLUSIVE;"=vec![
            Pragma::new(
                SchemaTableContainer::SchemaAndTable{
                    schema: "schema".into(),
                    table: "locking_mode".into(),
                },
                PragmaInvocation::Assign { value: Token::new(Type::Keyword(Keyword::EXCLUSIVE)) }
            )],
        assign_boolean:"PRAGMA foreign_keys = true;"=vec![
            Pragma::new(
                SchemaTableContainer::Table("foreign_keys".into()),
                PragmaInvocation::Assign { value: Token::new(Type::Boolean(true)) }
            )],
        call:"PRAGMA schema.optimize(0xfffe);"=vec![
            Pragma::new(
            SchemaTableContainer::SchemaAndTable{
                schema: "schema".into(),
                table: "optimize".into(),
            },
            PragmaInvocation::Call { value: Token::new(Type::Number(0xfffe as f64)) }
            )],
        call_boolean:"PRAGMA foreign_keys(false);"=vec![
            Pragma::new(
                SchemaTableContainer::Table("foreign_keys".into()),
                PragmaInvocation::Call { value: Token::new(Type::Boolean(false)) }
            )]
    }

    test_group_pass_assert! {
        bind_parameter,

        // expr() bind parameter paths, reached through ATTACH <expr> AS <schema>
        bind_question_no_counter:r"ATTACH ? AS db;"=vec![
            Attach::new("db".into(), Expr::new(None, Some(BindParameter::new(None, None)), None, None, None, None, None, vec![]))
        ],
        bind_question_with_counter:r"ATTACH ?5 AS db;"=vec![
            Attach::new("db".into(), Expr::new(
                None,
                Some(BindParameter::new(
                    Some(Box::new(Literal::new(Token::new(Type::Number(5.0)))) as Box<dyn Node>),
                    None,
                )),
                None, None, None, None, None, vec![],
            ))
        ],
        bind_colon:r"ATTACH :name AS db;"=vec![
            Attach::new("db".into(), Expr::new(None, Some(BindParameter::new(None, Some("name".into()))), None, None, None, None, None, vec![]))
        ],
        bind_at:r"ATTACH @name AS db;"=vec![
            Attach::new("db".into(), Expr::new(None, Some(BindParameter::new(None, Some("name".into()))), None, None, None, None, None, vec![]))
        ],
        bind_dollar:r"ATTACH $name AS db;"=vec![
            Attach::new("db".into(), Expr::new(None, Some(BindParameter::new(None, Some("name".into()))), None, None, None, None, None, vec![]))
        ]
    }

    test_group_pass_assert! {
        schema_table_container_strings,

        // table_name supplied as a quoted string instead of an identifier
        table_name_string:r"DROP TABLE 'table_name';"=vec![
            Drop::new(false, Keyword::TABLE, SchemaTableContainer::Table("table_name".into()))
        ],
        schema_and_table_string:r"DROP TABLE schema_name.'table_name';"=vec![
            Drop::new(false, Keyword::TABLE, SchemaTableContainer::SchemaAndTable {
                schema: "schema_name".into(),
                table: "table_name".into(),
            })
        ]
    }

    test_group_pass_assert! {
        column_type_parameters,

        type_with_single_parameter:
        r"ALTER TABLE schema.table_name ADD COLUMN c INTEGER(10);"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Integer), vec![])),
            None,
        )],

        type_with_two_parameters:
        r"ALTER TABLE schema.table_name ADD COLUMN c REAL(10, 2);"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "table_name".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Real), vec![])),
            None,
        )]
    }

    test_group_pass_assert! {
        conflict_clause_variants,

        not_null_abort:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT NOT NULL ON CONFLICT ABORT;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::NotNull { on_conflict: Some(Keyword::ABORT) }])),
            None,
        )],

        not_null_fail:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT NOT NULL ON CONFLICT FAIL;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::NotNull { on_conflict: Some(Keyword::FAIL) }])),
            None,
        )],

        not_null_rollback:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT NOT NULL ON CONFLICT ROLLBACK;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::NotNull { on_conflict: Some(Keyword::ROLLBACK) }])),
            None,
        )]
    }

    test_group_pass_assert! {
        foreign_key_clause_variants,

        on_delete_restrict:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON DELETE RESTRICT;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: Some(ForeignKeyAction::Restrict),
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        on_delete_no_action:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON DELETE NO ACTION;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: Some(ForeignKeyAction::NoAction),
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        on_update_set_default:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON UPDATE SET DEFAULT;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: Some(ForeignKeyAction::SetDefault),
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        match_partial:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft MATCH PARTIAL;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: Some(ForeignKeyMatch::Partial),
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        match_simple:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft MATCH SIMPLE;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: Some(ForeignKeyMatch::Simple),
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        multiple_reference_columns:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft (a, b);"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec!["a".into(), "b".into()],
                    on_delete: None,
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        not_deferrable:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft NOT DEFERRABLE;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )],

        deferrable_only:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft DEFERRABLE;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: None,
                    deferrable: true,
                    initially_deferred: false,
                })])),
            None,
        )],

        deferrable_initially_immediate:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft DEFERRABLE INITIALLY IMMEDIATE;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: None,
                    on_update: None,
                    match_type: None,
                    deferrable: true,
                    initially_deferred: false,
                })])),
            None,
        )],

        on_delete_and_on_update:
        r"ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON DELETE CASCADE ON UPDATE RESTRICT;"=
        vec![Alter::new(
            SchemaTableContainer::SchemaAndTable { schema: "schema".into(), table: "t".into() },
            None, None, None,
            Some(ColumnDef::new("c".into(), Some(SqliteStorageClass::Text),
                vec![ColumnConstraint::ForeignKey(ForeignKeyClause {
                    foreign_table: "ft".into(),
                    references_columns: vec![],
                    on_delete: Some(ForeignKeyAction::Cascade),
                    on_update: Some(ForeignKeyAction::Restrict),
                    match_type: None,
                    deferrable: false,
                    initially_deferred: false,
                })])),
            None,
        )]
    }
}

#[allow(unused_macros)]
macro_rules! test_group_fail {
    ($group_name:ident,$($ident:ident:$input:literal),*) => {
    mod $group_name {
        use crate::{lexer, parser::Parser};

        $(
            #[test]
            fn $ident() {
                let input = $input.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&input, "parser_test_fail");
                let toks = l.run();
                assert_eq!(l.errors.len(), 0);

                let mut parser = Parser::new(toks, "parser_test_fail");
                let _ = parser.parse();
                assert_ne!(parser.errors.len(), 0);
            }
        )*
        }
    };
}

#[allow(unused_macros)]
macro_rules! test_group_fail_assert {
    ($group_name:ident,$($ident:ident:$input:literal => $rule:expr, $note:literal),*) => {
    mod $group_name {
        use crate::{lexer, parser::Parser, types::rules::Rule};

        $(
            #[test]
            fn $ident() {
                let input = $input.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&input, "parser_test_fail");
                let toks = l.run();
                assert_eq!(l.errors.len(), 0);

                let mut parser = Parser::new(toks, "parser_test_fail");
                let _ = parser.parse();
                assert_ne!(parser.errors.len(), 0);

                let error = &parser.errors[0];
                assert_eq!(error.rule, $rule);
                assert!(
                    error.note.contains($note),
                    "expected note to contain {:?}, got {:?}",
                    $note,
                    error.note
                );
            }
        )*
        }
    };
}

#[allow(unused_macros)]
macro_rules! test_group_analysis_assert {
    ($group_name:ident,$($ident:ident:$input:literal => $rule:expr, $note:literal),*) => {
    mod $group_name {
        use crate::{lexer, parser::Parser, types::rules::Rule};

        $(
            #[test]
            fn $ident() {
                let input = $input.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&input, "parser_test_analysis");
                let toks = l.run();
                assert_eq!(l.errors.len(), 0);

                let mut parser = Parser::new(toks, "parser_test_analysis");
                let ast = parser.parse();
                assert_eq!(parser.errors.len(), 0);

                let diagnostics = ast
                    .iter()
                    .flat_map(|node| node.analyse("parser_test_analysis"))
                    .collect::<Vec<_>>();
                assert!(
                    diagnostics
                        .iter()
                        .any(|error| error.rule == $rule && error.note.contains($note)),
                    "expected a {:?} diagnostic with note containing {:?}, got {:?}",
                    $rule,
                    $note,
                    diagnostics
                );
            }
        )*
        }
    };
}

#[allow(unused_macros)]
macro_rules! test_group_analysis_pass {
    ($group_name:ident,$($ident:ident:$input:literal),*) => {
    mod $group_name {
        use crate::{lexer, parser::Parser};

        $(
            #[test]
            fn $ident() {
                let input = $input.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&input, "parser_test_analysis");
                let toks = l.run();
                assert_eq!(l.errors.len(), 0);

                let mut parser = Parser::new(toks, "parser_test_analysis");
                let ast = parser.parse();
                assert_eq!(parser.errors.len(), 0);

                let diagnostics = ast
                    .iter()
                    .flat_map(|node| node.analyse("parser_test_analysis"))
                    .collect::<Vec<_>>();
                assert_eq!(diagnostics.len(), 0);
            }
        )*
        }
    };
}

#[cfg(test)]
mod should_fail {
    test_group_fail! {
        negative_tests,
        eof_semi: ";",
        eof_literal: "'str'",
        alter_no_table: "ALTER;",
        alter_no_name: "ALTER TABLE;",
        commit_no_semicolon: "COMMIT",
        end_no_semicolon: "END",
        rollback_no_semicolon: "ROLLBACK",
        rollback_to_savepoint_no_name: "ROLLBACK TO SAVEPOINT",
        begin_no_semicolon: "BEGIN",
        begin_invalid_modifiers: "BEGIN DEFERRED IMMEDIATE EXCLUSIVE EXCLUSIVE;",
        detach_no_name: "DETACH;",
        detach_invalid_literal: "DETACH 'bad';",
        drop_no_object: "DROP TABLE;",
        drop_invalid_object: "DROP INDEX 5;",
        savepoint_no_name: "SAVEPOINT;",
        release_no_name: "RELEASE;",
        reindex_no_name: "REINDEX",
        reindex_invalid_literal: "REINDEX 25;",
        vacuum_no_semicolon: "VACUUM",
        vacuum_invalid_combined: "VACUUM 5 INTO 5;",
        attach_missing_expr: "ATTACH AS db;",
        attach_bind_without_ident: "ATTACH : AS db;",
        pragma_missing_name: "PRAGMA;",
        commit_no_semicolon_keyword: "COMMIT FOO;",
        explain_without_stmt: "EXPLAIN;"
    }

    test_group_fail_assert! {
        diagnostic_tests,
        analyze_invalid_target: "ANALYZE 12;" => Rule::Syntax, "expected either schema_name.table or table",
        invalid_foreign_key_match: "ALTER TABLE schema.table_name ADD COLUMN column_name TEXT REFERENCES foreign_table MATCH wat;" => Rule::Syntax, "Wanted FULL, PARTIAL or SIMPLE after MATCH",
        create_table_empty_column_list: "CREATE TABLE users ();" => Rule::Syntax, "requires at least one column definition",
        create_table_trailing_comma: "CREATE TABLE users (id INTEGER,);" => Rule::Syntax, "Expected Ident(<name>)",
        create_table_as_select_unimplemented: "CREATE TABLE users AS SELECT id FROM old_users;" => Rule::Unimplemented, "CREATE TABLE ... AS <select_stmt> is not yet supported",
        create_virtual_table_unimplemented: "CREATE VIRTUAL TABLE docs USING fts5(content);" => Rule::Unimplemented, "CREATE VIRTUAL TABLE is not yet supported",
        create_table_unknown_option: "CREATE TABLE users (id INTEGER) ROWID;" => Rule::Syntax, "expected STRICT or WITHOUT ROWID",
        create_table_without_missing_rowid: "CREATE TABLE users (id INTEGER) WITHOUT;" => Rule::Syntax, "Wanted Keyword(ROWID)",
        create_table_option_trailing_comma: "CREATE TABLE users (id INTEGER) STRICT,;" => Rule::Syntax, "trailing comma",
        create_table_primary_key_constraint_empty: "CREATE TABLE users (id INTEGER, PRIMARY KEY ());" => Rule::Syntax, "PRIMARY KEY requires at least one indexed column",
        create_table_constraint_missing_kind: "CREATE TABLE users (id INTEGER, CONSTRAINT bad NOT NULL);" => Rule::Syntax, "expected PRIMARY KEY, UNIQUE, CHECK or FOREIGN KEY",
        create_table_unique_constraint_trailing_comma: "CREATE TABLE users (email TEXT, UNIQUE (email,));" => Rule::Syntax, "UNIQUE indexed column list has a trailing comma",
        create_table_foreign_key_constraint_empty: "CREATE TABLE users (team_id INTEGER, FOREIGN KEY () REFERENCES teams);" => Rule::Syntax, "FOREIGN KEY requires at least one column name",
        create_table_foreign_key_constraint_trailing_comma: "CREATE TABLE users (team_id INTEGER, FOREIGN KEY (team_id,) REFERENCES teams);" => Rule::Syntax, "FOREIGN KEY column list has a trailing comma",
        create_temp_index_invalid: "CREATE TEMP INDEX idx ON users (id);" => Rule::Syntax, "CREATE INDEX does not support TEMP or TEMPORARY",
        create_index_missing_name: "CREATE INDEX ON users (id);" => Rule::Syntax, "expected either schema_name.index or index",
        create_index_missing_on: "CREATE INDEX idx users (id);" => Rule::Syntax, "Wanted Keyword(ON)",
        create_index_missing_table: "CREATE INDEX idx ON (id);" => Rule::Syntax, "Expected Ident(<table_name>)",
        create_index_empty_column_list: "CREATE INDEX idx ON users ();" => Rule::Syntax, "requires at least one indexed column",
        create_index_trailing_comma: "CREATE INDEX idx ON users (id,);" => Rule::Syntax, "trailing comma",
        create_index_expression_unimplemented: "CREATE INDEX idx ON users ((lower(name)));" => Rule::Unimplemented, "expression indexes are not yet supported",
        create_index_where_unimplemented: "CREATE INDEX idx ON users (id) WHERE active;" => Rule::Unimplemented, "partial indexes are not yet supported",
        create_unique_table_invalid: "CREATE UNIQUE TABLE users (id INTEGER);" => Rule::Syntax, "CREATE UNIQUE is only valid for INDEX",
        create_unique_view_invalid: "CREATE UNIQUE VIEW users AS SELECT id FROM old_users;" => Rule::Syntax, "CREATE UNIQUE is only valid for INDEX",
        create_view_empty_column_list: "CREATE VIEW active_users () AS SELECT id FROM users;" => Rule::Syntax, "requires at least one column name",
        create_view_trailing_comma: "CREATE VIEW active_users (id,) AS SELECT id FROM users;" => Rule::Syntax, "trailing comma",
        create_view_missing_name: "CREATE VIEW AS SELECT id FROM users;" => Rule::Syntax, "expected either schema_name.view or view",
        create_view_missing_as: "CREATE VIEW active_users SELECT id FROM users;" => Rule::Syntax, "Wanted Keyword(AS)",
        create_view_missing_select: "CREATE VIEW active_users AS 12;" => Rule::Syntax, "requires select-stmt after AS",
        create_view_select_unimplemented: "CREATE VIEW active_users AS SELECT id FROM users;" => Rule::Unimplemented, "CREATE VIEW ... AS <select_stmt> is not yet supported",
        create_view_column_list_select_unimplemented: "CREATE TEMP VIEW IF NOT EXISTS temp.active_users (id, name) AS SELECT id, name FROM users;" => Rule::Unimplemented, "CREATE VIEW ... AS <select_stmt> is not yet supported",
        create_trigger_missing_name: "CREATE TRIGGER AFTER INSERT ON users BEGIN SELECT 1; END;" => Rule::Syntax, "expected either schema_name.trigger or trigger",
        create_trigger_missing_event: "CREATE TRIGGER user_ai ON users BEGIN SELECT 1; END;" => Rule::Syntax, "expected BEFORE, AFTER, INSTEAD OF, DELETE, INSERT or UPDATE",
        create_trigger_update_of_missing_column: "CREATE TRIGGER user_au UPDATE OF ON users BEGIN SELECT 1; END;" => Rule::Syntax, "Expected Ident(<column_name>)",
        create_trigger_update_of_trailing_comma: "CREATE TRIGGER user_au UPDATE OF name, ON users BEGIN SELECT 1; END;" => Rule::Syntax, "trailing comma",
        create_trigger_missing_on: "CREATE TRIGGER user_ai AFTER INSERT users BEGIN SELECT 1; END;" => Rule::Syntax, "Wanted Keyword(ON)",
        create_trigger_for_missing_each: "CREATE TRIGGER user_ai AFTER INSERT ON users FOR ROW BEGIN SELECT 1; END;" => Rule::Syntax, "Wanted Keyword(EACH)",
        create_trigger_for_missing_row: "CREATE TRIGGER user_ai AFTER INSERT ON users FOR EACH BEGIN SELECT 1; END;" => Rule::Syntax, "Wanted Keyword(ROW)",
        create_trigger_when_missing_expr: "CREATE TRIGGER user_ai AFTER INSERT ON users WHEN BEGIN SELECT 1; END;" => Rule::Syntax, "WHEN requires an expression",
        create_trigger_body_missing_semicolon: "CREATE TRIGGER user_ai AFTER INSERT ON users BEGIN SELECT 1 END;" => Rule::Semicolon, "terminate statements with ';'",
        create_trigger_empty_body: "CREATE TRIGGER user_ai AFTER INSERT ON users BEGIN END;" => Rule::Syntax, "body requires at least one trigger statement",
        create_trigger_invalid_body_stmt: "CREATE TRIGGER user_ai AFTER INSERT ON users BEGIN PRAGMA database_list; END;" => Rule::Syntax, "body expected DELETE, INSERT, SELECT or UPDATE",
        delete_missing_from: "DELETE users;" => Rule::Syntax, "Wanted Keyword(FROM)",
        delete_missing_table: "DELETE FROM;" => Rule::Syntax, "expected either schema_name.table or table",
        delete_indexed_missing_by: "DELETE FROM users INDEXED idx;" => Rule::Syntax, "Wanted Keyword(BY)",
        delete_not_indexed_missing_indexed: "DELETE FROM users NOT idx;" => Rule::Syntax, "Wanted Keyword(INDEXED)",
        delete_returning_trailing_comma: "DELETE FROM users RETURNING id,;" => Rule::Syntax, "result column list has a trailing comma",
        delete_order_by_trailing_comma: "DELETE FROM users ORDER BY id, LIMIT 1;" => Rule::Syntax, "ORDER BY term list has a trailing comma",
        delete_order_by_nulls_invalid: "DELETE FROM users ORDER BY id NULLS NO;" => Rule::Syntax, "Wanted FIRST or LAST after NULLS",
        delete_with_select_unimplemented: "WITH stale AS (SELECT 1) DELETE FROM users;" => Rule::Unimplemented, "SELECT in WITH common table expressions is not yet supported",
        delete_with_recursive_select_unimplemented: "WITH RECURSIVE stale AS NOT MATERIALIZED (SELECT 1) DELETE FROM users;" => Rule::Unimplemented, "SELECT in WITH common table expressions is not yet supported",
        delete_with_body_unimplemented: "WITH stale AS (VALUES (1)) DELETE FROM users;" => Rule::Unimplemented, "WITH common table expression bodies require select-stmt support",
        insert_missing_into: "INSERT users VALUES (1);" => Rule::Syntax, "Wanted Keyword(INTO)",
        insert_bad_conflict_algorithm: "INSERT OR NO INTO users VALUES (1);" => Rule::Syntax, "Wanted ROLLBACK, ABORT, REPLACE, FAIL or IGNORE after INSERT OR",
        insert_missing_source: "INSERT INTO users;" => Rule::Syntax, "INSERT expected DEFAULT VALUES, VALUES or select-stmt",
        insert_empty_column_list: "INSERT INTO users () VALUES (1);" => Rule::Syntax, "requires at least one column name",
        insert_column_list_trailing_comma: "INSERT INTO users (id,) VALUES (1);" => Rule::Syntax, "column list has a trailing comma",
        insert_empty_values_row: "INSERT INTO users VALUES ();" => Rule::Syntax, "INSERT VALUES row requires at least one expression",
        insert_values_row_trailing_comma: "INSERT INTO users VALUES (1,);" => Rule::Syntax, "INSERT VALUES row has a trailing comma",
        insert_values_rows_trailing_comma: "INSERT INTO users VALUES (1),;" => Rule::Syntax, "INSERT VALUES row list has a trailing comma",
        insert_with_select_unimplemented: "WITH stale AS (SELECT 1) INSERT INTO users VALUES (1);" => Rule::Unimplemented, "SELECT in WITH common table expressions is not yet supported",
        insert_select_unimplemented: "INSERT INTO users SELECT id FROM old_users;" => Rule::Unimplemented, "INSERT ... <select-stmt> is not yet supported",
        insert_upsert_unimplemented: "INSERT INTO users VALUES (1) ON CONFLICT(id) DO NOTHING;" => Rule::Unimplemented, "INSERT upsert-clause is not yet supported",
        update_missing_table: "UPDATE SET name = 'Ada';" => Rule::Syntax, "expected either schema_name.table or table",
        update_missing_set: "UPDATE users name = 'Ada';" => Rule::Syntax, "Wanted Keyword(SET)",
        update_bad_conflict_algorithm: "UPDATE OR NO users SET name = 'Ada';" => Rule::Syntax, "Wanted ROLLBACK, ABORT, REPLACE, FAIL or IGNORE after UPDATE OR",
        update_assignment_missing_column: "UPDATE users SET = 'Ada';" => Rule::Syntax, "Expected Ident(<column_name>)",
        update_assignment_missing_equal: "UPDATE users SET name 'Ada';" => Rule::Syntax, "Wanted Equal",
        update_assignment_trailing_comma: "UPDATE users SET name = 'Ada', WHERE id = 1;" => Rule::Syntax, "UPDATE assignment list has a trailing comma",
        update_column_list_empty: "UPDATE users SET () = user_defaults();" => Rule::Syntax, "requires at least one column name",
        update_column_list_trailing_comma: "UPDATE users SET (name,) = user_defaults();" => Rule::Syntax, "column list has a trailing comma",
        update_from_unimplemented: "UPDATE users SET name = old_users.name FROM old_users WHERE users.id = old_users.id;" => Rule::Unimplemented, "UPDATE FROM is not yet supported",
        update_with_select_unimplemented: "WITH stale AS (SELECT 1) UPDATE users SET name = 'Ada';" => Rule::Unimplemented, "SELECT in WITH common table expressions is not yet supported",
        select_trailing_result_column: "SELECT id, FROM users;" => Rule::Syntax, "result column list has a trailing comma",
        select_from_trailing_comma: "SELECT id FROM users, WHERE active = true;" => Rule::Syntax, "SELECT FROM table list has a trailing comma",
        select_distinct_unimplemented: "SELECT DISTINCT id FROM users;" => Rule::Unimplemented, "SELECT DISTINCT/ALL is not yet supported",
        select_join_unimplemented: "SELECT id FROM users JOIN teams;" => Rule::Unimplemented, "SELECT JOIN clauses are not yet supported",
        select_group_by_unimplemented: "SELECT team_id FROM users GROUP BY team_id;" => Rule::Unimplemented, "SELECT GROUP BY/HAVING is not yet supported",
        select_window_unimplemented: "SELECT id FROM users WINDOW win AS ();" => Rule::Unimplemented, "SELECT WINDOW clauses are not yet supported",
        select_compound_unimplemented: "SELECT id FROM users UNION SELECT id FROM archived_users;" => Rule::Unimplemented, "compound SELECT statements are not yet supported",
        select_result_subquery_unimplemented: "SELECT (SELECT 1);" => Rule::Unimplemented, "SELECT subqueries in result columns are not yet supported",
        select_from_subquery_unimplemented: "SELECT id FROM (SELECT id FROM users);" => Rule::Unimplemented, "SELECT FROM subqueries are not yet supported",

        // sql_stmt dispatch
        unknown_keyword_suggestion: "usrs;" => Rule::UnknownKeyword, "did you mean one of",
        literal_cannot_start_statement: "12;" => Rule::Syntax, "can not start a statement",

        // create_stmt dispatch
        create_unknown_object: "CREATE DATABASE foo;" => Rule::Syntax, "CREATE requires TABLE,INDEX,TRIGGER or VIEW",

        // alter_stmt dispatch
        alter_invalid_action: "ALTER TABLE t FOO;" => Rule::Syntax, "ALTER requires either RENAME, ADD or DROP",

        // drop_stmt dispatch
        drop_invalid_object_keyword: "DROP DATABASE foo;" => Rule::Syntax, "DROP requires either",

        // transaction statements
        begin_invalid_token: "BEGIN FOO;" => Rule::Syntax, "Wanted any of TRANSACTION, DEFERRED, IMMEDIATE or EXCLUSIVE",
        commit_invalid_token: "COMMIT FOO;" => Rule::Syntax, "Wanted Keyword(TRANSACTION) or Semicolon",
        rollback_invalid_token: "ROLLBACK FOO;" => Rule::Syntax, "ROLLBACK requires TRANSACTION, TO or to end",
        rollback_to_invalid_token: "ROLLBACK TO 5;" => Rule::Syntax, "ROLLBACK requires SAVEPOINT, Ident or to end",

        // pragma_stmt
        pragma_invalid_rhs: "PRAGMA foo bar;" => Rule::Syntax, "A pragma rhs value has to be",
        pragma_bad_assign_value: "PRAGMA foo = ;" => Rule::Syntax, "A pragma assignment value has to be",
        pragma_bad_call_value: "PRAGMA foo();" => Rule::Syntax, "A pragma call value has to be",

        // vacuum_stmt
        vacuum_into_non_string: "VACUUM INTO 5;" => Rule::Syntax, "for VACUUM stmt with Keyword(INTO)",

        // conflict_clause
        conflict_invalid_keyword: "ALTER TABLE schema.t ADD COLUMN c TEXT NOT NULL ON CONFLICT FOO;" => Rule::Syntax, "Wanted either ROLLBACK, ABORT, FAIL, IGNORE or REPLACE",
        conflict_non_keyword: "ALTER TABLE schema.t ADD COLUMN c TEXT NOT NULL ON CONFLICT 5;" => Rule::Syntax, "Wanted either ROLLBACK, ABORT, FAIL, IGNORE or REPLACE",

        // foreign_key_clause
        fk_on_invalid_event: "ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON FOO;" => Rule::Syntax, "Wanted DELETE or UPDATE",
        fk_action_invalid: "ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON DELETE FOO;" => Rule::Syntax, "Wanted SET, CASCADE, RESTRICT or NO",
        fk_set_invalid: "ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft ON DELETE SET FOO;" => Rule::Syntax, "Wanted NULL or DEFAULT after SET",
        fk_deferrable_initially_invalid: "ALTER TABLE schema.t ADD COLUMN c TEXT REFERENCES ft DEFERRABLE INITIALLY FOO;" => Rule::Syntax, "Wanted DEFERRED or IMMEDIATE after DEFERRABLE INITIALLY",

        // column type / type-name parameters
        non_canonical_type: "CREATE TABLE t (a NUMERIC);" => Rule::Quirk, "Consider using a canonical sqlite type",
        type_param_non_number: "ALTER TABLE schema.t ADD COLUMN c INTEGER(z);" => Rule::Syntax, "Wanted a Number after Type::BraceLeft",
        type_param_second_non_number: "ALTER TABLE schema.t ADD COLUMN c INTEGER(10, z);" => Rule::Syntax, "Wanted a Number after Type::BraceLeft, Type::Number and Type::Comma",

        // schema_table_container malformed paths
        schema_table_keyword: "ANALYZE schema_name.SELECT;" => Rule::Syntax, "is a keyword, if you want to use it",
        schema_table_malformed: "ANALYZE schema_name.(;" => Rule::Syntax, "expected a table name after <schema_name>.",

        // expr invalid construct
        expr_invalid_construct: "ATTACH AS db;" => Rule::Syntax, "is not a valid construct",
        expr_column_reference_too_deep: "ALTER TABLE schema.t ADD COLUMN c TEXT CHECK (a.b.c.d);" => Rule::Syntax, "at most schema.table.column",
        expr_function_trailing_comma: "ALTER TABLE schema.t ADD COLUMN c TEXT CHECK (length(c,));" => Rule::Syntax, "trailing comma",
        bind_without_identifier: "ATTACH : AS db;" => Rule::Syntax, "requires an identifier as a postfix"
    }
}

#[cfg(test)]
mod should_analyze {
    test_group_analysis_assert! {
        diagnostic_tests,
        create_table_recommends_strict:
            "CREATE TABLE users (id INTEGER);" => Rule::Quirk, "Add STRICT",
        create_table_column_missing_type:
            "CREATE TABLE users (name);" => Rule::Quirk, "SQLite allows columns without a declared type",
        alter_add_column_missing_type:
            "ALTER TABLE users ADD COLUMN name;" => Rule::Quirk, "SQLite allows columns without a declared type",
        nullable_column_primary_key:
            "CREATE TABLE users (email TEXT PRIMARY KEY);" => Rule::Quirk, "PRIMARY KEY columns",
        deprecated_pragma:
            "PRAGMA case_sensitive_like = true;" => Rule::Quirk, "deprecated",
        foreign_keys_disabled:
            "PRAGMA foreign_keys = false;" => Rule::Quirk, "foreign key constraints",
        ignore_check_constraints_enabled:
            "PRAGMA ignore_check_constraints = 1;" => Rule::Quirk, "CHECK constraint enforcement",
        writable_schema_enabled:
            "PRAGMA writable_schema = ON;" => Rule::Quirk, "corrupt the database",
        pragma_unsupported_documented_form:
            "PRAGMA foreign_keys(true);" => Rule::Syntax, "query or assignment form",
        pragma_unsupported_documented_value:
            "PRAGMA foreign_keys = maybe;" => Rule::Syntax, "a boolean value",
        unknown_pragma:
            "PRAGMA some_extension_pragma;" => Rule::UnknownPragma, "SQLite ignores unknown PRAGMAs"
    }

    test_group_analysis_pass! {
        negative_diagnostic_tests,
        create_table_strict_has_no_recommendation:
            "CREATE TABLE users (id INTEGER) STRICT;",
        foreign_keys_query_has_no_recommendation:
            "PRAGMA foreign_keys;",
        foreign_keys_enabled_has_no_recommendation:
            "PRAGMA foreign_keys = true;",
        ignore_check_constraints_disabled_has_no_recommendation:
            "PRAGMA ignore_check_constraints = 0;",
        known_pragma_without_analysis_rule_has_no_recommendation:
            "PRAGMA application_id;"
    }
}
