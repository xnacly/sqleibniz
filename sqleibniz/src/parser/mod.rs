use nodes::{BindParameter, SchemaTableContainer};

#[cfg(feature = "trace")]
use sqleibniz_proc::trace;

use crate::{
    error::{Error, ImprovedLine, Location},
    parser::nodes::{
        ColumnConstraint, ForeignKeyAction, ForeignKeyClause, ForeignKeyMatch, Pragma,
    },
    types::{Keyword, Token, Type, rules::Rule, storage::SqliteStorageClass},
};

/// implement serialisation manually for all nodes and contained types
pub mod debug;
/// nodes holds all abstract syntax tree nodes, the node! macro, the lua preparation for the plugin execution and the sqleibniz analysis
pub mod nodes;
mod tests;

// this sucks but is necessary to track the call depth for indentation when printing the parser
// stack
#[cfg(feature = "trace")]
thread_local! {
    static CALL_DEPTH: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

pub struct Parser<'a> {
    pos: usize,
    tokens: Vec<Token>,
    name: &'a str,
    pub errors: Vec<Error>,
}

enum ExprOperator {
    Postfix {
        operator: String,
        left_bp: u8,
        advance: usize,
    },
    Infix {
        operator: String,
        left_bp: u8,
        right_bp: u8,
        advance: usize,
    },
    Between {
        operator: String,
        left_bp: u8,
        right_bp: u8,
        advance: usize,
    },
    InList {
        operator: String,
        left_bp: u8,
        advance: usize,
    },
}

impl ExprOperator {
    fn postfix(operator: &str, left_bp: u8) -> Self {
        Self::postfix_advanced(operator, left_bp, 1)
    }

    fn postfix_advanced(operator: &str, left_bp: u8, advance: usize) -> Self {
        Self::Postfix {
            operator: operator.into(),
            left_bp,
            advance,
        }
    }

    fn infix(operator: &str, left_bp: u8, right_bp: u8) -> Self {
        Self::infix_advanced(operator, left_bp, right_bp, 1)
    }

    fn infix_advanced(operator: &str, left_bp: u8, right_bp: u8, advance: usize) -> Self {
        Self::Infix {
            operator: operator.into(),
            left_bp,
            right_bp,
            advance,
        }
    }

    fn between(operator: &str, left_bp: u8, right_bp: u8, advance: usize) -> Self {
        Self::Between {
            operator: operator.into(),
            left_bp,
            right_bp,
            advance,
        }
    }

    fn in_list(operator: &str, left_bp: u8, advance: usize) -> Self {
        Self::InList {
            operator: operator.into(),
            left_bp,
            advance,
        }
    }
}

fn prefix_binding_power(operator: &str) -> u8 {
    match operator {
        "NOT" => 4,
        "+" | "-" | "~" => 16,
        _ => unreachable!("prefix operator is controlled by prefix_operator"),
    }
}

/// wrap argument in Some(Box::new(_))
macro_rules! some_box {
    ($expr:expr) => {
        Some(Box::new($expr) as Box<dyn nodes::Node>)
    };
}

const TRANSACTION_DOC: &str = "https://www.sqlite.org/lang_transaction.html";
const FOREIGN_KEY_DOC: &str = "https://www.sqlite.org/syntax/foreign-key-clause.html";

/// Recursive-descent parser for the SQLite syntax diagrams.
///
/// The top-level flow mirrors SQLite's statement grammar:
///
/// - [`Parser::sql_stmt_list`] parses a sequence of semicolon-terminated statements.
/// - [`Parser::sql_stmt_prefix`] handles optional statement prefixes such as `EXPLAIN`.
/// - [`Parser::sql_stmt`] dispatches to the concrete `<name>_stmt` parser.
///
/// Concrete statement parser names intentionally follow SQLite documentation names where possible.
///
/// ## See:
///
/// - https://www.sqlite.org/lang.html
/// - https://www.sqlite.org/lang_expr.html
impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, name: &'a str) -> Parser<'a> {
        Parser {
            pos: 0,
            name,
            tokens,
            errors: vec![],
        }
    }

    fn cur(&self) -> &Token {
        if let Some(tok) = self.tokens.get(self.pos) {
            tok
        } else {
            &Token {
                ttype: Type::Eof,
                start: 0,
                end: 0,
                line: 0,
            }
        }
    }

    fn err(
        &self,
        msg: impl Into<String>,
        note: &str,
        src: impl Into<Location>,
        rule: Rule,
    ) -> Error {
        Error::new(self.name, src, rule, msg, note)
    }

    fn push_err(
        &mut self,
        msg: impl Into<String>,
        note: &str,
        src: impl Into<Location>,
        rule: Rule,
    ) {
        let err = self.err(msg, note, src, rule);
        self.errors.push(err);
    }

    fn push_doc_err(
        &mut self,
        msg: impl Into<String>,
        note: impl Into<String>,
        src: impl Into<Location>,
        rule: Rule,
        doc: &'static str,
    ) {
        let note = note.into();
        self.errors
            .push(self.err(msg, &note, src, rule).with_doc_url(doc));
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn advance(&mut self) {
        if !self.is_eof() {
            self.pos += 1
        }
    }

    fn is(&mut self, t: Type) -> bool {
        self.cur().ttype == t
    }

    fn is_keyword(&mut self, keyword: Keyword) -> bool {
        self.cur().ttype == Type::Keyword(keyword)
    }

    fn skip_until_semicolon_or_eof(&mut self) {
        while !self.is_eof() && !self.is(Type::Semicolon) {
            self.advance();
        }
    }

    /// checks if type of current token is equal to t, otherwise pushs an error, advances either way
    fn consume(&mut self, t: Type) {
        let tt = t.clone();
        if !self.is(tt) {
            let cur = self.cur().clone();
            let mut err = self.err(
                match cur.ttype {
                    Type::Eof => "Unexpected End of input",
                    _ => "Unexpected Token",
                },
                &format!("Wanted {:?}, got {:?}", t, cur.ttype),
                &cur,
                Rule::Syntax,
            );
            if t == Type::Semicolon {
                err.msg = "Missing semicolon".into();
                err.note.push_str(", terminate statements with ';'");
                err.rule = Rule::Semicolon;
                err.improved_line = Some(ImprovedLine {
                    snippet: ";",
                    start: self.cur().end,
                });
            }
            err.doc_url = Some("https://www.sqlite.org/syntax/sql-stmt.html");
            self.errors.push(err);
        }
        self.advance(); // we advance either way to keep the parser error resistant
    }

    fn consume_keyword(&mut self, keyword: Keyword) {
        self.consume(Type::Keyword(keyword));
    }

    fn consume_if(&mut self, t: Type) -> bool {
        if self.is(t) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_if_keyword(&mut self, keyword: Keyword) -> bool {
        if self.is_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn next_is(&self, t: Type) -> bool {
        self.tokens
            .get(self.pos + 1)
            .is_some_and(|tok| tok.ttype == t)
    }

    fn nth_is(&self, offset: usize, t: Type) -> bool {
        self.tokens
            .get(self.pos + offset)
            .is_some_and(|tok| tok.ttype == t)
    }

    /// checks if current token is semicolon, if not pushes Rule::Syntax
    fn expect_end(&mut self, doc: &'static str) -> Option<()> {
        if !self.is(Type::Semicolon) {
            let cur = self.cur().clone();
            let mut err = self.err(
                "Unexpected Statement Continuation",
                &format!("Expected statement end via Semicolon, got {:?}", cur.ttype),
                &cur,
                Rule::Syntax,
            );
            if !doc.is_empty() {
                err.doc_url = Some(doc);
            }
            self.errors.push(err);
            self.advance();
        }
        None
    }

    fn consume_ident(
        &mut self,
        doc: &'static str,
        expected_ident_name: &'static str,
    ) -> Option<String> {
        if let Type::Ident(ident) = &self.cur().ttype {
            let i = ident.to_string();
            self.advance();
            Some(i)
        } else {
            let cur = self.cur().clone();
            let mut err = self.err(
                "Unexpected Token",
                &format!(
                    "Expected Ident(<{}>), got {:?}",
                    expected_ident_name, cur.ttype
                ),
                &cur,
                Rule::Syntax,
            );
            err.doc_url = Some(doc);
            self.errors.push(err);
            self.advance();
            None
        }
    }

    // <signed-number|name|signed-literal>
    #[cfg_attr(feature = "trace", trace)]
    fn consume_pragma_value(&mut self, invocation_kind: &str) -> Token {
        // PRAGMA values are intentionally token-level here. Known PRAGMA-specific form and value
        // validation happens later in the analyser.
        let is_pragma_value = matches!(
            self.cur().ttype,
            Type::String(_)
                | Type::Number(_)
                | Type::Boolean(_)
                | Type::Ident(_)
                | Type::Keyword(_)
        );

        if !is_pragma_value {
            let cur = self.cur().clone();
            self.push_err(
                "Bad pragma value",
                &format!(
                    "A pragma {invocation_kind} value has to be either String, Number, Boolean, Ident or a Keyword, got {:?} instead",
                    cur.ttype
                ),
                &cur,
                Rule::Syntax,
            );
        }

        let value = self.cur().clone();
        self.advance();
        value
    }

    #[cfg_attr(feature = "trace", trace)]
    pub fn parse(&mut self) -> Vec<Box<dyn nodes::Node>> {
        self.sql_stmt_list()
    }

    /// Parses `sql-stmt-list`.
    ///
    /// This is the parser entry point after lexing. It keeps parsing until EOF and consumes the
    /// statement terminator after each statement parser returns. `@sqleibniz::expect` instructions
    /// are handled here because they suppress the next complete statement rather than a single
    /// syntax production.
    ///
    /// See: https://www.sqlite.org/syntax/sql-stmt-list.html
    #[cfg_attr(feature = "trace", trace)]
    fn sql_stmt_list(&mut self) -> Vec<Box<dyn nodes::Node>> {
        let mut r = vec![];
        while !self.is_eof() {
            if let Token {
                ttype: Type::InstructionExpect,
                ..
            } = self.cur()
            {
                // skip all token until the statement ends
                self.skip_until_semicolon_or_eof();
                // only consume ; if we arent at an eof, otherwise we want the last comment of a
                // file to end with a ; which doesnt make sense
                if !self.is_eof() {
                    // skip ';'
                    self.consume(Type::Semicolon);
                    continue;
                }
            }
            if let Some(stmt) = self.sql_stmt_prefix() {
                r.push(stmt);
            }
            self.consume(Type::Semicolon);
        }
        r
    }

    /// Parses an optional statement prefix.
    ///
    /// SQLite currently supports `EXPLAIN` and `EXPLAIN QUERY PLAN` before a regular statement.
    /// The wrapped statement is parsed by [`Parser::sql_stmt`].
    #[cfg_attr(feature = "trace", trace)]
    fn sql_stmt_prefix(&mut self) -> Option<Box<dyn nodes::Node>> {
        let r: Option<Box<dyn nodes::Node>> = match self.cur().ttype {
            Type::Keyword(Keyword::EXPLAIN) => {
                let location = Location::from(self.cur());
                // skip EXPLAIN
                self.advance();

                // path for EXPLAIN->QUERY->PLAN
                if self.is(Type::Keyword(Keyword::QUERY)) {
                    self.advance();
                    self.consume(Type::Keyword(Keyword::PLAN));
                }

                // else path is EXPLAIN->*_stmt
                some_box!(nodes::Explain {
                    location,
                    child: self.sql_stmt()?,
                })
            }
            _ => self.sql_stmt(),
        };

        r
    }

    /// Dispatches to a concrete statement parser based on the first token.
    ///
    /// Unsupported statement keywords are reported as `sqleibniz/unimplemented` diagnostics. This
    /// keeps the unsupported surface explicit without adding parser stubs that panic at runtime.
    ///
    /// See: https://www.sqlite.org/syntax/sql-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn sql_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        match self.cur().ttype {
            Type::Keyword(Keyword::PRAGMA) => self.pragma_stmt(),
            Type::Keyword(Keyword::ALTER) => self.alter_stmt(),
            Type::Keyword(Keyword::ATTACH) => self.attach_stmt(),
            Type::Keyword(Keyword::CREATE) => self.create_stmt(),
            Type::Keyword(Keyword::REINDEX) => self.reindex_stmt(),
            Type::Keyword(Keyword::RELEASE) => self.release_stmt(),
            Type::Keyword(Keyword::SAVEPOINT) => self.savepoint_stmt(),
            Type::Keyword(Keyword::DROP) => self.drop_stmt(),
            Type::Keyword(Keyword::ANALYZE) => self.analyze_stmt(),
            Type::Keyword(Keyword::DETACH) => self.detach_stmt(),
            Type::Keyword(Keyword::ROLLBACK) => self.rollback_stmt(),
            Type::Keyword(Keyword::COMMIT) | Type::Keyword(Keyword::END) => self.commit_stmt(),
            Type::Keyword(Keyword::BEGIN) => self.begin_stmt(),
            Type::Keyword(Keyword::VACUUM) => self.vacuum_stmt(),

            // statement should not start with a semicolon 󰚌
            Type::Semicolon => {
                self.push_err(
                    "Unexpected Token",
                    "Semicolon makes no sense at this point, Semicolons are used to terminate statements",
                    &self.cur().clone(),
                    Rule::Syntax,
                );
                self.advance();
                None
            }

            // explicitly disallowing literals at this point: results in clearer and more
            // understandable error messages
            Type::String(_)
            | Type::Number(_)
            | Type::Blob(_)
            | Type::Keyword(Keyword::NULL)
            | Type::Boolean(_)
            | Type::Keyword(Keyword::CURRENT_TIME)
            | Type::Keyword(Keyword::CURRENT_DATE)
            | Type::Keyword(Keyword::CURRENT_TIMESTAMP) => {
                let mut err = self.err(
                    "Unexpected Literal",
                    &format!("Literal {:?} can not start a statement", self.cur().ttype),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/syntax/sql-stmt.html");
                self.errors.push(err);
                self.advance();
                None
            }
            Type::Ident(ref name) => {
                let suggestions = Keyword::suggestions(name);
                if !suggestions.is_empty() {
                    let mut err = self.err(
                        "Unknown Keyword",
                        &format!(
                            "'{}' is not an SQL keyword, did you mean one of: {}",
                            name,
                            suggestions.join(", ").as_str()
                        ),
                        self.cur(),
                        Rule::UnknownKeyword,
                    );
                    err.doc_url = Some("https://sqlite.org/lang_keywords.html");
                    self.errors.push(err);
                } else {
                    self.push_err(
                        "Unknown Keyword",
                        &format!("'{name}' is not a keyword"),
                        &self.cur().clone(),
                        Rule::UnknownKeyword,
                    );
                };
                self.advance();
                None
            }
            Type::Keyword(_) => {
                let cur = self.cur().clone();
                self.push_err(
                    "Unimplemented",
                    &format!("sqleibniz can not yet analyse the token {:?}", cur.ttype,),
                    &cur,
                    Rule::Unimplemented,
                );
                self.advance();
                None
            }
            _ => {
                let cur = self.cur().clone();
                self.push_err(
                    "Unknown Token",
                    &format!(
                        "sqleibniz does not understand the token {:?}, skipping ahead to next statement",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Unimplemented,
                );
                self.skip_until_semicolon_or_eof();
                None
            }
        }
    }

    /// Dispatches SQLite `CREATE` statements that sqleibniz currently models.
    ///
    /// Supported AST-producing forms are the column-list form of `CREATE TABLE`, column-name
    /// `CREATE INDEX`, and structurally parsed `CREATE TRIGGER` statements. `CREATE VIEW` is
    /// recognized so unsupported select bodies can produce a precise diagnostic.
    ///
    /// Explicitly unsupported advanced forms are reported as `sqleibniz/unimplemented`:
    ///
    /// - `CREATE VIRTUAL TABLE ... USING ...`
    /// - `CREATE TABLE ... AS <select_stmt>`
    /// - expression indexes
    /// - partial indexes
    /// - `CREATE VIEW ... AS <select_stmt>`
    ///
    /// See: https://www.sqlite.org/lang_create.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let location = Location::from(self.cur());
        self.advance();

        let temporary =
            self.consume_if_keyword(Keyword::TEMP) || self.consume_if_keyword(Keyword::TEMPORARY);
        let unique = self.consume_if_keyword(Keyword::UNIQUE);

        if unique && !self.is_keyword(Keyword::INDEX) {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Unexpected Token",
                "CREATE UNIQUE is only valid for INDEX",
                src,
                Rule::Syntax,
                "https://www.sqlite.org/lang_createindex.html",
            );
            self.skip_until_semicolon_or_eof();
            return None;
        }

        match self.cur().ttype {
            // TABLE ...
            Type::Keyword(Keyword::TABLE) if !unique => self.create_table_stmt(location, temporary),
            // [UNIQUE] INDEX ...
            Type::Keyword(Keyword::INDEX) => self.create_index_stmt(location, temporary, unique),
            // VIEW ...
            Type::Keyword(Keyword::VIEW) if !unique => self.create_view_stmt(),
            // TRIGGER ...
            Type::Keyword(Keyword::TRIGGER) if !unique => {
                self.create_trigger_stmt(location, temporary)
            }
            // VIRTUAL TABLE ...
            Type::Keyword(Keyword::VIRTUAL) if !unique => {
                let src = Location::from(self.cur());
                self.push_doc_err(
                    "Unimplemented",
                    "CREATE VIRTUAL TABLE is not yet supported",
                    src,
                    Rule::Unimplemented,
                    "https://www.sqlite.org/lang_createvtab.html",
                );
                self.skip_until_semicolon_or_eof();
                None
            }
            _ => {
                let src = Location::from(self.cur());
                let note = format!(
                    "CREATE requires TABLE,INDEX,TRIGGER or VIEW at this point, got {:?}.",
                    self.cur().ttype
                );
                self.push_doc_err(
                    "Unexpected Token",
                    note,
                    src,
                    Rule::Syntax,
                    "https://www.sqlite.org/lang_create.html",
                );
                self.advance();
                None
            }
        }
    }

    /// https://www.sqlite.org/lang_createtable.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_table_stmt(
        &mut self,
        location: Location,
        temporary: bool,
    ) -> Option<Box<dyn nodes::Node>> {
        // TABLE
        self.consume_keyword(Keyword::TABLE);

        // IF NOT EXISTS
        let if_not_exists = if self.consume_if_keyword(Keyword::IF) {
            self.consume_keyword(Keyword::NOT);
            self.consume_keyword(Keyword::EXISTS);
            true
        } else {
            false
        };

        let name = self.schema_table_container(None)?;

        // AS <select-stmt>
        if self.is(Type::Keyword(Keyword::AS)) {
            let src = Location::from(self.cur());
            self.push_err(
                "Unimplemented",
                "CREATE TABLE ... AS <select_stmt> is not yet supported",
                src,
                Rule::Unimplemented,
            );
            self.skip_until_semicolon_or_eof();
            return None;
        }

        self.consume(Type::BraceLeft);

        let mut columns = vec![];
        let mut table_constraints = vec![];
        loop {
            if self.is(Type::BraceRight) {
                if columns.is_empty() && table_constraints.is_empty() {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed column list",
                        "CREATE TABLE requires at least one column definition",
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/lang_createtable.html",
                    );
                }
                break;
            }

            if self.starts_table_constraint() {
                // <table-constraint>
                table_constraints.push(self.table_constraint()?);
            } else {
                // <column-def>
                columns.push(self.column_def()?);
            }

            if self.is(Type::Comma) {
                self.advance();
                if self.is(Type::BraceRight) {
                    if self.starts_table_constraint() {
                        self.table_constraint()?;
                    } else {
                        self.column_def()?;
                    }
                }
            } else {
                break;
            }
        }

        self.consume(Type::BraceRight);
        let (strict, without_rowid) = self.create_table_options();
        self.expect_end("https://www.sqlite.org/lang_createtable.html");

        some_box!(nodes::CreateTable {
            location,
            temporary,
            if_not_exists,
            name,
            columns,
            table_constraints,
            strict,
            without_rowid,
        })
    }

    /// https://www.sqlite.org/syntax/table-options.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_table_options(&mut self) -> (bool, bool) {
        let mut strict = false;
        let mut without_rowid = false;

        while !self.is(Type::Semicolon) && !self.is_eof() {
            match self.cur().ttype {
                // STRICT
                Type::Keyword(Keyword::STRICT) => {
                    strict = true;
                    self.advance();
                }
                // WITHOUT ROWID
                Type::Keyword(Keyword::WITHOUT) => {
                    without_rowid = true;
                    self.advance();
                    self.consume_keyword(Keyword::ROWID);
                }
                _ => {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Unexpected Token",
                        format!(
                            "CREATE TABLE table options expected STRICT or WITHOUT ROWID, got {:?}",
                            self.cur().ttype
                        ),
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/lang_createtable.html",
                    );
                    self.skip_until_semicolon_or_eof();
                    break;
                }
            }

            if self.is(Type::Comma) {
                self.advance();
                if self.is(Type::Semicolon) || self.is_eof() {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed table options",
                        "CREATE TABLE table options list has a trailing comma",
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/lang_createtable.html",
                    );
                    break;
                }
            } else {
                break;
            }
        }

        (strict, without_rowid)
    }

    /// https://www.sqlite.org/lang_createindex.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_index_stmt(
        &mut self,
        location: Location,
        temporary: bool,
        unique: bool,
    ) -> Option<Box<dyn nodes::Node>> {
        if temporary {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Unexpected Token",
                "CREATE INDEX does not support TEMP or TEMPORARY",
                src,
                Rule::Syntax,
                "https://www.sqlite.org/lang_createindex.html",
            );
            self.skip_until_semicolon_or_eof();
            return None;
        }

        // INDEX
        self.consume_keyword(Keyword::INDEX);

        // IF NOT EXISTS
        let if_not_exists = if self.consume_if_keyword(Keyword::IF) {
            self.consume_keyword(Keyword::NOT);
            self.consume_keyword(Keyword::EXISTS);
            true
        } else {
            false
        };

        let name = self.schema_table_container(Some("index"))?;
        self.consume_keyword(Keyword::ON);
        let table =
            self.consume_ident("https://www.sqlite.org/lang_createindex.html", "table_name")?;
        self.consume(Type::BraceLeft);

        let mut columns = vec![];
        loop {
            if self.is(Type::BraceRight) {
                if columns.is_empty() {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed indexed column list",
                        "CREATE INDEX requires at least one indexed column",
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/lang_createindex.html",
                    );
                }
                break;
            }

            let Some(column) = self.indexed_column() else {
                break;
            };
            columns.push(column);

            if self.is(Type::Comma) {
                self.advance();
                if self.is(Type::BraceRight) {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed indexed column list",
                        "CREATE INDEX indexed column list has a trailing comma",
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/lang_createindex.html",
                    );
                    break;
                }
            } else {
                break;
            }
        }

        self.consume(Type::BraceRight);

        // WHERE <expr>
        if self.consume_if_keyword(Keyword::WHERE) {
            let src = Location::from(self.cur());
            self.push_err(
                "Unimplemented",
                "CREATE INDEX partial indexes are not yet supported",
                src,
                Rule::Unimplemented,
            );
            self.skip_until_semicolon_or_eof();
        }

        self.expect_end("https://www.sqlite.org/lang_createindex.html");

        some_box!(nodes::CreateIndex {
            location,
            unique,
            if_not_exists,
            name,
            table,
            columns,
        })
    }

    /// https://www.sqlite.org/lang_createview.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_view_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        // VIEW
        self.consume_keyword(Keyword::VIEW);

        // IF NOT EXISTS
        if self.consume_if_keyword(Keyword::IF) {
            self.consume_keyword(Keyword::NOT);
            self.consume_keyword(Keyword::EXISTS);
        }

        self.schema_table_container(Some("view"))?;
        let mut columns = vec![];

        // (<column-name>, ...)
        if self.is(Type::BraceLeft) {
            self.advance();
            loop {
                if self.is(Type::BraceRight) {
                    if columns.is_empty() {
                        let src = Location::from(self.cur());
                        self.push_doc_err(
                            "Malformed view column list",
                            "CREATE VIEW column list requires at least one column name",
                            src,
                            Rule::Syntax,
                            "https://www.sqlite.org/lang_createview.html",
                        );
                    }
                    break;
                }

                columns.push(
                    self.consume_ident(
                        "https://www.sqlite.org/lang_createview.html",
                        "column_name",
                    )?,
                );

                if self.is(Type::Comma) {
                    self.advance();
                    if self.is(Type::BraceRight) {
                        let src = Location::from(self.cur());
                        self.push_doc_err(
                            "Malformed view column list",
                            "CREATE VIEW column list has a trailing comma",
                            src,
                            Rule::Syntax,
                            "https://www.sqlite.org/lang_createview.html",
                        );
                        break;
                    }
                } else {
                    break;
                }
            }
            self.consume(Type::BraceRight);
        }

        self.consume_keyword(Keyword::AS);

        // <select-stmt>
        if self.is_keyword(Keyword::SELECT) {
            let src = Location::from(self.cur());
            self.push_err(
                "Unimplemented",
                "CREATE VIEW ... AS <select_stmt> is not yet supported",
                src,
                Rule::Unimplemented,
            );
            self.skip_until_semicolon_or_eof();
        } else {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Unexpected Token",
                format!(
                    "CREATE VIEW requires select-stmt after AS, got {:?}",
                    self.cur().ttype
                ),
                src,
                Rule::Syntax,
                "https://www.sqlite.org/lang_createview.html",
            );
            self.advance();
        }

        self.expect_end("https://www.sqlite.org/lang_createview.html");
        None
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_trigger_stmt(
        &mut self,
        location: Location,
        temporary: bool,
    ) -> Option<Box<dyn nodes::Node>> {
        // TRIGGER
        self.consume_keyword(Keyword::TRIGGER);

        // IF NOT EXISTS
        let if_not_exists = if self.consume_if_keyword(Keyword::IF) {
            self.consume_keyword(Keyword::NOT);
            self.consume_keyword(Keyword::EXISTS);
            true
        } else {
            false
        };

        let name = self.schema_table_container(Some("trigger"))?;
        let timing = self.trigger_timing()?;
        let event = self.trigger_event()?;
        self.consume_keyword(Keyword::ON);
        let table = self.consume_ident(
            "https://www.sqlite.org/lang_createtrigger.html",
            "table_name",
        )?;
        let for_each_row = self.trigger_for_each_row();
        let when = self.trigger_when_clause();
        self.consume_keyword(Keyword::BEGIN);
        let body = self.trigger_body();
        self.consume_keyword(Keyword::END);
        self.expect_end("https://www.sqlite.org/lang_createtrigger.html");

        some_box!(nodes::CreateTrigger {
            location,
            temporary,
            if_not_exists,
            name,
            timing,
            event,
            table,
            for_each_row,
            when,
            body,
        })
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_timing(&mut self) -> Option<Option<nodes::TriggerTiming>> {
        match self.cur().ttype {
            // BEFORE
            Type::Keyword(Keyword::BEFORE) => {
                self.advance();
                Some(Some(nodes::TriggerTiming::Before))
            }
            // AFTER
            Type::Keyword(Keyword::AFTER) => {
                self.advance();
                Some(Some(nodes::TriggerTiming::After))
            }
            // INSTEAD OF
            Type::Keyword(Keyword::INSTEAD) => {
                self.advance();
                self.consume_keyword(Keyword::OF);
                Some(Some(nodes::TriggerTiming::InsteadOf))
            }
            // no trigger-time, event starts here
            Type::Keyword(Keyword::DELETE)
            | Type::Keyword(Keyword::INSERT)
            | Type::Keyword(Keyword::UPDATE) => Some(None),
            _ => {
                let src = Location::from(self.cur());
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "CREATE TRIGGER expected BEFORE, AFTER, INSTEAD OF, DELETE, INSERT or UPDATE, got {:?}",
                        self.cur().ttype
                    ),
                    src,
                    Rule::Syntax,
                    "https://www.sqlite.org/lang_createtrigger.html",
                );
                self.skip_until_semicolon_or_eof();
                None
            }
        }
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_event(&mut self) -> Option<nodes::TriggerEvent> {
        match self.cur().ttype {
            // DELETE
            Type::Keyword(Keyword::DELETE) => {
                self.advance();
                Some(nodes::TriggerEvent::Delete)
            }
            // INSERT
            Type::Keyword(Keyword::INSERT) => {
                self.advance();
                Some(nodes::TriggerEvent::Insert)
            }
            // UPDATE [OF <column-name>, ...]
            Type::Keyword(Keyword::UPDATE) => {
                self.advance();
                let columns = if self.consume_if_keyword(Keyword::OF) {
                    self.trigger_column_list()?
                } else {
                    vec![]
                };
                Some(nodes::TriggerEvent::Update { columns })
            }
            _ => {
                let src = Location::from(self.cur());
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "CREATE TRIGGER expected DELETE, INSERT or UPDATE event, got {:?}",
                        self.cur().ttype
                    ),
                    src,
                    Rule::Syntax,
                    "https://www.sqlite.org/lang_createtrigger.html",
                );
                self.skip_until_semicolon_or_eof();
                None
            }
        }
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_column_list(&mut self) -> Option<Vec<String>> {
        let mut columns = vec![];
        loop {
            columns.push(self.consume_ident(
                "https://www.sqlite.org/lang_createtrigger.html",
                "column_name",
            )?);

            if self.is(Type::Comma) {
                self.advance();
                if self.is_keyword(Keyword::ON) {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed trigger column list",
                        "CREATE TRIGGER UPDATE OF column list has a trailing comma",
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/lang_createtrigger.html",
                    );
                    return None;
                }
            } else {
                break;
            }
        }
        Some(columns)
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_for_each_row(&mut self) -> bool {
        // no FOR EACH ROW
        if !self.consume_if_keyword(Keyword::FOR) {
            return false;
        }

        // FOR EACH ROW
        self.consume_keyword(Keyword::EACH);
        self.consume_keyword(Keyword::ROW);
        true
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_when_clause(&mut self) -> bool {
        // no WHEN clause
        if !self.consume_if_keyword(Keyword::WHEN) {
            return false;
        }

        // WHEN <expr>
        if self.is_keyword(Keyword::BEGIN) {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Malformed trigger WHEN clause",
                "CREATE TRIGGER WHEN requires an expression before BEGIN",
                src,
                Rule::Syntax,
                "https://www.sqlite.org/lang_createtrigger.html",
            );
            return true;
        }

        // The parser currently records that a WHEN clause exists, but does not build an AST for
        // the expression itself. Advance to the body-opening BEGIN so the rest of the trigger can
        // still be parsed. Parenthesized expressions may contain BEGIN-like tokens, so only a
        // depth-zero BEGIN terminates this skip.
        let mut depth = 0;
        while !self.is_eof() {
            match self.cur().ttype {
                Type::BraceLeft => depth += 1,
                Type::BraceRight if depth > 0 => depth -= 1,
                Type::Keyword(Keyword::BEGIN) if depth == 0 => break,
                _ => {}
            }
            self.advance();
        }
        true
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_body(&mut self) -> Vec<nodes::TriggerBodyStmt> {
        let mut body = vec![];

        while !self.is_eof() && !self.is_keyword(Keyword::END) {
            let Some(stmt) = self.trigger_body_stmt_kind() else {
                // We could not identify a supported trigger body statement kind at the current
                // token. Skip the unknown statement payload until either its semicolon or the
                // trigger-level END keyword so the outer CREATE TRIGGER parser can resynchronize.
                while !self.is_eof() && !self.is(Type::Semicolon) && !self.is_keyword(Keyword::END)
                {
                    self.advance();
                }
                break;
            };
            body.push(stmt);
            // Trigger body statements are represented by kind only for now; their SELECT/UPDATE/
            // INSERT/DELETE payloads are not parsed into AST nodes yet. Keep the recognized kind,
            // then skip the remaining payload until the statement semicolon or trigger-level END.
            while !self.is_eof() && !self.is(Type::Semicolon) && !self.is_keyword(Keyword::END) {
                self.advance();
            }

            if self.is(Type::Semicolon) {
                self.advance();
            } else if self.is_keyword(Keyword::END) {
                let src = Location::from(self.cur());
                self.push_err(
                    "Missing semicolon",
                    "Wanted Semicolon before END, terminate statements with ';'",
                    src,
                    Rule::Semicolon,
                );
                break;
            } else if !self.is_keyword(Keyword::END) {
                self.consume(Type::Semicolon);
                break;
            }
        }

        if body.is_empty() {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Malformed trigger body",
                "CREATE TRIGGER body requires at least one trigger statement",
                src,
                Rule::Syntax,
                "https://www.sqlite.org/lang_createtrigger.html",
            );
        }

        body
    }

    /// https://www.sqlite.org/lang_createtrigger.html
    #[cfg_attr(feature = "trace", trace)]
    fn trigger_body_stmt_kind(&mut self) -> Option<nodes::TriggerBodyStmt> {
        match self.cur().ttype {
            // DELETE ...
            Type::Keyword(Keyword::DELETE) => Some(nodes::TriggerBodyStmt::Delete),
            // INSERT ...
            Type::Keyword(Keyword::INSERT) => Some(nodes::TriggerBodyStmt::Insert),
            // SELECT ...
            Type::Keyword(Keyword::SELECT) => Some(nodes::TriggerBodyStmt::Select),
            // UPDATE ...
            Type::Keyword(Keyword::UPDATE) => Some(nodes::TriggerBodyStmt::Update),
            _ => {
                let src = Location::from(self.cur());
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "CREATE TRIGGER body expected DELETE, INSERT, SELECT or UPDATE, got {:?}",
                        self.cur().ttype
                    ),
                    src,
                    Rule::Syntax,
                    "https://www.sqlite.org/lang_createtrigger.html",
                );
                None
            }
        }
    }

    /// https://www.sqlite.org/syntax/indexed-column.html
    #[cfg_attr(feature = "trace", trace)]
    fn indexed_column(&mut self) -> Option<nodes::IndexedColumn> {
        let name = match self.cur().ttype.clone() {
            Type::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                let src = Location::from(self.cur());
                self.push_err(
                    "Unimplemented",
                    "CREATE INDEX expression indexes are not yet supported",
                    src,
                    Rule::Unimplemented,
                );

                // Skip the unsupported indexed-column expression until the next top-level comma
                // or closing parenthesis, so the surrounding column list can recover cleanly.
                let mut depth = 0;
                while !self.is_eof() {
                    match self.cur().ttype {
                        Type::BraceLeft => depth += 1,
                        Type::BraceRight if depth == 0 => break,
                        Type::BraceRight => depth -= 1,
                        Type::Comma if depth == 0 => break,
                        _ => {}
                    }
                    self.advance();
                }
                return None;
            }
        };

        let collation = if self.consume_if_keyword(Keyword::COLLATE) {
            Some(self.consume_ident(
                "https://www.sqlite.org/syntax/indexed-column.html",
                "collation_name",
            )?)
        } else {
            None
        };

        let order = self.consume_optional_keyword(&[Keyword::ASC, Keyword::DESC]);

        Some(nodes::IndexedColumn {
            name,
            collation,
            order,
        })
    }

    /// https://www.sqlite.org/pragma.html
    #[cfg_attr(feature = "trace", trace)]
    fn pragma_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let location = Location::from(self.cur());

        // PRAGMA
        self.advance();

        // [schema-name].<pragma-name>
        let Some(schema_and_pragma) = self.schema_table_container(Some("pragma")) else {
            return None;
        };

        // ;EOF
        let pragma = if self.is(Type::Semicolon) {
            Pragma {
                location,
                name: schema_and_pragma,
                invocation: nodes::PragmaInvocation::Query,
            }
        } else if self.is(Type::Equal) {
            // = <pragma-value>
            self.advance();
            Pragma {
                location,
                name: schema_and_pragma,
                invocation: nodes::PragmaInvocation::Assign {
                    value: self.consume_pragma_value("assignment"),
                },
            }
        } else if self.is(Type::BraceLeft) {
            // (<pragma-value>)
            self.advance();
            let value = self.consume_pragma_value("call");
            self.consume(Type::BraceRight);
            Pragma {
                location,
                name: schema_and_pragma,
                invocation: nodes::PragmaInvocation::Call { value },
            }
        } else {
            let cur = self.cur().clone();
            self.push_err(
                "Bad pragma value",
                &format!(
                    "A pragma rhs value has to be either an assignment via '=', a call via '(<arg>)' or simply be a query, got {:?} instead",
                    cur.ttype
                ),
                &cur,
                Rule::Syntax,
            );
            self.advance();
            return None;
        };

        self.expect_end("https://www.sqlite.org/pragma.html");

        some_box!(pragma)
    }

    /// https://www.sqlite.org/lang_altertable.html
    #[cfg_attr(feature = "trace", trace)]
    fn alter_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut a = nodes::Alter {
            location: Location::from(self.cur()),
            target: SchemaTableContainer::Table(String::new()),
            rename_to: None,
            rename_column_target: None,
            new_column_name: None,
            add_column: None,
            drop_column: None,
        };

        self.advance();
        self.consume(Type::Keyword(Keyword::TABLE));

        a.target = self.schema_table_container(None)?;

        match self.cur().ttype {
            // RENAME TO <new-table-name> | RENAME [COLUMN] <old-name> TO <new-name>
            Type::Keyword(Keyword::RENAME) => {
                self.advance();
                if self.is(Type::Keyword(Keyword::TO)) {
                    // RENAME TO <new_table_name>
                    self.advance();
                    let new_table_name = self.consume_ident(
                        "https://www.sqlite.org/lang_altertable.html",
                        "new_table_name",
                    )?;
                    a.rename_to = Some(new_table_name);
                } else {
                    if self.is(Type::Keyword(Keyword::COLUMN)) {
                        self.advance();
                    }

                    a.rename_column_target = self.consume_ident(
                        "https://www.sqlite.org/lang_altertable.html",
                        "column_name",
                    );
                    self.consume(Type::Keyword(Keyword::TO));
                    a.new_column_name = self.consume_ident(
                        "https://www.sqlite.org/lang_altertable.html",
                        "column_name",
                    );
                }
            }
            // ADD [COLUMN] <column-def>
            Type::Keyword(Keyword::ADD) => {
                self.advance();
                if self.is(Type::Keyword(Keyword::COLUMN)) {
                    self.advance();
                }

                a.add_column = self.column_def();
            }
            // DROP [COLUMN] <column-name>
            Type::Keyword(Keyword::DROP) => {
                self.advance();
                if self.is(Type::Keyword(Keyword::COLUMN)) {
                    self.advance();
                }
                a.drop_column = self
                    .consume_ident("https://www.sqlite.org/lang_altertable.html", "column_name");
            }
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "ALTER requires either RENAME, ADD or DROP at this point, got {:?}",
                        self.cur().ttype
                    ),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_altertable.html");
                self.errors.push(err);
                self.advance();
                return None;
            }
        }

        self.expect_end("https://www.sqlite.org/lang_altertable.html");

        some_box!(a)
    }

    /// https://www.sqlite.org/syntax/reindex-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn reindex_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut r = nodes::Reindex {
            location: Location::from(self.cur()),
            target: None,
        };
        self.advance();

        // REINDEX has a path with no further nodes
        if self.is(Type::Semicolon) {
            return some_box!(r);
        }

        r.target = self.schema_table_container(None);

        self.expect_end("https://www.sqlite.org/syntax/reindex-stmt.html");

        some_box!(r)
    }

    /// https://www.sqlite.org/syntax/attach-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn attach_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let location = Location::from(self.cur());
        // skipping ATTACH
        self.advance();
        // skipping optional DATABASE
        if self.is(Type::Keyword(Keyword::DATABASE)) {
            self.advance();
        }

        let mut a = nodes::Attach {
            location,
            schema_name: String::new(),
            expr: self.expr()?,
        };

        self.consume(Type::Keyword(Keyword::AS));

        a.schema_name =
            self.consume_ident("https://www.sqlite.org/lang_attach.html", "schema_name")?;

        self.expect_end("https://www.sqlite.org/lang_attach.html");

        some_box!(a)
    }

    /// https://www.sqlite.org/syntax/release-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn release_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut r = nodes::Release {
            location: Location::from(self.cur()),
            savepoint_name: String::new(),
        };
        self.advance();

        if self.is(Type::Keyword(Keyword::SAVEPOINT)) {
            self.advance();
        }

        r.savepoint_name = self.consume_ident(
            "https://www.sqlite.org/syntax/release-stmt.html",
            "savepoint_name",
        )?;

        self.expect_end("https://www.sqlite.org/syntax/release-stmt.html");

        some_box!(r)
    }

    /// https://www.sqlite.org/syntax/savepoint-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn savepoint_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut s = nodes::Savepoint {
            location: Location::from(self.cur()),
            savepoint_name: String::new(),
        };
        self.advance();
        s.savepoint_name = self.consume_ident(
            "https://www.sqlite.org/syntax/savepoint-stmt.html",
            "savepoint_name",
        )?;
        self.expect_end("https://www.sqlite.org/lang_savepoint.html");

        some_box!(s)
    }

    /// https://www.sqlite.org/lang_dropindex.html
    /// https://www.sqlite.org/lang_droptable.html
    /// https://www.sqlite.org/lang_droptrigger.html
    /// https://www.sqlite.org/lang_dropview.html
    #[cfg_attr(feature = "trace", trace)]
    fn drop_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let location = Location::from(self.cur());
        self.advance();

        match self.cur().ttype {
            // INDEX
            Type::Keyword(Keyword::INDEX) => (),
            // TABLE
            Type::Keyword(Keyword::TABLE) => (),
            // TRIGGER
            Type::Keyword(Keyword::TRIGGER) => (),
            // VIEW
            Type::Keyword(Keyword::VIEW) => (),
            _ => {
                let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "DROP requires either TRIGGER, TABLE, TRIGGER or VIEW at this point, got {:?}",
                            self.cur().ttype
                        ),
                        self.cur(),
                        Rule::Syntax,
                    );
                err.doc_url = Some("https://www.sqlite.org/lang.html");
                self.errors.push(err);
                self.advance();
                return None;
            }
        }

        let ttype = {
            let Type::Keyword(keyword) = &self.cur().ttype else {
                unreachable!("self.cur() in (in the set theory kind) {{INDEX,TABLE,TRIGGER,VIEW}}")
            };
            *keyword
        };

        // INDEX | TABLE | TRIGGER | VIEW
        self.advance();

        // IF EXISTS
        let if_exists = if self.is(Type::Keyword(Keyword::IF)) {
            self.advance();
            self.consume(Type::Keyword(Keyword::EXISTS));
            true
        } else {
            false
        };

        let argument = self.schema_table_container(None)?;

        some_box!(nodes::Drop {
            location,
            ttype,
            if_exists,
            argument,
        })
    }

    /// https://www.sqlite.org/syntax/analyze-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn analyze_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut a = nodes::Analyze {
            location: Location::from(self.cur()),
            target: None,
        };

        self.advance();

        if !self.is(Type::Semicolon) {
            a.target = self.schema_table_container(None);
        }

        self.expect_end("https://www.sqlite.org/lang_analyze.html");

        some_box!(a)
    }

    /// https://www.sqlite.org/syntax/detach-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn detach_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let location = Location::from(self.cur());
        self.advance();

        // skip optional DATABASE path
        if self.is(Type::Keyword(Keyword::DATABASE)) {
            self.advance();
        }

        let schema_name =
            self.consume_ident("https://www.sqlite.org/lang_detach.html", "schema_name")?;

        let d = nodes::Detach {
            location,
            schema_name,
        };

        self.expect_end("https://www.sqlite.org/lang_detach.html");

        some_box!(d)
    }

    /// https://www.sqlite.org/syntax/rollback-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn rollback_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut rollback = nodes::Rollback {
            location: Location::from(self.cur()),
            save_point: None,
        };
        self.advance();

        match self.cur().ttype {
            // TRANSACTION | TO | ;
            Type::Keyword(Keyword::TRANSACTION) | Type::Keyword(Keyword::TO) | Type::Semicolon => {}
            _ => {
                let cur = self.cur().clone();
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "ROLLBACK requires TRANSACTION, TO or to end at this point, got {:?}",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                    TRANSACTION_DOC,
                );
            }
        }

        self.consume_if_keyword(Keyword::TRANSACTION);

        // TO [SAVEPOINT] <savepoint-name>
        if self.consume_if_keyword(Keyword::TO) {
            self.consume_if_keyword(Keyword::SAVEPOINT);

            match self.cur().ttype {
                // <savepoint-name> | ;
                Type::Keyword(Keyword::SAVEPOINT) | Type::Ident(_) | Type::Semicolon => {}
                _ => {
                    let cur = self.cur().clone();
                    self.push_doc_err(
                        "Unexpected Token",
                        format!(
                            "ROLLBACK requires SAVEPOINT, Ident or to end at this point, got {:?}",
                            cur.ttype
                        ),
                        &cur,
                        Rule::Syntax,
                        TRANSACTION_DOC,
                    );
                    self.advance();
                }
            }

            if let Type::Ident(str) = &self.cur().ttype {
                rollback.save_point = Some(String::from(str));
            } else {
                let cur = self.cur().clone();
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "ROLLBACK wants Ident as <savepoint-name>, got {:?}",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                    TRANSACTION_DOC,
                );
            }
            self.advance();
        }

        self.expect_end(TRANSACTION_DOC);

        some_box!(rollback)
    }

    /// https://www.sqlite.org/syntax/commit-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn commit_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let commit: Option<Box<dyn nodes::Node>> = some_box!(nodes::Commit {
            location: Location::from(self.cur()),
        });

        // skip either COMMIT or END
        self.advance();

        match self.cur().ttype {
            // ;
            Type::Semicolon => (),
            // TRANSACTION
            Type::Keyword(Keyword::TRANSACTION) => self.advance(),
            _ => {
                let cur = self.cur().clone();
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "Wanted Keyword(TRANSACTION) or Semicolon, got {:?}",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                    TRANSACTION_DOC,
                );
                self.advance();
            }
        }

        self.expect_end(TRANSACTION_DOC);

        commit
    }

    /// https://www.sqlite.org/syntax/begin-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn begin_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut begin: nodes::Begin = nodes::Begin {
            location: Location::from(self.cur()),
            transaction_kind: None,
        };

        // skip BEGIN
        self.advance();

        if self.is(Type::Semicolon) {
            return some_box!(begin);
        }

        begin.transaction_kind = self.consume_transaction_kind();

        match self.cur().ttype {
            // ;
            Type::Semicolon => return some_box!(begin),
            // TRANSACTION
            Type::Keyword(Keyword::TRANSACTION) => self.advance(),
            // second transaction-kind
            Type::Keyword(Keyword::DEFERRED)
            | Type::Keyword(Keyword::IMMEDIATE)
            | Type::Keyword(Keyword::EXCLUSIVE) => {
                let cur = self.cur().clone();
                self.push_doc_err(
                    "Unexpected Token",
                    "BEGIN does not allow multiple transaction behaviour modifiers",
                    &cur,
                    Rule::Syntax,
                    TRANSACTION_DOC,
                );
                // TODO: think about if this is smart at this point, skipping to the next ; could
                // be skipping too many tokens
                self.skip_until_semicolon_or_eof();
            }
            _ => {
                let cur = self.cur().clone();
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "Wanted any of TRANSACTION, DEFERRED, IMMEDIATE or EXCLUSIVE before this point, got {:?}",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                    TRANSACTION_DOC,
                );
            }
        }

        self.expect_end(TRANSACTION_DOC);

        some_box!(begin)
    }

    #[cfg_attr(feature = "trace", trace)]
    fn consume_transaction_kind(&mut self) -> Option<Keyword> {
        match self.cur().ttype {
            // DEFERRED | IMMEDIATE | EXCLUSIVE
            Type::Keyword(kind @ (Keyword::DEFERRED | Keyword::IMMEDIATE | Keyword::EXCLUSIVE)) => {
                self.advance();
                Some(kind)
            }
            _ => None,
        }
    }

    /// https://www.sqlite.org/lang_vacuum.html
    #[cfg_attr(feature = "trace", trace)]
    fn vacuum_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut v = nodes::Vacuum {
            location: Location::from(self.cur()),
            schema_name: None,
            filename: None,
        };
        self.consume(Type::Keyword(Keyword::VACUUM));

        match self.cur().ttype {
            // ; | <schema-name> | INTO
            Type::Semicolon | Type::Ident(_) | Type::Keyword(Keyword::INTO) => {}
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted {:?} with {:?} or {:?} for VACUUM stmt, got {:?}",
                        Type::Keyword(Keyword::INTO),
                        Type::String("<filename>".to_string()),
                        Type::Ident("<schema_name>".to_string()),
                        self.cur().ttype.clone()
                    ),
                    &self.cur().clone(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_vacuum.html");
                self.errors.push(err);
                self.advance(); // skip error_token
            }
        }

        // ;
        if let Type::Semicolon = self.cur().ttype {
            return some_box!(v);
        }

        // <schema-name>
        if let Type::Ident(_) = self.cur().ttype {
            v.schema_name = Some(self.cur().clone());
            self.advance(); // skip schema_name
        }

        // INTO <filename>
        if let Type::Keyword(Keyword::INTO) = self.cur().ttype {
            self.advance(); // skip INTO
            if let Type::String(_) = self.cur().ttype {
                v.filename = Some(self.cur().clone());
            } else {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted {:?} for VACUUM stmt with {:?}, got {:?}",
                        Type::String("<filename>".to_string()),
                        Type::Keyword(Keyword::INTO),
                        self.cur().ttype.clone()
                    ),
                    &self.cur().clone(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_vacuum.html");
                self.errors.push(err);
            }
            self.advance(); // skip filename or error token
        }

        self.expect_end("https://www.sqlite.org/lang_vacuum.html");

        some_box!(v)
    }

    /// see: https://www.sqlite.org/syntax/literal-value.html
    #[cfg_attr(feature = "trace", trace)]
    fn literal_value(&mut self) -> Option<nodes::Literal> {
        let cur = self.cur();
        match cur.ttype {
            Type::String(_)
            | Type::Number(_)
            | Type::Blob(_)
            | Type::Keyword(Keyword::NULL)
            | Type::Boolean(_)
            | Type::Keyword(Keyword::CURRENT_TIME)
            | Type::Keyword(Keyword::CURRENT_DATE)
            | Type::Keyword(Keyword::CURRENT_TIMESTAMP) => {
                let literal = nodes::Literal {
                    location: Location::from(cur),
                    value: cur.clone(),
                };
                // skipping over the current character
                self.advance();
                Some(literal)
            }
            _ => {
                let mut err = self.err("Unexpected Token", &format!("Wanted a literal (any of number,string,blob,null,true,false,CURRENT_TIME,CURRENT_DATE,CURRENT_DATE), got {:?}", cur.ttype),cur, Rule::Syntax);
                err.doc_url = Some("https://www.sqlite.org/syntax/literal-value.html");
                self.errors.push(err);
                self.advance();
                None
            }
        }
    }

    /// parses an sql expression: https://www.sqlite.org/syntax/expr.html
    #[cfg_attr(feature = "trace", trace)]
    fn expr(&mut self) -> Option<nodes::Expr> {
        self.expr_bp(0)
    }

    /// Pratt parser for the SQLite expression subset sqleibniz currently models.
    ///
    /// This handles primary expressions, unary operators, binary operators, grouped expressions,
    /// and identifier-led column/function expressions. The surrounding parser decides where an
    /// expression must stop by leaving `)`, `,`, and statement terminators out of the operator set.
    ///
    /// See: https://www.sqlite.org/lang_expr.html
    #[cfg_attr(feature = "trace", trace)]
    fn expr_bp(&mut self, min_bp: u8) -> Option<nodes::Expr> {
        let mut lhs = self.expr_prefix()?;

        loop {
            let Some(next) = self.expr_operator() else {
                break;
            };

            let location = lhs.location;
            match next {
                ExprOperator::Postfix {
                    operator,
                    left_bp,
                    advance,
                } => {
                    if left_bp < min_bp {
                        break;
                    }
                    for _ in 0..advance {
                        self.advance();
                    }
                    lhs = self.operator_expr(location, operator, vec![lhs]);
                }
                ExprOperator::Infix {
                    operator,
                    left_bp,
                    right_bp,
                    advance,
                } => {
                    if left_bp < min_bp {
                        break;
                    }
                    for _ in 0..advance {
                        self.advance();
                    }
                    let rhs = self.expr_bp(right_bp)?;
                    lhs = self.operator_expr(location, operator, vec![lhs, rhs]);
                }
                ExprOperator::Between {
                    operator,
                    left_bp,
                    right_bp,
                    advance,
                } => {
                    if left_bp < min_bp {
                        break;
                    }
                    for _ in 0..advance {
                        self.advance();
                    }
                    let lower = self.expr_bp(right_bp)?;
                    self.consume_keyword(Keyword::AND);
                    let upper = self.expr_bp(right_bp)?;
                    lhs = self.operator_expr(location, operator, vec![lhs, lower, upper]);
                }
                ExprOperator::InList {
                    operator,
                    left_bp,
                    advance,
                } => {
                    if left_bp < min_bp {
                        break;
                    }
                    for _ in 0..advance {
                        self.advance();
                    }
                    self.consume(Type::BraceLeft);
                    let mut arguments = vec![lhs];

                    if self.consume_if(Type::BraceRight) {
                        lhs = self.operator_expr(location, operator, arguments);
                        continue;
                    }

                    loop {
                        arguments.push(self.expr()?);

                        if self.consume_if(Type::Comma) {
                            if self.is(Type::BraceRight) {
                                let src = Location::from(self.cur());
                                self.push_doc_err(
                                    "Malformed IN expression",
                                    "IN expression value list has a trailing comma",
                                    src,
                                    Rule::Syntax,
                                    "https://www.sqlite.org/lang_expr.html#the_in_and_not_in_operators",
                                );
                                break;
                            }
                            continue;
                        }

                        break;
                    }

                    self.consume(Type::BraceRight);
                    lhs = self.operator_expr(location, operator, arguments);
                }
            }
        }

        Some(lhs)
    }

    fn operator_expr(
        &self,
        location: Location,
        operator: String,
        arguments: Vec<nodes::Expr>,
    ) -> nodes::Expr {
        nodes::Expr {
            location,
            literal: None,
            bind: None,
            schema: None,
            table: None,
            column: None,
            function: None,
            operator: Some(operator),
            arguments,
        }
    }

    #[cfg_attr(feature = "trace", trace)]
    fn expr_prefix(&mut self) -> Option<nodes::Expr> {
        let mut e = nodes::Expr {
            location: Location::from(self.cur()),
            literal: None,
            bind: None,
            schema: None,
            table: None,
            column: None,
            function: None,
            operator: None,
            arguments: vec![],
        };
        match self.cur().ttype {
            // literal value
            Type::String(_)
            | Type::Number(_)
            | Type::Blob(_)
            | Type::Keyword(Keyword::NULL)
            | Type::Boolean(_)
            | Type::Keyword(Keyword::CURRENT_TIME)
            | Type::Keyword(Keyword::CURRENT_DATE)
            | Type::Keyword(Keyword::CURRENT_TIMESTAMP) => {
                e.literal = self.literal_value().map(|literal| literal.value)
            }
            // bind parameter with optional ident: ?[ident]
            Type::Question => {
                // sqlite documentation says: But because it is easy to miscount the question marks, the
                // use of this parameter format is discouraged. Programmers are encouraged to use
                // one of the symbolic formats [...] or the ?NNN format [...] instead.
                let mut param = BindParameter {
                    location: Location::from(self.cur()),

                    counter: None,
                    name: None,
                };
                self.advance();

                // question mark can have a number after them, but they are optional
                if let Token {
                    ttype: Type::Number(_),
                    ..
                } = self.cur()
                {
                    param.counter = self
                        .literal_value()
                        .map(|literal| Box::new(literal) as Box<dyn nodes::Node>);
                }
                e.bind = Some(param)
            }
            // bind parameter with required ident: [:@$]<ident>
            Type::Colon | Type::At | Type::Dollar => {
                let bind_location = Location::from(self.cur());
                let bind_type = self.cur().ttype.clone();
                let mut bind = BindParameter {
                    location: bind_location,
                    counter: None,
                    name: None,
                };
                self.advance();

                // all bind params need an identifier, because they need to be named
                if let Token {
                    ttype: Type::Ident(ident),
                    ..
                } = self.cur()
                {
                    bind.name = Some(ident.clone());
                    self.advance();
                } else {
                    self.push_err(
                        "Invalid bind parameter",
                        &format!(
                            "Bind parameter with {:?} requires an identifier as a postfix",
                            bind_type
                        ),
                        bind_location,
                        Rule::Syntax,
                    );
                    // skip invalid token
                    self.advance();
                    return None;
                }
                e.bind = Some(bind);
            }
            Type::Ident(_) => return self.ident_expr(),
            Type::BraceLeft => {
                self.advance();
                let expr = self.expr()?;
                self.consume(Type::BraceRight);
                return Some(expr);
            }
            Type::Plus | Type::Minus | Type::Tilde | Type::Keyword(Keyword::NOT) => {
                let operator = self.prefix_operator().unwrap();
                let location = Location::from(self.cur());
                self.advance();
                let argument = self.expr_bp(prefix_binding_power(&operator))?;
                return Some(self.operator_expr(location, operator, vec![argument]));
            }
            _ => {
                let cur = self.cur().clone();
                self.push_err(
                    "Invalid construct",
                    &format!(
                        "At this point in an expression, {:?} is not a valid construct",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                );
                self.advance();
                return None;
            }
        }
        Some(e)
    }

    fn prefix_operator(&self) -> Option<String> {
        match self.cur().ttype {
            Type::Plus => Some("+".into()),
            Type::Minus => Some("-".into()),
            Type::Tilde => Some("~".into()),
            Type::Keyword(Keyword::NOT) => Some("NOT".into()),
            _ => None,
        }
    }

    fn expr_operator(&self) -> Option<ExprOperator> {
        match self.cur().ttype {
            Type::Keyword(Keyword::OR) => Some(ExprOperator::infix("OR", 1, 2)),
            Type::Keyword(Keyword::AND) => Some(ExprOperator::infix("AND", 3, 4)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::BETWEEN)) => {
                Some(ExprOperator::between("NOT BETWEEN", 5, 6, 2))
            }
            Type::Keyword(Keyword::BETWEEN) => Some(ExprOperator::between("BETWEEN", 5, 6, 1)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::IN)) => {
                Some(ExprOperator::in_list("NOT IN", 5, 2))
            }
            Type::Keyword(Keyword::IN) => Some(ExprOperator::in_list("IN", 5, 1)),
            Type::Keyword(Keyword::IS)
                if self.next_is(Type::Keyword(Keyword::DISTINCT))
                    && self.nth_is(2, Type::Keyword(Keyword::FROM)) =>
            {
                Some(ExprOperator::infix_advanced("IS DISTINCT FROM", 5, 6, 3))
            }
            Type::Keyword(Keyword::IS)
                if self.next_is(Type::Keyword(Keyword::NOT))
                    && self.nth_is(2, Type::Keyword(Keyword::DISTINCT))
                    && self.nth_is(3, Type::Keyword(Keyword::FROM)) =>
            {
                Some(ExprOperator::infix_advanced(
                    "IS NOT DISTINCT FROM",
                    5,
                    6,
                    4,
                ))
            }
            Type::Keyword(Keyword::IS) if self.next_is(Type::Keyword(Keyword::NOT)) => {
                Some(ExprOperator::infix_advanced("IS NOT", 5, 6, 2))
            }
            Type::Keyword(Keyword::IS) => Some(ExprOperator::infix("IS", 5, 6)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::LIKE)) => {
                Some(ExprOperator::infix_advanced("NOT LIKE", 5, 6, 2))
            }
            Type::Keyword(Keyword::LIKE) => Some(ExprOperator::infix("LIKE", 5, 6)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::GLOB)) => {
                Some(ExprOperator::infix_advanced("NOT GLOB", 5, 6, 2))
            }
            Type::Keyword(Keyword::GLOB) => Some(ExprOperator::infix("GLOB", 5, 6)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::MATCH)) => {
                Some(ExprOperator::infix_advanced("NOT MATCH", 5, 6, 2))
            }
            Type::Keyword(Keyword::MATCH) => Some(ExprOperator::infix("MATCH", 5, 6)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::REGEXP)) => {
                Some(ExprOperator::infix_advanced("NOT REGEXP", 5, 6, 2))
            }
            Type::Keyword(Keyword::REGEXP) => Some(ExprOperator::infix("REGEXP", 5, 6)),
            Type::Keyword(Keyword::ESCAPE) => Some(ExprOperator::infix("ESCAPE", 5, 6)),
            Type::Equal => Some(ExprOperator::infix("=", 5, 6)),
            Type::EqualEqual => Some(ExprOperator::infix("==", 5, 6)),
            Type::Less => Some(ExprOperator::infix("<", 5, 6)),
            Type::LessEqual => Some(ExprOperator::infix("<=", 5, 6)),
            Type::Greater => Some(ExprOperator::infix(">", 5, 6)),
            Type::GreaterEqual => Some(ExprOperator::infix(">=", 5, 6)),
            Type::NotEqual => Some(ExprOperator::infix("!=", 5, 6)),
            Type::Ampersand => Some(ExprOperator::infix("&", 7, 8)),
            Type::Pipe => Some(ExprOperator::infix("|", 7, 8)),
            Type::ShiftLeft => Some(ExprOperator::infix("<<", 7, 8)),
            Type::ShiftRight => Some(ExprOperator::infix(">>", 7, 8)),
            Type::Plus => Some(ExprOperator::infix("+", 9, 10)),
            Type::Minus => Some(ExprOperator::infix("-", 9, 10)),
            Type::Asterisk => Some(ExprOperator::infix("*", 11, 12)),
            Type::Slash => Some(ExprOperator::infix("/", 11, 12)),
            Type::Percent => Some(ExprOperator::infix("%", 11, 12)),
            Type::PipePipe => Some(ExprOperator::infix("||", 13, 14)),
            Type::Arrow => Some(ExprOperator::infix("->", 13, 14)),
            Type::ArrowArrow => Some(ExprOperator::infix("->>", 13, 14)),
            Type::Keyword(Keyword::COLLATE) => Some(ExprOperator::infix("COLLATE", 15, 16)),
            Type::Keyword(Keyword::ISNULL) => Some(ExprOperator::postfix("ISNULL", 5)),
            Type::Keyword(Keyword::NOTNULL) => Some(ExprOperator::postfix("NOTNULL", 5)),
            Type::Keyword(Keyword::NOT) if self.next_is(Type::Keyword(Keyword::NULL)) => {
                Some(ExprOperator::postfix_advanced("NOT NULL", 5, 2))
            }
            _ => None,
        }
    }

    /// Parses an identifier-led expression.
    ///
    /// This covers the currently modelled subset of SQLite expressions:
    ///
    /// - `[schema-name.][table-name.]<column-name>`
    /// - `function-name(function-arguments)`
    ///
    /// Operators are handled by the surrounding Pratt parser; this function consumes only the
    /// identifier-led primary expression before operator binding continues.
    ///
    /// See: https://www.sqlite.org/lang_expr.html
    #[cfg_attr(feature = "trace", trace)]
    fn ident_expr(&mut self) -> Option<nodes::Expr> {
        let location = Location::from(self.cur());
        let Type::Ident(first) = self.cur().ttype.clone() else {
            return None;
        };
        self.advance();

        // function-name(function-arguments)
        if self.consume_if(Type::BraceLeft) {
            return Some(nodes::Expr {
                location,
                literal: None,
                bind: None,
                schema: None,
                table: None,
                column: None,
                function: Some(first),
                operator: None,
                arguments: self.expr_arguments()?,
            });
        }

        let mut parts = vec![first];
        while self.consume_if(Type::Dot) {
            parts.push(self.consume_ident("https://www.sqlite.org/lang_expr.html", "column_name")?);
        }

        if parts.len() > 3 {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Malformed column reference",
                "Expressions support at most schema.table.column references",
                src,
                Rule::Syntax,
                "https://www.sqlite.org/lang_expr.html",
            );
            return None;
        }

        let (schema, table, column) = match parts.as_slice() {
            [column] => (None, None, Some(column.clone())),
            [table, column] => (None, Some(table.clone()), Some(column.clone())),
            [schema, table, column] => (
                Some(schema.clone()),
                Some(table.clone()),
                Some(column.clone()),
            ),
            _ => unreachable!("parts always contains one to three identifiers"),
        };

        Some(nodes::Expr {
            location,
            literal: None,
            bind: None,
            schema,
            table,
            column,
            function: None,
            operator: None,
            arguments: vec![],
        })
    }

    /// Parses `function-arguments` inside an already consumed opening parenthesis.
    ///
    /// See: https://www.sqlite.org/syntax/function-arguments.html
    #[cfg_attr(feature = "trace", trace)]
    fn expr_arguments(&mut self) -> Option<Vec<nodes::Expr>> {
        let mut arguments = vec![];

        if self.consume_if(Type::BraceRight) {
            return Some(arguments);
        }

        loop {
            arguments.push(self.expr()?);

            if self.consume_if(Type::Comma) {
                if self.is(Type::BraceRight) {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed function arguments",
                        "function argument list has a trailing comma",
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/syntax/function-arguments.html",
                    );
                    break;
                }
                continue;
            }

            break;
        }

        self.consume(Type::BraceRight);
        Some(arguments)
    }

    /// parses schema_name.table_name and table_name
    #[cfg_attr(feature = "trace", trace)]
    fn schema_table_container(
        &mut self,
        target_name: Option<&str>,
    ) -> Option<SchemaTableContainer> {
        match self.cur().ttype.clone() {
            // TEMP.<table-name>
            Type::Keyword(Keyword::TEMP) if self.next_is(Type::Dot) => {
                self.advance();
                self.advance();

                let table = match &self.cur().ttype {
                    Type::Ident(table) | Type::String(table) => table.clone(),
                    _ => {
                        self.push_malformed_schema_table_error(target_name);
                        self.advance();
                        return None;
                    }
                };

                self.advance();
                Some(SchemaTableContainer::SchemaAndTable {
                    schema: "temp".into(),
                    table,
                })
            }
            // <schema-name>.<table-name>
            Type::Ident(schema) if self.next_is(Type::Dot) => {
                // skip schema_name
                self.advance();
                // skip Type::Dot
                self.advance();

                let table = match &self.cur().ttype {
                    Type::Ident(table) | Type::String(table) => table.clone(),
                    _ => {
                        self.push_malformed_schema_table_error(target_name);
                        self.advance();
                        return None;
                    }
                };

                // skip table_name
                self.advance();
                Some(SchemaTableContainer::SchemaAndTable { schema, table })
            }
            // <table-name>
            Type::Ident(table_name) | Type::String(table_name) => {
                // skip table_name
                self.advance();
                Some(SchemaTableContainer::Table(table_name))
            }
            _ => {
                let cur = self.cur().clone();
                let target_name = target_name.unwrap_or("table");
                self.push_err(
                    format!("Malformed {} name", target_name),
                    &format!(
                        "expected either schema_name.{} or {}, got {:?}",
                        target_name, target_name, cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                );
                self.advance();
                None
            }
        }
    }

    fn push_malformed_schema_table_error(&mut self, target_name: Option<&str>) {
        let cur = self.cur().clone();
        let target_name = target_name.unwrap_or("table");
        match cur.ttype {
            Type::Keyword(keyword) => {
                let as_str: &str = keyword.into();
                self.push_err(
                    format!("Malformed {target_name} name"),
                    &format!(
                        "`{as_str}` is a keyword, if you want to use it as a {target_name} or column name, quote it: '{as_str}'"
                    ),
                    &cur,
                    Rule::Syntax,
                );
            }
            _ => {
                self.push_err(
                    format!("Malformed {target_name} name"),
                    &format!(
                        "expected a {target_name} name after <schema_name>. - got {:?}",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                );
            }
        }
    }

    /// https://www.sqlite.org/syntax/conflict-clause.html
    #[cfg_attr(feature = "trace", trace)]
    fn conflict_clause(&mut self) -> Option<Keyword> {
        if self.is_keyword(Keyword::ON) {
            self.advance();
            self.consume_keyword(Keyword::CONFLICT);
            if let Type::Keyword(keyword) = &self.cur().ttype {
                match keyword {
                    Keyword::ROLLBACK
                    | Keyword::ABORT
                    | Keyword::FAIL
                    | Keyword::IGNORE
                    | Keyword::REPLACE => {
                        let keyword = *keyword;
                        self.advance();
                        return Some(keyword);
                    }
                    _ => {
                        let mut err = self.err(
                            "Unexpected Keyword",
                            &format!(
                                "Wanted either ROLLBACK, ABORT, FAIL, IGNORE or REPLACE after ON CONFLICT, got {:?}.",
                                self.cur().ttype
                            ),
                            self.cur(),
                            Rule::Syntax,
                        );
                        err.doc_url = Some("https://www.sqlite.org/syntax/conflict-clause.html");
                        self.errors.push(err);
                    }
                }
            } else {
                let mut err = self.err(
                    "Unexpected Keyword",
                    &format!(
                        "Wanted either ROLLBACK, ABORT, FAIL, IGNORE or REPLACE after ON CONFLICT, got {:?}.",
                        self.cur().ttype
                    ),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/syntax/conflict-clause.html");
                self.errors.push(err);
            }
            self.advance();
        }
        None
    }

    /// https://www.sqlite.org/syntax/foreign-key-clause.html but specifically the ON and MATCH paths.
    #[cfg_attr(feature = "trace", trace)]
    fn foreign_key_clause_on_and_match(&mut self, fk: &mut ForeignKeyClause) {
        loop {
            // ON DELETE|UPDATE <foreign-key-action>
            if self.consume_if_keyword(Keyword::ON) {
                let is_delete = match &self.cur().ttype {
                    Type::Keyword(Keyword::DELETE) => true,
                    Type::Keyword(Keyword::UPDATE) => false,
                    _ => {
                        let cur = self.cur().clone();
                        self.push_doc_err(
                            "Unexpected Token",
                            format!("Wanted DELETE or UPDATE, got {:?}.", cur.ttype),
                            &cur,
                            Rule::Syntax,
                            FOREIGN_KEY_DOC,
                        );
                        false
                    }
                };
                self.advance();

                let action = self.foreign_key_action();
                if is_delete {
                    fk.on_delete = action;
                } else {
                    fk.on_update = action;
                }

                continue;
            }

            // MATCH FULL|PARTIAL|SIMPLE
            if self.consume_if_keyword(Keyword::MATCH) {
                fk.match_type = match self.cur().ttype {
                    Type::Keyword(Keyword::FULL) => Some(ForeignKeyMatch::Full),
                    Type::Keyword(Keyword::PARTIAL) => Some(ForeignKeyMatch::Partial),
                    Type::Keyword(Keyword::SIMPLE) => Some(ForeignKeyMatch::Simple),
                    _ => {
                        let cur = self.cur().clone();
                        self.push_doc_err(
                            "Unexpected Keyword",
                            format!(
                                "Wanted FULL, PARTIAL or SIMPLE after MATCH, got {:?}.",
                                cur.ttype
                            ),
                            &cur,
                            Rule::Syntax,
                            FOREIGN_KEY_DOC,
                        );
                        None
                    }
                };
                self.advance();
                continue;
            }

            break;
        }
    }

    #[cfg_attr(feature = "trace", trace)]
    fn foreign_key_action(&mut self) -> Option<ForeignKeyAction> {
        match self.cur().ttype {
            // CASCADE
            Type::Keyword(Keyword::CASCADE) => {
                self.advance();
                Some(ForeignKeyAction::Cascade)
            }
            // RESTRICT
            Type::Keyword(Keyword::RESTRICT) => {
                self.advance();
                Some(ForeignKeyAction::Restrict)
            }
            // NO ACTION
            Type::Keyword(Keyword::NO) => {
                self.advance();
                self.consume_keyword(Keyword::ACTION);
                Some(ForeignKeyAction::NoAction)
            }
            // SET NULL | SET DEFAULT
            Type::Keyword(Keyword::SET) => {
                self.advance();
                if self.consume_if_keyword(Keyword::NULL) {
                    Some(ForeignKeyAction::SetNull)
                } else if self.consume_if_keyword(Keyword::DEFAULT) {
                    Some(ForeignKeyAction::SetDefault)
                } else {
                    let cur = self.cur().clone();
                    self.push_doc_err(
                        "Unexpected Token",
                        format!(
                            "Wanted NULL or DEFAULT after SET in foreign key action, got {:?}.",
                            cur.ttype
                        ),
                        &cur,
                        Rule::Syntax,
                        FOREIGN_KEY_DOC,
                    );
                    self.advance();
                    None
                }
            }
            _ => {
                let cur = self.cur().clone();
                self.push_doc_err(
                    "Unexpected Token",
                    format!(
                        "Wanted SET, CASCADE, RESTRICT or NO after ON DELETE/UPDATE, got {:?}.",
                        cur.ttype
                    ),
                    &cur,
                    Rule::Syntax,
                    FOREIGN_KEY_DOC,
                );
                self.advance();
                None
            }
        }
    }

    /// https://www.sqlite.org/syntax/foreign-key-clause.html and https://sqlite.org/foreignkeys.html
    #[cfg_attr(feature = "trace", trace)]
    fn foreign_key_clause(&mut self) -> Option<ForeignKeyClause> {
        let mut fk = ForeignKeyClause {
            foreign_table: String::new(),
            references_columns: vec![],
            on_delete: None,
            on_update: None,
            match_type: None,
            deferrable: false,
            initially_deferred: false,
        };

        self.consume_keyword(Keyword::REFERENCES);
        fk.foreign_table = self.consume_ident(
            "https://www.sqlite.org/syntax/foreign-key-clause.html",
            "foreign_table",
        )?;

        if self.is(Type::BraceLeft) {
            self.advance();
            loop {
                fk.references_columns.push(self.consume_ident(
                    "https://www.sqlite.org/syntax/foreign-key-clause.html",
                    "column_name",
                )?);

                // if we have a comma, the next token is an identifier
                if self.is(Type::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.consume(Type::BraceRight);
        }

        self.foreign_key_clause_on_and_match(&mut fk);

        // [NOT] DEFERRABLE [INITIALLY DEFERRED|IMMEDIATE]
        if self.is_keyword(Keyword::NOT) || self.is_keyword(Keyword::DEFERRABLE) {
            fk.deferrable = true;
            if self.is_keyword(Keyword::NOT) {
                fk.deferrable = false;
                self.advance();
            }
            self.consume_keyword(Keyword::DEFERRABLE);
            if self.is_keyword(Keyword::INITIALLY) {
                self.advance();
                match &self.cur().ttype {
                    Type::Keyword(Keyword::DEFERRED) => fk.initially_deferred = true,
                    Type::Keyword(Keyword::IMMEDIATE) => (),
                    _ => {
                        let cur = self.cur().clone();
                        self.push_doc_err(
                            "Unexpected Keyword",
                            format!(
                                "Wanted DEFERRED or IMMEDIATE after DEFERRABLE INITIALLY, got {:?}.",
                                cur.ttype
                            ),
                            &cur,
                            Rule::Syntax,
                            FOREIGN_KEY_DOC,
                        );
                    }
                };

                self.advance();
            }

            if !fk.deferrable {
                fk.initially_deferred = false;
            }
        }

        Some(fk)
    }

    #[cfg_attr(feature = "trace", trace)]
    fn parse_column_type(&mut self, def: &mut nodes::ColumnDef) {
        let Type::Ident(name) = self.cur().ttype.clone() else {
            return;
        };

        def.type_name = Some(SqliteStorageClass::from_str(&name));

        if SqliteStorageClass::from_str_strict(name.as_str()).is_none() {
            let mut e = self.err(
                format!("non-canonical SQLite type name `{name}`",),
                &format!("SQLite will assign {} affinity to this column based on it being declared as type {name}. Consider using a canonical sqlite type: TEXT, BLOB, REAL or INTEGER instead.",
                    SqliteStorageClass::from_str(name.as_str())),
                self.cur(),
                Rule::Quirk,
            );
            e.doc_url = Some("https://www.sqlite.org/datatype3.html");
            self.errors.push(e);
        }

        // skip type name
        self.advance();

        if self.is(Type::BraceLeft) {
            self.parse_type_name_parameters();
        }
    }

    #[cfg_attr(feature = "trace", trace)]
    fn parse_type_name_parameters(&mut self) {
        // skip Type::BraceLeft
        self.advance();
        self.consume_type_name_number("Wanted a Number after Type::BraceLeft");

        if self.is(Type::Comma) {
            self.advance();
            self.consume_type_name_number(
                "Wanted a Number after Type::BraceLeft, Type::Number and Type::Comma",
            );
        }
        self.consume(Type::BraceRight);
    }

    #[cfg_attr(feature = "trace", trace)]
    fn consume_type_name_number(&mut self, message: &str) {
        if let Type::Number(_) = self.cur().ttype {
            self.advance();
        } else {
            let mut err = self.err(
                "Unexpected Token",
                &format!("{message}, got {:?}.", self.cur().ttype),
                self.cur(),
                Rule::Syntax,
            );
            err.doc_url = Some("https://www.sqlite.org/syntax/type-name.html");
            self.errors.push(err);
            self.advance();
        }
    }

    fn starts_column_constraint(&mut self) -> bool {
        matches!(
            self.cur().ttype,
            Type::Keyword(Keyword::CONSTRAINT)
                | Type::Keyword(Keyword::PRIMARY)
                | Type::Keyword(Keyword::NOT)
                | Type::Keyword(Keyword::UNIQUE)
                | Type::Keyword(Keyword::CHECK)
                | Type::Keyword(Keyword::DEFAULT)
                | Type::Keyword(Keyword::COLLATE)
                | Type::Keyword(Keyword::REFERENCES)
                | Type::Keyword(Keyword::GENERATED)
                | Type::Keyword(Keyword::AS)
        )
    }

    fn starts_table_constraint(&mut self) -> bool {
        matches!(
            self.cur().ttype,
            Type::Keyword(Keyword::CONSTRAINT)
                | Type::Keyword(Keyword::PRIMARY)
                | Type::Keyword(Keyword::UNIQUE)
                | Type::Keyword(Keyword::CHECK)
                | Type::Keyword(Keyword::FOREIGN)
        )
    }

    /// https://www.sqlite.org/syntax/table-constraint.html
    #[cfg_attr(feature = "trace", trace)]
    fn table_constraint(&mut self) -> Option<nodes::TableConstraint> {
        // CONSTRAINT <name>
        if self.consume_if_keyword(Keyword::CONSTRAINT) {
            self.consume_ident(
                "https://www.sqlite.org/syntax/table-constraint.html",
                "constraint_name",
            )?;
        }

        // PRIMARY KEY (<indexed-column>, ...) [ON CONFLICT <conflict-algorithm>]
        if self.is_keyword(Keyword::PRIMARY) {
            self.advance();
            self.consume_keyword(Keyword::KEY);
            self.consume(Type::BraceLeft);
            let columns = self.indexed_column_list("PRIMARY KEY")?;
            self.consume(Type::BraceRight);
            Some(nodes::TableConstraint::PrimaryKey {
                columns,
                on_conflict: self.conflict_clause(),
            })
        // UNIQUE (<indexed-column>, ...) [ON CONFLICT <conflict-algorithm>]
        } else if self.is_keyword(Keyword::UNIQUE) {
            self.advance();
            self.consume(Type::BraceLeft);
            let columns = self.indexed_column_list("UNIQUE")?;
            self.consume(Type::BraceRight);
            Some(nodes::TableConstraint::Unique {
                columns,
                on_conflict: self.conflict_clause(),
            })
        // CHECK (<expr>)
        } else if self.is_keyword(Keyword::CHECK) {
            self.advance();
            self.consume(Type::BraceLeft);
            let expr = self.expr()?;
            self.consume(Type::BraceRight);
            Some(nodes::TableConstraint::Check(expr))
        // FOREIGN KEY (<column-name>, ...) <foreign-key-clause>
        } else if self.is_keyword(Keyword::FOREIGN) {
            self.advance();
            self.consume_keyword(Keyword::KEY);
            self.consume(Type::BraceLeft);
            let columns = self.column_name_list("FOREIGN KEY")?;
            self.consume(Type::BraceRight);
            Some(nodes::TableConstraint::ForeignKey {
                columns,
                foreign_key_clause: self.foreign_key_clause()?,
            })
        } else {
            let src = Location::from(self.cur());
            self.push_doc_err(
                "Unexpected Token",
                format!(
                    "CREATE TABLE table constraint expected PRIMARY KEY, UNIQUE, CHECK or FOREIGN KEY, got {:?}",
                    self.cur().ttype
                ),
                src,
                Rule::Syntax,
                "https://www.sqlite.org/syntax/table-constraint.html",
            );
            None
        }
    }

    /// https://www.sqlite.org/syntax/indexed-column.html
    #[cfg_attr(feature = "trace", trace)]
    fn indexed_column_list(&mut self, constraint_name: &str) -> Option<Vec<nodes::IndexedColumn>> {
        let mut columns = vec![];
        loop {
            if self.is(Type::BraceRight) {
                if columns.is_empty() {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed table constraint",
                        format!("{constraint_name} requires at least one indexed column"),
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/syntax/table-constraint.html",
                    );
                }
                break;
            }

            columns.push(self.indexed_column()?);

            if self.is(Type::Comma) {
                self.advance();
                if self.is(Type::BraceRight) {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed table constraint",
                        format!("{constraint_name} indexed column list has a trailing comma"),
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/syntax/table-constraint.html",
                    );
                    break;
                }
            } else {
                break;
            }
        }
        Some(columns)
    }

    /// https://www.sqlite.org/syntax/column-name-list.html
    #[cfg_attr(feature = "trace", trace)]
    fn column_name_list(&mut self, constraint_name: &str) -> Option<Vec<String>> {
        let mut columns = vec![];
        loop {
            if self.is(Type::BraceRight) {
                if columns.is_empty() {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed table constraint",
                        format!("{constraint_name} requires at least one column name"),
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/syntax/table-constraint.html",
                    );
                }
                break;
            }

            columns.push(self.consume_ident(
                "https://www.sqlite.org/syntax/column-name-list.html",
                "column_name",
            )?);

            if self.is(Type::Comma) {
                self.advance();
                if self.is(Type::BraceRight) {
                    let src = Location::from(self.cur());
                    self.push_doc_err(
                        "Malformed table constraint",
                        format!("{constraint_name} column list has a trailing comma"),
                        src,
                        Rule::Syntax,
                        "https://www.sqlite.org/syntax/table-constraint.html",
                    );
                    break;
                }
            } else {
                break;
            }
        }
        Some(columns)
    }

    #[cfg_attr(feature = "trace", trace)]
    fn column_constraint(&mut self) -> Option<ColumnConstraint> {
        // CONSTRAINT <name>
        if self.is_keyword(Keyword::CONSTRAINT) {
            self.advance();
            self.consume_ident(
                "https://www.sqlite.org/syntax/column-constraint.html",
                "name",
            );
        }

        // PRIMARY KEY [ASC|DESC] [ON CONFLICT <conflict-algorithm>] [AUTOINCREMENT]
        if self.is_keyword(Keyword::PRIMARY) {
            self.primary_key_column_constraint()
        // NOT NULL [ON CONFLICT <conflict-algorithm>]
        } else if self.is_keyword(Keyword::NOT) {
            self.advance();
            self.consume_keyword(Keyword::NULL);
            Some(ColumnConstraint::NotNull {
                on_conflict: self.conflict_clause(),
            })
        // UNIQUE [ON CONFLICT <conflict-algorithm>]
        } else if self.is_keyword(Keyword::UNIQUE) {
            self.advance();
            Some(ColumnConstraint::Unique {
                on_conflict: self.conflict_clause(),
            })
        // CHECK (<expr>)
        } else if self.is_keyword(Keyword::CHECK) {
            self.advance();
            self.consume(Type::BraceLeft);
            let e = self.expr()?;
            self.consume(Type::BraceRight);
            Some(ColumnConstraint::Check(e))
        // DEFAULT <literal-value> | DEFAULT (<expr>)
        } else if self.is_keyword(Keyword::DEFAULT) {
            self.default_column_constraint()
        // COLLATE <collation-name>
        } else if self.is_keyword(Keyword::COLLATE) {
            self.advance();
            Some(ColumnConstraint::Collate(self.consume_ident(
                "https://www.sqlite.org/syntax/column-constraint.html",
                "collation_name",
            )?))
        // <foreign-key-clause>
        } else if self.is_keyword(Keyword::REFERENCES) {
            Some(ColumnConstraint::ForeignKey(self.foreign_key_clause()?))
        // [GENERATED ALWAYS] AS (<expr>) [STORED|VIRTUAL]
        } else if self.is_keyword(Keyword::GENERATED) || self.is_keyword(Keyword::AS) {
            self.generated_column_constraint()
        } else {
            None
        }
    }

    #[cfg_attr(feature = "trace", trace)]
    fn primary_key_column_constraint(&mut self) -> Option<ColumnConstraint> {
        self.advance();
        self.consume_keyword(Keyword::KEY);
        let asc_desc = self.consume_optional_keyword(&[Keyword::ASC, Keyword::DESC]);
        let on_conflict = self.conflict_clause();
        let autoincrement = self
            .consume_optional_keyword(&[Keyword::AUTOINCREMENT])
            .is_some();

        Some(ColumnConstraint::PrimaryKey {
            asc_desc,
            on_conflict,
            autoincrement,
        })
    }

    #[cfg_attr(feature = "trace", trace)]
    fn default_column_constraint(&mut self) -> Option<ColumnConstraint> {
        self.advance();
        if self.is(Type::BraceLeft) {
            self.advance();
            let expr = self.expr();
            self.consume(Type::BraceRight);
            Some(ColumnConstraint::Default {
                literal: None,
                expr,
            })
        } else {
            let lit = self.literal_value();
            Some(ColumnConstraint::Default {
                literal: lit,
                expr: None,
            })
        }
    }

    #[cfg_attr(feature = "trace", trace)]
    fn generated_column_constraint(&mut self) -> Option<ColumnConstraint> {
        let mut is_generated = false;
        if self.is_keyword(Keyword::GENERATED) {
            is_generated = true;
            self.advance();
            self.consume_keyword(Keyword::ALWAYS);
        }

        self.consume_keyword(Keyword::AS);
        self.consume(Type::BraceLeft);
        let expr = self.expr().unwrap();
        self.consume(Type::BraceRight);

        let stored_virtual = self.consume_optional_keyword(&[Keyword::STORED, Keyword::VIRTUAL]);

        if is_generated {
            Some(ColumnConstraint::Generated {
                expr,
                stored_virtual,
            })
        } else {
            Some(ColumnConstraint::As {
                expr,
                stored_virtual,
            })
        }
    }

    fn consume_optional_keyword(&mut self, keywords: &[Keyword]) -> Option<Keyword> {
        let Type::Keyword(keyword) = self.cur().ttype else {
            return None;
        };

        if keywords.contains(&keyword) {
            self.advance();
            Some(keyword)
        } else {
            None
        }
    }

    /// https://www.sqlite.org/syntax/column-def.html
    #[cfg_attr(feature = "trace", trace)]
    fn column_def(&mut self) -> Option<nodes::ColumnDef> {
        let mut def = nodes::ColumnDef {
            location: Location::from(self.cur()),
            name: String::new(),
            type_name: None,
            constraints: vec![],
        };

        def.name = self.consume_ident("https://www.sqlite.org/syntax/column-def.html", "name")?;
        self.parse_column_type(&mut def);

        while !self.is_eof() && self.starts_column_constraint() {
            if let Some(constraint) = self.column_constraint() {
                def.constraints.push(constraint);
            }
        }

        Some(def)
    }
}
