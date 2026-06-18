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
        .and_then(|entry| entry.diagnostic(file, pragma))
        .into_iter()
        .collect()
}

struct PragmaEntry {
    name: &'static str,
    diagnostic: PragmaDiagnostic,
}

enum PragmaDiagnostic {
    Deprecated {
        msg: &'static str,
        note: &'static str,
    },
    Check(fn(&str, &Pragma) -> Option<Error>),
}

impl PragmaEntry {
    fn diagnostic(&self, file: &str, pragma: &Pragma) -> Option<Error> {
        match self.diagnostic {
            PragmaDiagnostic::Deprecated { msg, note } => Some(
                Error::new(file, pragma.location, Rule::Quirk, msg, note)
                    .with_doc_url("https://www.sqlite.org/pragma.html"),
            ),
            PragmaDiagnostic::Check(check) => check(file, pragma),
        }
    }
}

const PRAGMAS: &[PragmaEntry] = &[
    PragmaEntry {
        name: "case_sensitive_like",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA case_sensitive_like is deprecated",
            note: "SQLite documents case_sensitive_like as deprecated. Avoid new use because changing LIKE semantics can make existing schema objects appear corrupt until the setting is restored or indexes are rebuilt.",
        },
    },
    PragmaEntry {
        name: "count_changes",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA count_changes is deprecated",
            note: "SQLite documents count_changes as deprecated. Avoid new use; sqlite3_changes() and sqlite3_total_changes() are the supported interfaces.",
        },
    },
    PragmaEntry {
        name: "data_store_directory",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA data_store_directory is deprecated",
            note: "SQLite documents data_store_directory as deprecated and not threadsafe. Avoid changing process-global SQLite directory state from SQL.",
        },
    },
    PragmaEntry {
        name: "default_cache_size",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA default_cache_size is deprecated",
            note: "SQLite documents default_cache_size as deprecated. Prefer PRAGMA cache_size for connection-local cache tuning.",
        },
    },
    PragmaEntry {
        name: "empty_result_callbacks",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA empty_result_callbacks is deprecated",
            note: "SQLite documents empty_result_callbacks as deprecated. Avoid new use.",
        },
    },
    PragmaEntry {
        name: "full_column_names",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA full_column_names is deprecated",
            note: "SQLite documents full_column_names as deprecated. Avoid relying on deprecated result-column naming controls.",
        },
    },
    PragmaEntry {
        name: "short_column_names",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA short_column_names is deprecated",
            note: "SQLite documents short_column_names as deprecated. Avoid relying on deprecated result-column naming controls.",
        },
    },
    PragmaEntry {
        name: "temp_store_directory",
        diagnostic: PragmaDiagnostic::Deprecated {
            msg: "PRAGMA temp_store_directory is deprecated",
            note: "SQLite documents temp_store_directory as deprecated and not threadsafe. Avoid changing process-global SQLite directory state from SQL.",
        },
    },
    PragmaEntry {
        name: "foreign_keys",
        diagnostic: PragmaDiagnostic::Check(foreign_keys),
    },
    PragmaEntry {
        name: "ignore_check_constraints",
        diagnostic: PragmaDiagnostic::Check(ignore_check_constraints),
    },
    PragmaEntry {
        name: "writable_schema",
        diagnostic: PragmaDiagnostic::Check(writable_schema),
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
        analyse::pragma::pragma,
        parser::nodes::{Pragma, PragmaInvocation, SchemaTableContainer},
        types::{Token, Type, rules::Rule},
    };

    fn pragma_node(name: &str, invocation: PragmaInvocation) -> Pragma {
        Pragma::new(SchemaTableContainer::Table(name.into()), invocation)
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
