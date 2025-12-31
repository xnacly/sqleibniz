#[allow(unused_macros)]
macro_rules! test_group_pass_assert {
    ($group_name:ident,$($ident:ident:$input:literal=$expected:expr),*) => {
    mod $group_name {
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
        // TODO: somehow there is no ADD column in this test:
        foreign_key_clause_ast_bug_exception,

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
        )]
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
                        None, None, None, None
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
                        t: Token::new(Type::String("literal".into()))
                    }),
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
                        None, None, None, None
                    ),
                    stored_virtual: Some(Keyword::STORED),
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
                        None, None, None, None
                    )
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
        vacuum_invalid_combined: "VACUUM 5 INTO 5;"
    }
}
