#[allow(unused_macros)]
macro_rules! test_group_pass_assert {
    ($group_name:ident,$($ident:ident:$input:literal=$expected:expr),*) => {
    mod $group_name {
        #[allow(unused_imports)]
        use crate::{lexer, parser::Parser, parser::nodes::*, types::*};

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
                        .map(|n| n.unwrap().as_serializable())
                        .collect::<Vec<_>>(),
                ).unwrap();
                let serialized_expected = serde_json::to_string(
                    &$expected.into_iter()
                        .map(|n| n.as_serializable())
                        .collect::<Vec<_>>(),
                    )
                .unwrap();
                assert_eq!(serialized_expected, serialized_ast);
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

    // test_group_pass_assert! {
    //     commit_stmt,
    //     commit:            r"COMMIT;"=vec![Type::Keyword(Keyword::COMMIT)],
    //     end:               r"END;"=vec![Type::Keyword(Keyword::END)],
    //     commit_transaction:r"COMMIT TRANSACTION;"=vec![Type::Keyword(Keyword::COMMIT)],
    //     end_transaction:   r"END TRANSACTION;"=vec![Type::Keyword(Keyword::END)]
    // }

    // test_group_pass_assert! {
    //     rollback_stmt,

    //     rollback:r"ROLLBACK;"=vec![Type::Keyword(Keyword::ROLLBACK)],
    //     rollback_to_save_point:r"ROLLBACK TO save_point;"=vec![Type::Keyword(Keyword::ROLLBACK)],
    //     rollback_to_savepoint_save_point:r"ROLLBACK TO SAVEPOINT save_point;"=vec![Type::Keyword(Keyword::ROLLBACK)],
    //     rollback_transaction:r"ROLLBACK TRANSACTION;"=vec![Type::Keyword(Keyword::ROLLBACK)],
    //     rollback_transaction_to_save_point:r"ROLLBACK TRANSACTION TO save_point;"=vec![Type::Keyword(Keyword::ROLLBACK)],
    //     rollback_transaction_to_savepoint_save_point:r"ROLLBACK TRANSACTION TO SAVEPOINT save_point;"=vec![Type::Keyword(Keyword::ROLLBACK)]
    // }

    // test_group_pass_assert! {
    //     detach_stmt,

    //     detach_schema_name:r"DETACH schema_name;"=vec![Type::Keyword(Keyword::DETACH)],
    //     detach_database_schema_name:r"DETACH DATABASE schema_name;"=vec![Type::Keyword(Keyword::DETACH)]
    // }

    // test_group_pass_assert! {
    //     analyze_stmt,

    //     analyze:r"ANALYZE;"=vec![Type::Keyword(Keyword::ANALYZE)],
    //     analyze_schema_name:r"ANALYZE schema_name;"=vec![Type::Keyword(Keyword::ANALYZE)],
    //     analyze_index_or_table_name:r"ANALYZE index_or_table_name;"=vec![Type::Keyword(Keyword::ANALYZE)],
    //     analyze_schema_name_with_subtable:r"ANALYZE schema_name.index_or_table_name;"=vec![Type::Keyword(Keyword::ANALYZE)]
    // }

    // test_group_pass_assert! {
    //     drop_stmt,

    //     drop_index_index_name:r"DROP INDEX index_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_index_if_exists_schema_name_index_name:r"DROP INDEX IF EXISTS schema_name.index_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_table_table_name:r"DROP TABLE table_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_table_if_exists_schema_name_table_name:r"DROP TABLE IF EXISTS schema_name.table_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_trigger_trigger_name:r"DROP TRIGGER trigger_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_trigger_if_exists_schema_name_trigger_name:r"DROP TRIGGER IF EXISTS schema_name.trigger_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_view_view_name:r"DROP VIEW view_name;"=vec![Type::Keyword(Keyword::DROP)],
    //     drop_view_if_exists_schema_name_view_name:r"DROP VIEW IF EXISTS schema_name.view_name;"=vec![Type::Keyword(Keyword::DROP)]
    // }

    // test_group_pass_assert! {
    //     savepoint_stmt,

    //     savepoint_savepoint_name:r"SAVEPOINT savepoint_name;"=vec![Type::Keyword(Keyword::SAVEPOINT)]
    // }

    // test_group_pass_assert! {
    //     release_stmt,

    //     release_savepoint_savepoint_name:r"RELEASE SAVEPOINT savepoint_name;"=vec![Type::Keyword(Keyword::RELEASE)],
    //     release_savepoint_name:r"RELEASE savepoint_name;"=vec![Type::Keyword(Keyword::RELEASE)]
    // }

    // test_group_pass_assert! {
    //     attach_stmt,

    //     attach:r"ATTACH 'database.db' AS db;"=vec![Type::Keyword(Keyword::ATTACH)],
    //     attach_database:r"ATTACH DATABASE 'database.db' AS db;"=vec![Type::Keyword(Keyword::ATTACH)]
    // }

    // test_group_pass_assert! {
    //     reindex_stmt,

    //     reindex:r"REINDEX;"=vec![Type::Keyword(Keyword::REINDEX)],
    //     reindex_collation_name:r"REINDEX collation_name;"=vec![Type::Keyword(Keyword::REINDEX)],
    //     reindex_schema_name_table_name:r"REINDEX schema_name.table_name;"=vec![Type::Keyword(Keyword::REINDEX)]
    // }

    // test_group_pass_assert! {
    //     alter_stmt,

    //     alter_rename_to:r"ALTER TABLE schema.table_name RENAME TO new_table;"=vec![Type::Keyword(Keyword::ALTER)],
    //     alter_rename_colum_to:r"ALTER TABLE schema.table_name RENAME COLUMN old_column_name TO new_column_name;"=vec![Type::Keyword(Keyword::ALTER)],
    //     alter_rename_colum_to_without_column_keyword:r"ALTER TABLE schema.table_name RENAME old_column_name TO new_column_name;"=vec![Type::Keyword(Keyword::ALTER)],

    //     alter_add:r"ALTER TABLE schema.table_name ADD COLUMN column_name TEXT;"=vec![Type::Keyword(Keyword::ALTER)],
    //     alter_add_without_column_keyword:r"ALTER TABLE schema.table_name ADD column_name TEXT;"=vec![Type::Keyword(Keyword::ALTER)],

    //     alter_drop_column:r"ALTER TABLE schema.table_name DROP COLUMN column_name;"=vec![Type::Keyword(Keyword::ALTER)],
    //     alter_drop_column_without_column_keyword:r"ALTER TABLE schema.table_name DROP column_name;"=vec![Type::Keyword(Keyword::ALTER)]
    // }

    // test_group_pass_assert! {
    //     column_constraint,

    //     primary_key_no_order_no_conflict_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_no_conflict_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         ASC;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_desc_no_conflict_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_conflict_rollback_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC
    //         ON CONFLICT ROLLBACK;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_conflict_abort_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC
    //         ON CONFLICT ABORT;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_conflict_fail_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC
    //         ON CONFLICT FAIL;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_conflict_ignore_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC
    //         ON CONFLICT IGNORE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_conflict_replace_no_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC
    //         ON CONFLICT REPLACE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     primary_key_asc_conflict_replace_autoincrement:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         PRIMARY KEY
    //         DESC
    //         ON CONFLICT REPLACE
    //         AUTOINCREMENT;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     not_null_no_conflict:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         NOT NULL;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     // i am not retesting the conflicts here, because i tested all cases above
    //     not_null_conflict:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         NOT NULL ON CONFLICT REPLACE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     unique_no_conflict:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         UNIQUE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     unique_conflict:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         UNIQUE ON CONFLICT REPLACE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     check:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         CHECK ('literal string lol');
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     default_expr:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         DEFAULT ('default string');
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     default_literal:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         DEFAULT 'literal';
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     default_signed_number:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         DEFAULT 25;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     collate:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         COLLATE collation_name;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     generated_always_as_expr_stored:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         GENERATED ALWAYS AS ('literal') STORED;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     generated_always_as_expr_virtual:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         GENERATED ALWAYS AS ('literal') VIRTUAL;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     generated_always_as_expr_no_postfix:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         GENERATED ALWAYS AS ('literal');
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     as_expr_no_postfix:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         AS ('literal');
    //     "=vec![Type::Keyword(Keyword::ALTER)]
    // }

    // test_group_pass_assert! {
    //     foreign_key_clause,

    //     references_with_optional_column_names:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table (colum1, colum2);
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_with_optional_column_name:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table (colum1);
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_delete_set_null:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON DELETE SET NULL;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_delete_set_default:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON DELETE SET DEFAULT;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_delete_cascade:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON DELETE CASCADE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_delete_restrict:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON DELETE RESTRICT;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_delete_no_action:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON DELETE NO ACTION;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_update_set_null:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON UPDATE SET NULL;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_update_set_default:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON UPDATE SET DEFAULT;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_update_cascade:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON UPDATE CASCADE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_update_restrict:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON UPDATE RESTRICT;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_on_update_no_action:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table ON UPDATE NO ACTION;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_match_name:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table MATCH name;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_deferrable:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table DEFERRABLE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_not_deferrable:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table NOT DEFERRABLE;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_not_deferrable_initially_deferred:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table NOT DEFERRABLE INITIALLY DEFERRED;
    //     "=vec![Type::Keyword(Keyword::ALTER)],

    //     references_not_deferrable_initially_immediate:r"
    //         ALTER TABLE schema.table_name
    //         ADD COLUMN column_name TEXT
    //         -- constraint:
    //         REFERENCES foreign_table NOT DEFERRABLE INITIALLY IMMEDIATE;
    //     "=vec![Type::Keyword(Keyword::ALTER)]
    // }
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
        edge_cases,
        eof_semi: ";",
        eof_hit_string: "'str'",
        eof_hit_number: "0x0",
        eof_hit_blob: "x''",
        eof_hit_null: "NULL",
        eof_hit_boolean: "true",
        eof_hit_cur_time: "CURRENT_TIME",
        eof_hit_cur_date: "CURRENT_DATE",
        eof_hit_cur_timestamp: "CURRENT_TIMESTAMP"
    }

    test_group_fail! {
        sql_stmt_prefix,
        explain: r#"EXPLAIN;"#,
        explain_query_plan: r#"EXPLAIN QUERY PLAN;"#
    }

    test_group_fail! {
        sql_vacuum,
        vacuum_no_semicolon: r#"VACUUM"#,
        vacuum_invalid_schema: r#"VACUUM 1;"#,
        vacuum_invalid_filename: r#"VACUUM INTO 5;"#,
        vacuum_invalid_combined: r#"VACUUM 5 INTO 5;"#
    }

    test_group_fail! {
        sql_begin,
        begin_no_semicolon: r#"BEGIN"#,
        begin_transaction_no_semicolon: r#"BEGIN TRANSACTION"#,
        begin_deferred_no_semicolon: r#"BEGIN DEFERRED"#,
        begin_immediate_no_semicolon: r#"BEGIN IMMEDIATE"#,
        begin_exclusive_no_semicolon: r#"BEGIN EXCLUSIVE"#,

        begin_transaction_with_literal: r#"BEGIN TRANSACTION 25;"#,
        begin_transaction_with_other_keyword: r#"BEGIN TRANSACTION AS;"#,
        begin_too_many_modifiers: r#"BEGIN DEFERRED IMMEDIATE EXCLUSIVE EXCLUSIVE;"#

    }

    test_group_fail! {
        commit_stmt,
        commit_no_semicolon:            r"COMMIT",
        end_no_semicolon:               r"END",
        commit_transaction_no_semicolon:r"COMMIT TRANSACTION",
        end_transaction_no_semicolon:   r"END TRANSACTION",

        commit_with_literal:            r"COMMIT 25;",
        end_with_literal:               r"END 12;",
        commit_transaction_with_literal:r"COMMIT TRANSACTION x'81938912';",
        end_transaction_with_literal:   r"END TRANSACTION 'kadl';"
    }

    test_group_fail! {
        rollback_stmt,

        rollback_no_semicolon:r"ROLLBACK",
        rollback_to_save_point_no_semicolon:r"ROLLBACK TO save_point",
        rollback_to_savepoint_save_point_no_semicolon:r"ROLLBACK TO SAVEPOINT save_point",
        rollback_transaction_no_semicolon:r"ROLLBACK TRANSACTION",
        rollback_transaction_to_save_point_no_semicolon:r"ROLLBACK TRANSACTION TO save_point",
        rollback_transaction_to_savepoint_save_point_no_semicolon:r"ROLLBACK TRANSACTION TO SAVEPOINT save_point",

        rollback_transaction_to_literal_save_point:r"ROLLBACK TRANSACTION TO SAVEPOINT 'hello';"
    }

    test_group_fail! {
        detach_stmt,

        detach_schema_name_no_semicolon:r"DETACH schema_name",
        detach_database_schema_name_no_semicolon:r"DETACH DATABASE schema_name",

        detach_schema_no_name:r"DETACH;",
        detach_database_no_schema_name:r"DETACH DATABASE;",
        detach_schema_literal_instead_of_name:r"DETACH 'this string should not be here';"
    }

    test_group_fail! {
        drop_stmt,

        drop_index_index_name_no_semicolon:r"DROP INDEX index_name",
        drop_index_if_exists_schema_name_index_name_no_semicolon:r"DROP INDEX IF EXISTS schema_name.index_name",
        drop_table_table_name_no_semicolon:r"DROP TABLE table_name",
        drop_table_if_exists_schema_name_table_name_no_semicolon:r"DROP TABLE IF EXISTS schema_name.table_name",
        drop_trigger_trigger_name_no_semicolon:r"DROP TRIGGER trigger_name",
        drop_trigger_if_exists_schema_name_trigger_name_no_semicolon:r"DROP TRIGGER IF EXISTS schema_name.trigger_name",
        drop_view_view_name_no_semicolon:r"DROP VIEW view_name",
        drop_view_if_exists_schema_name_view_name_no_semicolon:r"DROP VIEW IF EXISTS schema_name.view_name",

        drop_index_no_index_name:r"DROP INDEX;",
        drop_table_no_table_name:r"DROP TABLE;",
        drop_trigger_no_trigger_name:r"DROP TRIGGER;",
        drop_view_no_view_name:r"DROP VIEW;"
    }

    test_group_fail! {
        savepoint_stmt,

        savepoint_savepoint_name_no_semicolon:r"SAVEPOINT savepoint_name",

        savepoint_no_savepoint_name:r"SAVEPOINT;"
    }

    test_group_fail! {
        release_stmt,

        release_savepoint_savepoint_name_no_semicolon:r"RELEASE SAVEPOINT savepoint_name",
        release_savepoint_name_no_semicolon:r"RELEASE savepoint_name",

        release_savepoint_no_savepoint_name:r"RELEASE SAVEPOINT;",
        release_savepoint_no_name:r"RELEASE;"
    }

    test_group_fail! {
        reindex_stmt,

        reindex_no_semicolon:r"REINDEX",
        reindex_collation_name_no_semicolon:r"REINDEX collation_name",
        reindex_schema_name_table_name_no_semicolon:r"REINDEX schema_name.table_name",

        reindex_schema_name_no_table_or_index:r"REINDEX schema_name.;",
        reindex_collation_name_literal:r"REINDEX 25;"
    }

    test_group_fail! {
        alter_stmt,

        alter_no_table_after_alter:r"ALTER;",
        alter_no_tablename:r"ALTER TABLE;"
    }
}
