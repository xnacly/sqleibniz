use crate::{
    error::Error,
    parser::nodes::{Pragma, PragmaInvocation, SchemaTableContainer},
    types::{Token, Type, rules::Rule},
};

pub fn pragma(file: &str, pragma: &Pragma) -> Vec<Error> {
    let name = pragma_name(pragma);

    let Some(entry) = PRAGMAS.iter().find(|entry| entry.name == name) else {
        return vec![
            Error::new(
                file,
                pragma.location,
                Rule::UnknownPragma,
                format!("Unknown SQLite PRAGMA `{name}`"),
                "SQLite ignores unknown PRAGMAs. If this is an extension-defined PRAGMA, disable sqlite/unknown-pragma for this file or project.",
            )
            .with_doc_url("https://www.sqlite.org/pragma.html"),
        ];
    };

    let mut diagnostics = Vec::new();
    if let Some(error) = entry.validate(file, pragma) {
        diagnostics.push(error);
        return diagnostics;
    }

    diagnostics.extend(entry.analyse(file, pragma));
    diagnostics
}

struct PragmaEntry {
    name: &'static str,
    doc_url: &'static str,
    /// Documented invocation forms for a known SQLite PRAGMA.
    ///
    /// These forms describe the public syntax from https://www.sqlite.org/pragma.html, not the
    /// generic parser grammar. Names missing from the known-PRAGMA table are reported with
    /// `sqlite/unknown-pragma`.
    forms: PragmaForms,
    /// Value contract for assignment/call forms.
    ///
    /// This is intentionally token-level validation. It checks documented value categories and
    /// option sets, but it does not try to prove runtime constraints such as page-size ranges,
    /// build-option availability, or database-state-dependent behaviour.
    value: PragmaValue,
    /// Optional semantic diagnostics after the documented form/value check has passed.
    analysis: PragmaAnalysis,
}

#[derive(Clone, Copy)]
struct PragmaForms(u8);

impl PragmaForms {
    const QUERY: u8 = 1;
    const ASSIGN: u8 = 1 << 1;
    const CALL: u8 = 1 << 2;

    const fn new(query: bool, assign: bool, call: bool) -> Self {
        Self(
            (query as u8 * Self::QUERY) | (assign as u8 * Self::ASSIGN) | (call as u8 * Self::CALL),
        )
    }

    fn allows_query(self) -> bool {
        self.0 & Self::QUERY != 0
    }

    fn allows_assign(self) -> bool {
        self.0 & Self::ASSIGN != 0
    }

    fn allows_call(self) -> bool {
        self.0 & Self::CALL != 0
    }

    fn description(self) -> &'static str {
        match (
            self.allows_query(),
            self.allows_assign(),
            self.allows_call(),
        ) {
            (true, false, false) => "query form",
            (false, true, false) => "assignment form",
            (false, false, true) => "call form",
            (true, true, false) => "query or assignment form",
            (true, false, true) => "query or call form",
            (false, true, true) => "assignment or call form",
            (true, true, true) => "query, assignment or call form",
            (false, false, false) => "no invocation form",
        }
    }
}

#[derive(Clone, Copy)]
enum PragmaValue {
    /// Query-only pragmas. Assignment/call forms using this value spec should be rejected by
    /// `PragmaForms` before value validation runs.
    ///
    /// Example: `PRAGMA database_list;` has no value.
    None,
    /// SQLite boolean values: `0`, `1`, `on`, `off`, `yes`, `no`, `true`, `false`.
    ///
    /// Example: `PRAGMA foreign_keys = ON;` or `PRAGMA trusted_schema = false;`.
    Boolean,
    /// Numeric literals with no fractional component.
    ///
    /// Example: `PRAGMA cache_size = 2000;`.
    Integer,
    /// Table/index/schema-like names represented by identifiers, strings, or SQLite keywords.
    ///
    /// Example: `PRAGMA table_info(users);` or `PRAGMA index_info('idx_users_email');`.
    Name,
    /// Text-like values represented by string literals or identifiers.
    ///
    /// Example: `PRAGMA encoding = 'UTF-8';`.
    Text,
    /// Boolean values or integer numeric literals.
    ///
    /// Example: `PRAGMA cache_spill = OFF;` or `PRAGMA cache_spill = 1000;`.
    BooleanOrInteger,
    /// Boolean values or one of a fixed set of documented named options.
    ///
    /// Example: `PRAGMA secure_delete = true;` or `PRAGMA secure_delete = FAST;`.
    BooleanOrNamed(&'static [&'static str]),
    /// Integer numeric literals or one of a fixed set of documented named options.
    ///
    /// Example: `PRAGMA synchronous = 2;` or `PRAGMA synchronous = NORMAL;`.
    IntegerOrNamed(&'static [&'static str]),
    /// Integer numeric literals or table/index/schema-like names.
    ///
    /// Example: `PRAGMA integrity_check(10);` or `PRAGMA integrity_check(users);`.
    IntegerOrName,
    /// One of a fixed set of documented named options.
    ///
    /// Example: `PRAGMA wal_checkpoint(TRUNCATE);`.
    Named(&'static [&'static str]),
}

impl PragmaValue {
    fn accepts(self, token: &Token) -> bool {
        match self {
            Self::None => false,
            Self::Boolean => token.is_pragma_boolean(),
            Self::Integer => is_integer_literal(token),
            Self::Name => is_name_token(token),
            Self::Text => is_name_token(token),
            Self::BooleanOrInteger => token.is_pragma_boolean() || is_integer_literal(token),
            Self::BooleanOrNamed(options) => {
                token.is_pragma_boolean() || is_named_option(token, options)
            }
            Self::IntegerOrNamed(options) => {
                is_integer_literal(token) || is_named_option(token, options)
            }
            Self::IntegerOrName => is_integer_literal(token) || is_name_token(token),
            Self::Named(options) => is_named_option(token, options),
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::None => "no value",
            Self::Boolean => "a boolean value",
            Self::Integer => "an integer value",
            Self::Name => "an identifier or string name",
            Self::Text => "a string or identifier value",
            Self::BooleanOrInteger => "a boolean or integer value",
            Self::BooleanOrNamed(_) => "a boolean value or documented named option",
            Self::IntegerOrNamed(_) => "an integer value or documented named option",
            Self::IntegerOrName => "an integer value, identifier or string name",
            Self::Named(_) => "a documented named option",
        }
    }
}

enum PragmaAnalysis {
    /// No semantic lint beyond documented form/value validation.
    None,
    /// Unconditional warning for PRAGMAs SQLite documents as deprecated.
    Deprecated {
        msg: &'static str,
        note: &'static str,
    },
    /// Value-sensitive semantic diagnostic, such as warning only when a setting is enabled.
    Check {
        diagnostic: fn(&str, &'static str, &Pragma) -> Option<Error>,
        #[cfg(test)]
        diagnostic_value: Type,
    },
}

impl PragmaEntry {
    fn validate(&self, file: &str, pragma: &Pragma) -> Option<Error> {
        match &pragma.invocation {
            PragmaInvocation::Query if !self.forms.allows_query() => {
                Some(self.bad_form(file, pragma))
            }
            PragmaInvocation::Assign { value } if !self.forms.allows_assign() => {
                let _ = value;
                Some(self.bad_form(file, pragma))
            }
            PragmaInvocation::Call { value } if !self.forms.allows_call() => {
                let _ = value;
                Some(self.bad_form(file, pragma))
            }
            PragmaInvocation::Assign { value } | PragmaInvocation::Call { value }
                if !self.value.accepts(value) =>
            {
                Some(self.bad_value(file, pragma, value))
            }
            PragmaInvocation::Query => None,
            PragmaInvocation::Assign { .. } | PragmaInvocation::Call { .. } => None,
        }
    }

    fn analyse(&self, file: &str, pragma: &Pragma) -> Option<Error> {
        match self.analysis {
            PragmaAnalysis::None => None,
            PragmaAnalysis::Deprecated { msg, note } => Some(
                Error::new(file, pragma.location, Rule::Quirk, msg, note)
                    .with_doc_url(self.doc_url),
            ),
            PragmaAnalysis::Check { diagnostic, .. } => diagnostic(file, self.doc_url, pragma),
        }
    }

    fn bad_form(&self, file: &str, pragma: &Pragma) -> Error {
        Error::new(
            file,
            pragma.location,
            Rule::Syntax,
            format!("PRAGMA {} uses an unsupported invocation form", self.name),
            format!(
                "SQLite documents PRAGMA {} with {}; this invocation does not match that form.",
                self.name,
                self.forms.description()
            ),
        )
        .with_doc_url(self.doc_url)
    }

    fn bad_value(&self, file: &str, pragma: &Pragma, value: &Token) -> Error {
        Error::new(
            file,
            pragma.location,
            Rule::Syntax,
            format!("PRAGMA {} has an unsupported value", self.name),
            format!(
                "SQLite documents PRAGMA {} with {}; got {:?}.",
                self.name,
                self.value.description(),
                value.ttype
            ),
        )
        .with_doc_url(self.doc_url)
    }
}

macro_rules! forms {
    // `PRAGMA name;`
    (query) => {
        PragmaForms::new(true, false, false)
    };
    // `PRAGMA name(value);`
    (call) => {
        PragmaForms::new(false, false, true)
    };
    // `PRAGMA name;` and `PRAGMA name = value;`
    (query, assign) => {
        PragmaForms::new(true, true, false)
    };
    // `PRAGMA name;` and `PRAGMA name(value);`
    (query, call) => {
        PragmaForms::new(true, false, true)
    };
    // `PRAGMA name;`, `PRAGMA name = value;` and `PRAGMA name(value);`
    (query, assign, call) => {
        PragmaForms::new(true, true, true)
    };
}

/// Defines one known SQLite PRAGMA validation spec.
///
/// The first argument is the canonical SQLite PRAGMA name without an optional schema prefix. The
/// macro derives the docs URL as `https://www.sqlite.org/pragma.html#pragma_<name>`.
///
/// The second argument is the documented invocation shape, expressed with `forms!`:
///
/// - `forms!(query)` accepts `PRAGMA name;`
/// - `forms!(query, assign)` accepts `PRAGMA name;` and `PRAGMA name = value;`
/// - `forms!(query, call)` accepts `PRAGMA name;` and `PRAGMA name(value);`
/// - `forms!(call)` accepts only `PRAGMA name(value);`
///
/// The third argument is the `PragmaValue` contract for assignment/call values. It is ignored for
/// query invocations. Additional `deprecated(...)` and `check(...)` clauses add semantic diagnostics
/// after form/value validation passes.
macro_rules! pragma_entry {
    ($name:literal, $forms:expr, $value:expr) => {
        PragmaEntry {
            name: $name,
            doc_url: concat!("https://www.sqlite.org/pragma.html#pragma_", $name),
            forms: $forms,
            value: $value,
            analysis: PragmaAnalysis::None,
        }
    };
    ($name:literal, $forms:expr, $value:expr, deprecated($msg:literal, $note:literal)) => {
        PragmaEntry {
            name: $name,
            doc_url: concat!("https://www.sqlite.org/pragma.html#pragma_", $name),
            forms: $forms,
            value: $value,
            analysis: PragmaAnalysis::Deprecated {
                msg: $msg,
                note: $note,
            },
        }
    };
    ($name:literal, $forms:expr, $value:expr, check($diagnostic:path, $diagnostic_value:expr)) => {
        PragmaEntry {
            name: $name,
            doc_url: concat!("https://www.sqlite.org/pragma.html#pragma_", $name),
            forms: $forms,
            value: $value,
            analysis: PragmaAnalysis::Check {
                diagnostic: $diagnostic,
                #[cfg(test)]
                diagnostic_value: $diagnostic_value,
            },
        }
    };
}

const PRAGMAS: &[PragmaEntry] = &[
    pragma_entry!(
        "analysis_limit",
        forms!(query, assign, call),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "application_id",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "auto_vacuum",
        forms!(query, assign),
        PragmaValue::IntegerOrNamed(&["0", "1", "2", "full", "incremental", "none"])
    ),
    pragma_entry!(
        "automatic_index",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!("busy_timeout", forms!(query, assign), PragmaValue::Integer),
    pragma_entry!("cache_size", forms!(query, assign), PragmaValue::Integer),
    pragma_entry!(
        "cache_spill",
        forms!(query, assign),
        PragmaValue::BooleanOrInteger
    ),
    pragma_entry!(
        "case_sensitive_like",
        forms!(query, assign),
        PragmaValue::Boolean,
        deprecated(
            "PRAGMA case_sensitive_like is deprecated",
            "SQLite documents case_sensitive_like as deprecated. Avoid new use because changing LIKE semantics can make existing schema objects appear corrupt until the setting is restored or indexes are rebuilt."
        )
    ),
    pragma_entry!(
        "cell_size_check",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!(
        "checkpoint_fullfsync",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!("collation_list", forms!(query), PragmaValue::None),
    pragma_entry!("compile_options", forms!(query), PragmaValue::None),
    pragma_entry!(
        "count_changes",
        forms!(query, assign),
        PragmaValue::Boolean,
        deprecated(
            "PRAGMA count_changes is deprecated",
            "SQLite documents count_changes as deprecated. Avoid new use; sqlite3_changes() and sqlite3_total_changes() are the supported interfaces."
        )
    ),
    pragma_entry!(
        "data_store_directory",
        forms!(query, assign),
        PragmaValue::Text,
        deprecated(
            "PRAGMA data_store_directory is deprecated",
            "SQLite documents data_store_directory as deprecated and not threadsafe. Avoid changing process-global SQLite directory state from SQL."
        )
    ),
    pragma_entry!("data_version", forms!(query), PragmaValue::None),
    pragma_entry!("database_list", forms!(query), PragmaValue::None),
    pragma_entry!(
        "default_cache_size",
        forms!(query, assign),
        PragmaValue::Integer,
        deprecated(
            "PRAGMA default_cache_size is deprecated",
            "SQLite documents default_cache_size as deprecated. Prefer PRAGMA cache_size for connection-local cache tuning."
        )
    ),
    pragma_entry!(
        "defer_foreign_keys",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!(
        "empty_result_callbacks",
        forms!(query, assign),
        PragmaValue::Boolean,
        deprecated(
            "PRAGMA empty_result_callbacks is deprecated",
            "SQLite documents empty_result_callbacks as deprecated. Avoid new use."
        )
    ),
    pragma_entry!("encoding", forms!(query, assign), PragmaValue::Text),
    pragma_entry!("foreign_key_check", forms!(query, call), PragmaValue::Name),
    pragma_entry!("foreign_key_list", forms!(call), PragmaValue::Name),
    pragma_entry!(
        "foreign_keys",
        forms!(query, assign),
        PragmaValue::Boolean,
        check(foreign_keys, Type::Boolean(false))
    ),
    pragma_entry!("freelist_count", forms!(query), PragmaValue::None),
    pragma_entry!(
        "full_column_names",
        forms!(query, assign),
        PragmaValue::Boolean,
        deprecated(
            "PRAGMA full_column_names is deprecated",
            "SQLite documents full_column_names as deprecated. Avoid relying on deprecated result-column naming controls."
        )
    ),
    pragma_entry!("fullfsync", forms!(query, assign), PragmaValue::Boolean),
    pragma_entry!("function_list", forms!(query), PragmaValue::None),
    pragma_entry!(
        "hard_heap_limit",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "ignore_check_constraints",
        forms!(query, assign),
        PragmaValue::Boolean,
        check(ignore_check_constraints, Type::Boolean(true))
    ),
    pragma_entry!(
        "incremental_vacuum",
        forms!(query, call),
        PragmaValue::Integer
    ),
    pragma_entry!("index_info", forms!(call), PragmaValue::Name),
    pragma_entry!("index_list", forms!(call), PragmaValue::Name),
    pragma_entry!("index_xinfo", forms!(call), PragmaValue::Name),
    pragma_entry!(
        "integrity_check",
        forms!(query, call),
        PragmaValue::IntegerOrName
    ),
    pragma_entry!(
        "journal_mode",
        forms!(query, assign),
        PragmaValue::Named(&["delete", "memory", "off", "persist", "truncate", "wal"])
    ),
    pragma_entry!(
        "journal_size_limit",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "legacy_alter_table",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!(
        "locking_mode",
        forms!(query, assign),
        PragmaValue::Named(&["exclusive", "normal"])
    ),
    pragma_entry!(
        "max_page_count",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!("mmap_size", forms!(query, assign), PragmaValue::Integer),
    pragma_entry!("module_list", forms!(query), PragmaValue::None),
    pragma_entry!("optimize", forms!(query, call), PragmaValue::Integer),
    pragma_entry!("page_count", forms!(query), PragmaValue::None),
    pragma_entry!("page_size", forms!(query, assign), PragmaValue::Integer),
    pragma_entry!("parser_trace", forms!(query, assign), PragmaValue::Boolean),
    pragma_entry!("pragma_list", forms!(query), PragmaValue::None),
    pragma_entry!("query_only", forms!(query, assign), PragmaValue::Boolean),
    pragma_entry!(
        "quick_check",
        forms!(query, call),
        PragmaValue::IntegerOrName
    ),
    pragma_entry!(
        "read_uncommitted",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!(
        "recursive_triggers",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!(
        "reverse_unordered_selects",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!(
        "schema_version",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "secure_delete",
        forms!(query, assign),
        PragmaValue::BooleanOrNamed(&["fast"])
    ),
    pragma_entry!(
        "short_column_names",
        forms!(query, assign),
        PragmaValue::Boolean,
        deprecated(
            "PRAGMA short_column_names is deprecated",
            "SQLite documents short_column_names as deprecated. Avoid relying on deprecated result-column naming controls."
        )
    ),
    pragma_entry!("shrink_memory", forms!(query), PragmaValue::None),
    pragma_entry!(
        "soft_heap_limit",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "synchronous",
        forms!(query, assign),
        PragmaValue::IntegerOrNamed(&["extra", "full", "normal", "off"])
    ),
    pragma_entry!("table_info", forms!(call), PragmaValue::Name),
    pragma_entry!("table_list", forms!(query, call), PragmaValue::Name),
    pragma_entry!("table_xinfo", forms!(call), PragmaValue::Name),
    pragma_entry!(
        "temp_store",
        forms!(query, assign),
        PragmaValue::IntegerOrNamed(&["default", "file", "memory"])
    ),
    pragma_entry!(
        "temp_store_directory",
        forms!(query, assign),
        PragmaValue::Text,
        deprecated(
            "PRAGMA temp_store_directory is deprecated",
            "SQLite documents temp_store_directory as deprecated and not threadsafe. Avoid changing process-global SQLite directory state from SQL."
        )
    ),
    pragma_entry!("threads", forms!(query, assign), PragmaValue::Integer),
    pragma_entry!(
        "trusted_schema",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!("user_version", forms!(query, assign), PragmaValue::Integer),
    pragma_entry!(
        "vdbe_addoptrace",
        forms!(query, assign),
        PragmaValue::Boolean
    ),
    pragma_entry!("vdbe_debug", forms!(query, assign), PragmaValue::Boolean),
    pragma_entry!("vdbe_listing", forms!(query, assign), PragmaValue::Boolean),
    pragma_entry!("vdbe_trace", forms!(query, assign), PragmaValue::Boolean),
    pragma_entry!(
        "wal_autocheckpoint",
        forms!(query, assign),
        PragmaValue::Integer
    ),
    pragma_entry!(
        "wal_checkpoint",
        forms!(query, call),
        PragmaValue::Named(&["full", "passive", "restart", "truncate"])
    ),
    pragma_entry!(
        "writable_schema",
        forms!(query, assign),
        PragmaValue::Boolean,
        check(writable_schema, Type::Boolean(true))
    ),
];

fn foreign_keys(file: &str, doc_url: &'static str, pragma: &Pragma) -> Option<Error> {
    if !invocation_value_matches(pragma, |token| token.pragma_boolean() == Some(false)) {
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
        .with_doc_url(doc_url),
    )
}

fn ignore_check_constraints(file: &str, doc_url: &'static str, pragma: &Pragma) -> Option<Error> {
    if !invocation_value_matches(pragma, |token| token.pragma_boolean() == Some(true)) {
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
        .with_doc_url(doc_url),
    )
}

fn writable_schema(file: &str, doc_url: &'static str, pragma: &Pragma) -> Option<Error> {
    if !invocation_value_matches(pragma, |token| token.pragma_boolean() == Some(true)) {
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
        .with_doc_url(doc_url),
    )
}

fn pragma_name(pragma: &Pragma) -> String {
    match &pragma.name {
        SchemaTableContainer::SchemaAndTable { table, .. } | SchemaTableContainer::Table(table) => {
            table.to_ascii_lowercase()
        }
    }
}

fn is_integer_literal(token: &Token) -> bool {
    matches!(token.ttype, Type::Number(number) if number.fract() == 0.0)
}

fn is_name_token(token: &Token) -> bool {
    matches!(
        token.ttype,
        Type::Ident(_) | Type::String(_) | Type::Keyword(_)
    )
}

fn is_named_option(token: &Token, options: &[&str]) -> bool {
    let value = match &token.ttype {
        Type::Ident(value) | Type::String(value) => value.as_str(),
        Type::Keyword(keyword) => (*keyword).into(),
        _ => return false,
    };

    options
        .iter()
        .any(|option| option.eq_ignore_ascii_case(value))
}

fn invocation_value_matches(pragma: &Pragma, predicate: impl FnOnce(&Token) -> bool) -> bool {
    let value = match &pragma.invocation {
        PragmaInvocation::Assign { value } | PragmaInvocation::Call { value } => value,
        PragmaInvocation::Query => return false,
    };

    predicate(value)
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
    fn pragma_table_entries_have_specific_doc_urls() {
        for entry in PRAGMAS {
            assert_eq!(
                entry.doc_url,
                format!("https://www.sqlite.org/pragma.html#pragma_{}", entry.name)
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
            Some("https://www.sqlite.org/pragma.html#pragma_case_sensitive_like")
        );
    }

    #[test]
    fn reports_unsupported_invocation_forms() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node(
                "foreign_keys",
                PragmaInvocation::Call {
                    value: Token::new(Type::Boolean(true)),
                },
            ),
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::Syntax);
        assert!(diagnostics[0].note.contains("query or assignment form"));
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/pragma.html#pragma_foreign_keys")
        );
    }

    #[test]
    fn reports_unsupported_values() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node(
                "foreign_keys",
                PragmaInvocation::Assign {
                    value: Token::new(Type::Ident("maybe".into())),
                },
            ),
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::Syntax);
        assert!(diagnostics[0].note.contains("a boolean value"));
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/pragma.html#pragma_foreign_keys")
        );
    }

    #[test]
    fn reports_unknown_pragmas() {
        let diagnostics = pragma(
            "test.sql",
            &pragma_node("some_extension_pragma", PragmaInvocation::Query),
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, Rule::UnknownPragma);
        assert!(
            diagnostics[0]
                .note
                .contains("SQLite ignores unknown PRAGMAs")
        );
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/pragma.html")
        );
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
                PragmaInvocation::Assign {
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
