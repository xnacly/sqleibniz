use crate::{
    error::Error,
    parser::nodes::{Pragma, PragmaInvocation, SchemaTableContainer},
    types::{Keyword, Token, Type, rules::Rule},
};

pub fn pragma(file: &str, pragma: &Pragma) -> Vec<Error> {
    let name = pragma_name(pragma);

    PRAGMAS
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.analyse(file, pragma))
        .into_iter()
        .collect()
}

struct PragmaEntry {
    name: &'static str,
    analysis: PragmaAnalysis,
}

enum PragmaAnalysis {
    None,
    Deprecated {
        msg: &'static str,
        note: &'static str,
    },
    Check {
        diagnostic: fn(&str, &Pragma) -> Option<Error>,
        #[cfg(test)]
        diagnostic_value: Type,
    },
}

impl PragmaEntry {
    fn analyse(&self, file: &str, pragma: &Pragma) -> Option<Error> {
        match self.analysis {
            PragmaAnalysis::None => None,
            PragmaAnalysis::Deprecated { msg, note } => Some(
                Error::new(file, pragma.location, Rule::Quirk, msg, note)
                    .with_doc_url("https://www.sqlite.org/pragma.html"),
            ),
            PragmaAnalysis::Check { diagnostic, .. } => diagnostic(file, pragma),
        }
    }
}

const PRAGMAS: &[PragmaEntry] = &[
    PragmaEntry {
        name: "analysis_limit",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "application_id",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "auto_vacuum",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "automatic_index",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "busy_timeout",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "cache_size",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "cache_spill",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "case_sensitive_like",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA case_sensitive_like is deprecated",
            note: "SQLite documents case_sensitive_like as deprecated. Avoid new use because changing LIKE semantics can make existing schema objects appear corrupt until the setting is restored or indexes are rebuilt.",
        },
    },
    PragmaEntry {
        name: "cell_size_check",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "checkpoint_fullfsync",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "collation_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "compile_options",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "count_changes",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA count_changes is deprecated",
            note: "SQLite documents count_changes as deprecated. Avoid new use; sqlite3_changes() and sqlite3_total_changes() are the supported interfaces.",
        },
    },
    PragmaEntry {
        name: "data_store_directory",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA data_store_directory is deprecated",
            note: "SQLite documents data_store_directory as deprecated and not threadsafe. Avoid changing process-global SQLite directory state from SQL.",
        },
    },
    PragmaEntry {
        name: "data_version",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "database_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "default_cache_size",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA default_cache_size is deprecated",
            note: "SQLite documents default_cache_size as deprecated. Prefer PRAGMA cache_size for connection-local cache tuning.",
        },
    },
    PragmaEntry {
        name: "defer_foreign_keys",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "empty_result_callbacks",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA empty_result_callbacks is deprecated",
            note: "SQLite documents empty_result_callbacks as deprecated. Avoid new use.",
        },
    },
    PragmaEntry {
        name: "encoding",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "foreign_key_check",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "foreign_key_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "foreign_keys",
        analysis: PragmaAnalysis::Check {
            diagnostic: foreign_keys,
            #[cfg(test)]
            diagnostic_value: Type::Boolean(false),
        },
    },
    PragmaEntry {
        name: "freelist_count",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "full_column_names",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA full_column_names is deprecated",
            note: "SQLite documents full_column_names as deprecated. Avoid relying on deprecated result-column naming controls.",
        },
    },
    PragmaEntry {
        name: "fullfsync",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "function_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "hard_heap_limit",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "ignore_check_constraints",
        analysis: PragmaAnalysis::Check {
            diagnostic: ignore_check_constraints,
            #[cfg(test)]
            diagnostic_value: Type::Boolean(true),
        },
    },
    PragmaEntry {
        name: "incremental_vacuum",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "index_info",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "index_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "index_xinfo",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "integrity_check",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "journal_mode",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "journal_size_limit",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "legacy_alter_table",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "locking_mode",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "max_page_count",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "mmap_size",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "module_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "optimize",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "page_count",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "page_size",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "parser_trace",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "pragma_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "query_only",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "quick_check",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "read_uncommitted",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "recursive_triggers",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "reverse_unordered_selects",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "schema_version",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "secure_delete",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "short_column_names",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA short_column_names is deprecated",
            note: "SQLite documents short_column_names as deprecated. Avoid relying on deprecated result-column naming controls.",
        },
    },
    PragmaEntry {
        name: "shrink_memory",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "soft_heap_limit",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "synchronous",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "table_info",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "table_list",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "table_xinfo",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "temp_store",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "temp_store_directory",
        analysis: PragmaAnalysis::Deprecated {
            msg: "PRAGMA temp_store_directory is deprecated",
            note: "SQLite documents temp_store_directory as deprecated and not threadsafe. Avoid changing process-global SQLite directory state from SQL.",
        },
    },
    PragmaEntry {
        name: "threads",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "trusted_schema",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "user_version",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "vdbe_addoptrace",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "vdbe_debug",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "vdbe_listing",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "vdbe_trace",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "wal_autocheckpoint",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "wal_checkpoint",
        analysis: PragmaAnalysis::None,
    },
    PragmaEntry {
        name: "writable_schema",
        analysis: PragmaAnalysis::Check {
            diagnostic: writable_schema,
            #[cfg(test)]
            diagnostic_value: Type::Boolean(true),
        },
    },
];

fn foreign_keys(file: &str, pragma: &Pragma) -> Option<Error> {
    if !assigned_or_called_with(pragma, is_off) {
        return None;
    }

    Some(
        Error::new(
            file,
            pragma.location,
            Rule::Quirk,
            "PRAGMA foreign_keys disables foreign key enforcement",
            "SQLite does not enforce foreign key constraints when PRAGMA foreign_keys is OFF. Prefer enabling foreign key enforcement for each connection.",
        )
        .with_doc_url("https://www.sqlite.org/pragma.html#pragma_foreign_keys"),
    )
}

fn ignore_check_constraints(file: &str, pragma: &Pragma) -> Option<Error> {
    if !assigned_or_called_with(pragma, is_on) {
        return None;
    }

    Some(
        Error::new(
            file,
            pragma.location,
            Rule::Quirk,
            "PRAGMA ignore_check_constraints disables CHECK constraints",
            "SQLite skips CHECK constraint enforcement when PRAGMA ignore_check_constraints is enabled. Avoid enabling it outside narrow maintenance scripts.",
        )
        .with_doc_url("https://www.sqlite.org/pragma.html#pragma_ignore_check_constraints"),
    )
}

fn writable_schema(file: &str, pragma: &Pragma) -> Option<Error> {
    if !assigned_or_called_with(pragma, is_on) {
        return None;
    }

    Some(
        Error::new(
            file,
            pragma.location,
            Rule::Quirk,
            "PRAGMA writable_schema allows direct schema table writes",
            "SQLite warns that misuse of writable_schema can corrupt the database. Avoid enabling it unless you are deliberately performing low-level recovery or migration work.",
        )
        .with_doc_url("https://www.sqlite.org/pragma.html#pragma_writable_schema"),
    )
}

fn pragma_name(pragma: &Pragma) -> String {
    match &pragma.name {
        SchemaTableContainer::SchemaAndTable { table, .. } | SchemaTableContainer::Table(table) => {
            table.to_ascii_lowercase()
        }
    }
}

fn assigned_or_called_with(pragma: &Pragma, predicate: fn(&Token) -> bool) -> bool {
    match &pragma.invocation {
        PragmaInvocation::Assign { value } | PragmaInvocation::Call { value } => predicate(value),
        PragmaInvocation::Query => false,
    }
}

fn is_on(token: &Token) -> bool {
    match &token.ttype {
        Type::Boolean(true) => true,
        Type::Number(number) => *number != 0.0,
        Type::Keyword(Keyword::ON) => true,
        Type::Ident(value) | Type::String(value) => matches_on(value),
        _ => false,
    }
}

fn is_off(token: &Token) -> bool {
    match &token.ttype {
        Type::Boolean(false) => true,
        Type::Number(number) => *number == 0.0,
        Type::Ident(value) | Type::String(value) => matches_off(value),
        _ => false,
    }
}

fn matches_on(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "on" | "yes" | "true")
}

fn matches_off(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "off" | "no" | "false")
}

#[cfg(test)]
mod tests {
    use crate::{
        analyse::pragma::{PRAGMAS, PragmaAnalysis, pragma},
        parser::nodes::{Pragma, PragmaInvocation, SchemaTableContainer},
        types::{Token, Type, rules::Rule},
    };

    fn pragma_node(name: &str, invocation: PragmaInvocation) -> Pragma {
        Pragma::new(SchemaTableContainer::Table(name.into()), invocation)
    }

    fn assignment(name: &str, value: Type) -> Pragma {
        pragma_node(
            name,
            PragmaInvocation::Assign {
                value: Token::new(value),
            },
        )
    }

    #[test]
    fn pragma_table_is_sorted_and_unique() {
        for window in PRAGMAS.windows(2) {
            assert!(
                window[0].name < window[1].name,
                "PRAGMA table entries must be sorted and unique: {:?} came before {:?}",
                window[0].name,
                window[1].name
            );
        }
    }

    #[test]
    fn diagnostic_table_entries_emit_diagnostics() {
        for entry in PRAGMAS
            .iter()
            .filter(|entry| !matches!(entry.analysis, PragmaAnalysis::None))
        {
            let value = match &entry.analysis {
                PragmaAnalysis::None => unreachable!(),
                PragmaAnalysis::Deprecated { .. } => Type::Boolean(true),
                PragmaAnalysis::Check {
                    diagnostic_value, ..
                } => diagnostic_value.clone(),
            };
            let pragma = assignment(entry.name, value);
            let diagnostics = entry.analyse("test.sql", &pragma);

            assert!(
                diagnostics.is_some(),
                "expected PRAGMA {} to emit a diagnostic",
                entry.name
            );
        }
    }

    #[test]
    fn reports_deprecated_pragmas() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node("case_sensitive_like", PragmaInvocation::Query),
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::Quirk);
        assert!(diagnostics[0].note.contains("deprecated"));
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/pragma.html")
        );
    }

    #[test]
    fn accepts_unknown_pragmas() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node("some_extension_pragma", PragmaInvocation::Query),
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn accepts_known_pragmas_without_analysis_rules() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node("application_id", PragmaInvocation::Query),
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_foreign_key_enforcement_disabled() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node(
                "foreign_keys",
                PragmaInvocation::Assign {
                    value: Token::new(Type::Boolean(false)),
                },
            ),
        );

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].note.contains("foreign key constraints"));
    }

    #[test]
    fn accepts_foreign_key_query_and_enabled_values() {
        let query_diagnostics = pragma(
            "test.sql",
            &pragma_node("foreign_keys", PragmaInvocation::Query),
        );
        let enabled_diagnostics = pragma(
            "test.sql",
            &pragma_node(
                "foreign_keys",
                PragmaInvocation::Call {
                    value: Token::new(Type::Boolean(true)),
                },
            ),
        );

        assert!(query_diagnostics.is_empty());
        assert!(enabled_diagnostics.is_empty());
    }

    #[test]
    fn reports_ignored_check_constraints() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node(
                "ignore_check_constraints",
                PragmaInvocation::Assign {
                    value: Token::new(Type::Number(1.0)),
                },
            ),
        );

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].note.contains("CHECK constraint enforcement"));
    }

    #[test]
    fn reports_writable_schema_enabled() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node(
                "writable_schema",
                PragmaInvocation::Assign {
                    value: Token::new(Type::Ident("ON".into())),
                },
            ),
        );

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].note.contains("corrupt the database"));
    }
}
