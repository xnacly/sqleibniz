use nodes::{BindParameter, SchemaTableContainer};

#[cfg(feature = "trace")]
use proc::trace;

use crate::{
    error::{Error, ImprovedLine},
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

/// wrap argument in Some(Box::new(_))
macro_rules! some_box {
    ($expr:expr) => {
        Some(Box::new($expr) as Box<dyn nodes::Node>)
    };
}

/// Function naming directly corresponds to the sqlite3 documentation of sql syntax.
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

    fn err(&self, msg: impl Into<String>, note: &str, start: &Token, rule: Rule) -> Error {
        Error {
            improved_line: None,
            file: self.name.to_string(),
            line: start.line,
            rule,
            note: note.into(),
            msg: msg.into(),
            start: start.start,
            end: start.end,
            doc_url: None,
        }
    }

    fn push_err(&mut self, msg: impl Into<String>, note: &str, start: &Token, rule: Rule) {
        let err = self.err(msg, note, start, rule);
        self.errors.push(err);
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

    fn next_is(&self, t: Type) -> bool {
        self.tokens
            .get(self.pos + 1)
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

    #[cfg_attr(feature = "trace", trace)]
    pub fn parse(&mut self) -> Vec<Box<dyn nodes::Node>> {
        self.sql_stmt_list()
    }

    /// see: https://www.sqlite.org/syntax/sql-stmt-list.html
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

    #[cfg_attr(feature = "trace", trace)]
    fn sql_stmt_prefix(&mut self) -> Option<Box<dyn nodes::Node>> {
        let r: Option<Box<dyn nodes::Node>> = match self.cur().ttype {
            Type::Keyword(Keyword::EXPLAIN) => {
                let t = self.cur().clone();
                // skip EXPLAIN
                self.advance();

                // path for EXPLAIN->QUERY->PLAN
                if self.is(Type::Keyword(Keyword::QUERY)) {
                    self.advance();
                    self.consume(Type::Keyword(Keyword::PLAN));
                }

                // else path is EXPLAIN->*_stmt
                some_box!(nodes::Explain {
                    t,
                    child: self.sql_stmt()?,
                })
            }
            _ => self.sql_stmt(),
        };

        r
    }

    /// see: https://www.sqlite.org/syntax/sql-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn sql_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        match self.cur().ttype {
            // TODO: add new statement starts here
            Type::Keyword(Keyword::PRAGMA) => self.pragma_stmt(),
            Type::Keyword(Keyword::ALTER) => self.alter_stmt(),
            Type::Keyword(Keyword::ATTACH) => self.attach_stmt(),
            Type::Keyword(Keyword::REINDEX) => self.reindex_stmt(),
            Type::Keyword(Keyword::RELEASE) => self.release_stmt(),
            Type::Keyword(Keyword::SAVEPOINT) => self.savepoint_stmt(),
            Type::Keyword(Keyword::DROP) => self.drop_stmt(),
            Type::Keyword(Keyword::ANALYZE) => self.analyse_stmt(),
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

    // TODO: add new statement function here *_stmt()
    // #[cfg_attr(feature = "trace", trace)]
    // fn $1_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
    //
    //
    // }

    /// https://www.sqlite.org/lang_createindex.html
    #[cfg_attr(feature = "trace", trace)]
    fn create_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        todo!("Parser::create_stmt");
    }

    /// https://www.sqlite.org/pragma.html
    #[cfg_attr(feature = "trace", trace)]
    fn pragma_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let t = self.cur().clone();

        // skip PRAGMA
        self.advance();

        // PRAGMA needs a target name
        let Some(schema_and_pragma) = self.schema_table_container(Some("pragma")) else {
            return None;
        };

        let pragma = if self.is(Type::Semicolon) {
            Pragma {
                t,
                name: schema_and_pragma,
                invocation: nodes::PragmaInvocation::Query,
            }
        } else if self.is(Type::Equal) {
            self.advance();
            match self.cur().ttype {
                Type::String(_) | Type::Number(_) | Type::Ident(_) | Type::Keyword(_) => {}
                _ => {
                    let cur = self.cur().clone();
                    self.push_err("Bad pragma value", &format!("A pragmas assignment value has to be either String, Number, Ident or a Keyword, got {:?} instead", cur.ttype), &cur, Rule::Syntax,);
                    self.advance();
                }
            }
            let p = Pragma {
                t,
                name: schema_and_pragma,
                invocation: nodes::PragmaInvocation::Assign {
                    value: self.cur().clone(),
                },
            };
            self.advance();
            p
        } else if self.is(Type::BraceLeft) {
            self.advance();
            match self.cur().ttype {
                Type::String(_) | Type::Number(_) | Type::Ident(_) | Type::Keyword(_) => {}
                _ => {
                    let cur = self.cur().clone();
                    self.push_err("Bad pragma value", &format!("A pragmas call value has to be either String, Number, Ident or a Keyword, got {:?} instead", cur.ttype), &cur, Rule::Syntax,);
                    self.advance();
                }
            }
            let p = Pragma {
                t,
                name: schema_and_pragma,
                invocation: nodes::PragmaInvocation::Call {
                    value: self.cur().clone(),
                },
            };
            self.advance();
            self.consume(Type::BraceRight);
            p
        } else {
            let cur = self.cur().clone();
            self.push_err(
                "Bad pragma value",
                &format!(
                    "A pragmas rhs value has to be either an assignment via '=', a call via '(<arg>)' or simply be a query, got {:?} instead",
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
            t: self.cur().clone(),
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
            Type::Keyword(Keyword::ADD) => {
                self.advance();
                if self.is(Type::Keyword(Keyword::COLUMN)) {
                    self.advance();
                }

                a.add_column = self.column_def();
            }
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
            t: self.cur().clone(),
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
        let t = self.cur().clone();
        // skipping ATTACH
        self.advance();
        // skipping optional DATABASE
        if self.is(Type::Keyword(Keyword::DATABASE)) {
            self.advance();
        }

        let mut a = nodes::Attach {
            t,
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
            t: self.cur().clone(),
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
            t: self.cur().clone(),
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
        let t = self.cur().clone();
        self.advance();

        match self.cur().ttype {
            Type::Keyword(Keyword::INDEX) => (),
            Type::Keyword(Keyword::TABLE) => (),
            Type::Keyword(Keyword::TRIGGER) => (),
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

        // skip either INDEX;TABLE;TRIGGER or VIEW
        self.advance();

        let if_exists = if self.is(Type::Keyword(Keyword::IF)) {
            self.advance();
            self.consume(Type::Keyword(Keyword::EXISTS));
            true
        } else {
            false
        };

        let argument = self.schema_table_container(None)?;

        some_box!(nodes::Drop {
            t,
            ttype,
            if_exists,
            argument,
        })
    }

    /// https://www.sqlite.org/syntax/analyze-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn analyse_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut a = nodes::Analyze {
            t: self.cur().clone(),
            target: None,
        };

        self.advance();

        // inlined Parser::schema_table_container
        a.target = match self.cur().ttype.clone() {
            Type::Ident(schema) if self.next_is(Type::Dot) => {
                self.advance();
                self.advance();
                if let Type::Ident(table) = &self.cur().ttype {
                    let table = table.clone();
                    self.advance();
                    Some(SchemaTableContainer::SchemaAndTable { schema, table })
                } else if let Type::String(table) = &self.cur().ttype {
                    let table = table.clone();
                    self.advance();
                    Some(SchemaTableContainer::SchemaAndTable { schema, table })
                } else {
                    let cur = self.cur().clone();
                    self.errors.push(match cur.ttype {
                        Type::Keyword(keyword) => {
                            let as_str: &str = keyword.into();
                            self.err(
                            "Malformed table name",
                            &format!("`{as_str}` is a keyword, if you want to use it as a table or column name, quote it: '{as_str}'"),
                            &cur, Rule::Syntax)
                        }
                        _ => self.err(
                            "Malformed table name",
                            &format!(
                                "expected a table name after <schema_name>. - got {:?}",
                                cur.ttype
                            ),
                            &cur,
                            Rule::Syntax,
                        ),
                    });

                    // skip wrong token, should I skip to the next statement via
                    // self.skip_until_semicolon_or_eof?
                    self.advance();
                    None
                }
            }
            Type::Ident(table_name) | Type::String(table_name) => {
                // skip table_name
                self.advance();
                Some(SchemaTableContainer::Table(table_name))
            }
            _ => None,
        };

        self.expect_end("https://www.sqlite.org/lang_analyze.html");

        some_box!(a)
    }

    /// https://www.sqlite.org/syntax/detach-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn detach_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let t = self.cur().clone();
        self.advance();

        // skip optional DATABASE path
        if self.is(Type::Keyword(Keyword::DATABASE)) {
            self.advance();
        }

        let schema_name =
            self.consume_ident("https://www.sqlite.org/lang_detach.html", "schema_name")?;

        let d = nodes::Detach { t, schema_name };

        self.expect_end("https://www.sqlite.org/lang_detach.html");

        some_box!(d)
    }

    /// https://www.sqlite.org/syntax/rollback-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn rollback_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut rollback = nodes::Rollback {
            t: self.cur().clone(),
            save_point: None,
        };
        self.advance();

        match self.cur().ttype {
            Type::Keyword(Keyword::TRANSACTION) | Type::Keyword(Keyword::TO) | Type::Semicolon => {}
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "ROLLBACK requires TRANSACTION, TO or to end at this point, got {:?}",
                        self.cur().ttype
                    ),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                self.errors.push(err);
            }
        }

        // optional TRANSACTION
        if self.is(Type::Keyword(Keyword::TRANSACTION)) {
            self.advance();
        }

        // optional TO
        if self.is(Type::Keyword(Keyword::TO)) {
            self.advance();

            // optional SAVEPOINT
            if self.is(Type::Keyword(Keyword::SAVEPOINT)) {
                self.advance();
            }

            match self.cur().ttype {
                Type::Keyword(Keyword::SAVEPOINT) | Type::Ident(_) | Type::Semicolon => {}
                _ => {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "ROLLBACK requires SAVEPOINT, Ident or to end at this point, got {:?}",
                            self.cur().ttype
                        ),
                        self.cur(),
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                    self.errors.push(err);
                    self.advance();
                }
            }

            if let Type::Ident(str) = &self.cur().ttype {
                rollback.save_point = Some(String::from(str));
            } else {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "ROLLBACK wants Ident as <savepoint-name>, got {:?}",
                        self.cur().ttype
                    ),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                self.errors.push(err);
            }
            self.advance();
        }

        self.expect_end("https://www.sqlite.org/lang_transaction.html");

        some_box!(rollback)
    }

    /// https://www.sqlite.org/syntax/commit-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn commit_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let commit: Option<Box<dyn nodes::Node>> = some_box!(nodes::Commit {
            t: self.cur().clone(),
        });

        // skip either COMMIT or END
        self.advance();

        match self.cur().ttype {
            // expected end 1
            Type::Semicolon => (),
            // expected end 2, optional
            Type::Keyword(Keyword::TRANSACTION) => self.advance(),
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted Keyword(TRANSACTION) or Semicolon, got {:?}",
                        self.cur().ttype
                    ),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                self.errors.push(err);
                self.advance();
            }
        }

        self.expect_end("https://www.sqlite.org/lang_transaction.html");

        commit
    }

    /// https://www.sqlite.org/syntax/begin-stmt.html
    #[cfg_attr(feature = "trace", trace)]
    fn begin_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut begin: nodes::Begin = nodes::Begin {
            t: self.cur().clone(),
            transaction_kind: None,
        };

        // skip BEGIN
        self.advance();

        // skip modifiers
        match self.cur().ttype {
            // BEGIN;
            Type::Semicolon => return some_box!(begin),
            Type::Keyword(Keyword::DEFERRED)
            | Type::Keyword(Keyword::IMMEDIATE)
            | Type::Keyword(Keyword::EXCLUSIVE) => {
                begin.transaction_kind = if let Type::Keyword(word) = &self.cur().ttype {
                    Some(*word)
                } else {
                    None
                };
                self.advance()
            }
            _ => {}
        }

        match self.cur().ttype {
            Type::Semicolon => return some_box!(begin),
            // ending
            Type::Keyword(Keyword::TRANSACTION) => self.advance(),
            Type::Keyword(Keyword::DEFERRED)
            | Type::Keyword(Keyword::IMMEDIATE)
            | Type::Keyword(Keyword::EXCLUSIVE) => {
                let mut err = self.err(
                    "Unexpected Token",
                    "BEGIN does not allow multiple transaction behaviour modifiers",
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                self.errors.push(err);
                // TODO: think about if this is smart at this point, skipping to the next ; could
                // be skipping too many tokens
                self.skip_until_semicolon_or_eof();
            }
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted any of TRANSACTION, DEFERRED, IMMEDIATE or EXCLUSIVE before this point, got {:?}",
                        self.cur().ttype
                    ),
                    self.cur(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                self.errors.push(err);
            }
        }

        self.expect_end("https://www.sqlite.org/lang_transaction.html");

        some_box!(begin)
    }

    /// https://www.sqlite.org/lang_vacuum.html
    #[cfg_attr(feature = "trace", trace)]
    fn vacuum_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut v = nodes::Vacuum {
            t: self.cur().clone(),
            schema_name: None,
            filename: None,
        };
        self.consume(Type::Keyword(Keyword::VACUUM));

        match self.cur().ttype {
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

        // first path
        if let Type::Semicolon = self.cur().ttype {
            return some_box!(v);
        }

        // if schema_name is specified
        if let Type::Ident(_) = self.cur().ttype {
            v.schema_name = Some(self.cur().clone());
            self.advance(); // skip schema_name
        }

        // if INTO keyword is given is specified
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
    fn literal_value(&mut self) -> Option<Box<dyn nodes::Node>> {
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
                let s: Option<Box<dyn nodes::Node>> = some_box!(nodes::Literal { t: cur.clone() });
                // skipping over the current character
                self.advance();
                s
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
    fn expr(&mut self) -> Option<nodes::Expr> {
        let mut e = nodes::Expr {
            t: self.cur().clone(),
            literal: None,
            bind: None,
            schema: None,
            table: None,
            column: None,
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
                e.literal = self.literal_value().map(|e| e.token().clone())
            }
            // bind parameter with optional ident: ?[ident]
            Type::Question => {
                // sqlite documentation says: But because it is easy to miscount the question marks, the
                // use of this parameter format is discouraged. Programmers are encouraged to use
                // one of the symbolic formats [...] or the ?NNN format [...] instead.
                let mut param = BindParameter {
                    t: self.cur().clone(),

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
                    param.counter = self.literal_value();
                }
                e.bind = Some(param)
            }
            // bind parameter with required ident: [:@$]<ident>
            Type::Colon | Type::At | Type::Dollar => {
                let mut bind = BindParameter {
                    t: self.cur().clone(),
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
                            bind.t.ttype
                        ),
                        &bind.t,
                        Rule::Syntax,
                    );
                    // skip invalid token
                    self.advance();
                    return None;
                }
                e.bind = Some(bind);
            }
            Type::Ident(_) => {
                // this is the start of a function
                if self.next_is(Type::BraceLeft) {
                    todo!("function-name(function-arguments) [filter-clause] [over-clause]")
                }

                // this sets either the schema, the table or the column
                todo!("[schema-name.][table-name.]<column-name>");
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

    /// parses schema_name.table_name and table_name
    #[cfg_attr(feature = "trace", trace)]
    fn schema_table_container(
        &mut self,
        target_name: Option<&str>,
    ) -> Option<SchemaTableContainer> {
        match self.cur().ttype.clone() {
            Type::Ident(schema) if self.next_is(Type::Dot) => {
                // skip schema_name
                self.advance();
                // skip Type::Dot
                self.advance();
                if let Type::Ident(table) = &self.cur().ttype {
                    let table = table.clone();
                    // skip table_name
                    self.advance();
                    Some(SchemaTableContainer::SchemaAndTable { schema, table })
                } else if let Type::String(table) = &self.cur().ttype {
                    let table = table.clone();
                    // skip table_name
                    self.advance();
                    Some(SchemaTableContainer::SchemaAndTable { schema, table })
                } else {
                    // we got schema_name. but not Ident|String following? this is a syntax error
                    let cur = self.cur().clone();
                    self.errors.push(match cur.ttype {
                        Type::Keyword(keyword) => {
                let target_name = target_name.unwrap_or_else(|| "table");
                            let as_str: &str = keyword.into();
                            self.err(
                            &format!("Malformed {target_name} name"),
                            &format!("`{as_str}` is a keyword, if you want to use it as a {target_name} or column name, quote it: '{as_str}'"),
                            &cur, Rule::Syntax)
                        }
                        _ => {
                let target_name = target_name.unwrap_or_else(|| "table");
                            self.err(
                                                    &format!("Malformed {target_name} name"),
                                                    &format!(
                                                        "expected a {target_name} name after <schema_name>. - got {:?}",
                                                        cur.ttype
                                                    ),
                                                    &cur,
                                                    Rule::Syntax,
                                                )
                        },
                    });

                    // skip wrong token, should I skip to the next statement via
                    // self.skip_until_semicolon_or_eof?
                    self.advance();
                    None
                }
            }
            Type::Ident(table_name) | Type::String(table_name) => {
                // skip table_name
                self.advance();
                Some(SchemaTableContainer::Table(table_name))
            }
            _ => {
                let cur = self.cur().clone();
                let target_name = target_name.unwrap_or_else(|| "table");
                self.push_err(
                    &format!("Malformed {} name", target_name),
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

    /// https://www.sqlite.org/syntax/foreign-key-clause.html but specifically the ON and MATCH
    /// paths, necessary because the end of the block moves back to the state machine states ON and
    /// MATCH
    #[cfg_attr(feature = "trace", trace)]
    fn foreign_key_clause_on_and_match(&mut self, fk: &mut ForeignKeyClause) -> Option<()> {
        let mut is_delete = false;
        if self.is_keyword(Keyword::ON) {
            self.advance();

            match &self.cur().ttype {
                Type::Keyword(Keyword::DELETE) => is_delete = true,
                Type::Keyword(Keyword::UPDATE) => (),
                _ => {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!("Wanted DELETE or UPDATE, got {:?}.", self.cur().ttype),
                        self.cur(),
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                    self.errors.push(err);
                }
            };

            self.advance();

            let action = match self.cur().ttype {
                Type::Keyword(Keyword::CASCADE) => {
                    self.advance();
                    Some(ForeignKeyAction::Cascade)
                }
                Type::Keyword(Keyword::RESTRICT) => {
                    self.advance();
                    Some(ForeignKeyAction::Restrict)
                }
                Type::Keyword(Keyword::NO) => {
                    self.advance();
                    self.consume_keyword(Keyword::ACTION);
                    Some(ForeignKeyAction::NoAction)
                }
                Type::Keyword(Keyword::SET) => {
                    self.advance();
                    let a = Some(if self.is_keyword(Keyword::NULL) {
                        ForeignKeyAction::SetNull
                    } else {
                        self.consume_keyword(Keyword::DEFAULT);
                        ForeignKeyAction::SetDefault
                    });
                    self.advance();
                    a
                }
                _ => {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "Wanted SET, CASCADE, RESTRICT or NO after ON DELETE/UPDATE, got {:?}.",
                            self.cur().ttype
                        ),
                        self.cur(),
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                    self.errors.push(err);
                    self.advance();
                    None
                }
            };

            if is_delete {
                fk.on_delete = action;
            } else {
                fk.on_update = action;
            }

            self.foreign_key_clause_on_and_match(fk)
        } else if self.is_keyword(Keyword::MATCH) {
            self.advance();
            fk.match_type = match self.cur().ttype {
                Type::Keyword(Keyword::FULL) => Some(ForeignKeyMatch::Full),
                Type::Keyword(Keyword::PARTIAL) => Some(ForeignKeyMatch::Partial),
                Type::Keyword(Keyword::SIMPLE) => Some(ForeignKeyMatch::Simple),
                _ => todo!("error handling MATCH <kind>"),
            };
            self.advance();
            self.foreign_key_clause_on_and_match(fk)
        } else {
            None
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
                        let mut err = self.err(
                        "Unexpected Keyword",
                        &format!(
                            "Wanted DEFERRED or IMMEDIATE after DEFERRABLE INITIALLY, got {:?}.",
                            self.cur().ttype
                        ),
                        self.cur(),
                        Rule::Syntax,
                    );
                        err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                        self.errors.push(err);
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

    /// https://www.sqlite.org/syntax/column-def.html
    #[cfg_attr(feature = "trace", trace)]
    fn column_def(&mut self) -> Option<nodes::ColumnDef> {
        let mut def = nodes::ColumnDef {
            t: self.cur().clone(),
            name: String::new(),
            type_name: None,
            constraints: vec![],
        };

        def.name = self.consume_ident("https://www.sqlite.org/syntax/column-def.html", "name")?;

        // we got a type_name: https://www.sqlite.org/syntax/type-name.html
        if let Type::Ident(name) = &self.cur().ttype {
            def.type_name = Some(SqliteStorageClass::from_str(name));

            if SqliteStorageClass::from_str_strict(name.as_str()).is_none() {
                let mut e = self.err(
                    format!("Type `{name}` is not a sqlite type and thus will be of type INTEGER"),
                    "Consider using a known sqlite type: TEXT, BLOB, REAL or INTEGER",
                    self.cur(),
                    Rule::Quirk,
                );
                e.doc_url = Some("https://www.sqlite.org/datatype3.html");
                self.errors.push(e);
            }

            // skip type name
            self.advance();

            if self.is(Type::BraceLeft) {
                // skip Type::BraceLeft
                self.advance();
                if let Type::Number(_) = self.cur().ttype {
                    self.advance();
                } else {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "Wanted a Number after Type::BraceLeft, got {:?}.",
                            self.cur().ttype
                        ),
                        self.cur(),
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/type-name.html");
                    self.errors.push(err);
                    self.advance();
                }

                if self.is(Type::Comma) {
                    self.advance();
                    if let Type::Number(_) = self.cur().ttype {
                        self.advance();
                    } else {
                        let mut err = self.err(
                            "Unexpected Token",
                            &format!(
                                "Wanted a Number after Type::BraceLeft, Type::Number and Type::Comma, got {:?}.",
                                self.cur().ttype
                            ),
                            self.cur(),
                            Rule::Syntax,
                        );
                        err.doc_url = Some("https://www.sqlite.org/syntax/type-name.html");
                        self.errors.push(err);
                        self.advance();
                    }
                }
                self.consume(Type::BraceRight);
            }
        } else {
            let tok = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .unwrap_or_else(|| self.cur());

            let err = Error {
                improved_line: None,
                file: self.name.to_string(),
                line: tok.line,
                rule: Rule::Quirk,
                note: "SQLite allows columns without a declared type. Such columns use dynamic typing and type affinity is not enforced. Consider adding TEXT, BLOB, REAL, or INTEGER if this is unintended.".into(),
                msg: "Possibly unintended flexible typed column".into(),
                start: tok.start,
                end: tok.end,
                doc_url: Some("https://www.sqlite.org/quirks.html#the_datatype_is_optional"),
            };
            self.errors.push(err);
        }

        // column_constraint: https://www.sqlite.org/syntax/column-constraint.html
        while !self.is_eof()
            && matches!(
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
        {
            if self.is_keyword(Keyword::CONSTRAINT) {
                self.advance();
                self.consume_ident(
                    "https://www.sqlite.org/syntax/column-constraint.html",
                    "name",
                );
            }

            let constraint = if self.is_keyword(Keyword::PRIMARY) {
                self.advance();
                self.consume_keyword(Keyword::KEY);
                let asc_desc =
                    if let Type::Keyword(k @ (Keyword::ASC | Keyword::DESC)) = &self.cur().ttype {
                        let k = *k;
                        self.advance();
                        Some(k)
                    } else {
                        None
                    };

                let on_conflict = self.conflict_clause();
                let autoincrement = if self.is_keyword(Keyword::AUTOINCREMENT) {
                    self.advance();
                    true
                } else {
                    false
                };

                Some(ColumnConstraint::PrimaryKey {
                    asc_desc,
                    on_conflict,
                    autoincrement,
                })
            } else if self.is_keyword(Keyword::NOT) {
                self.advance();
                self.consume_keyword(Keyword::NULL);
                Some(ColumnConstraint::NotNull {
                    on_conflict: self.conflict_clause(),
                })
            } else if self.is_keyword(Keyword::UNIQUE) {
                self.advance();
                Some(ColumnConstraint::Unique {
                    on_conflict: self.conflict_clause(),
                })
            } else if self.is_keyword(Keyword::CHECK) {
                self.advance();
                self.consume(Type::BraceLeft);
                let e = self.expr()?;
                self.consume(Type::BraceRight);
                Some(ColumnConstraint::Check(e))
            } else if self.is_keyword(Keyword::DEFAULT) {
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
                    // this aint so pretty, but sometimes i do need literals as Option<Box<dyn
                    // Box>> and sometimes as Option<Literal>, it is what it is, Nodes sadly dont
                    // care about my feelings :(
                    let lit = self.literal_value();
                    Some(ColumnConstraint::Default {
                        literal: lit.map(|n| nodes::Literal {
                            t: n.token().clone(),
                        }),
                        expr: None,
                    })
                }
            } else if self.is_keyword(Keyword::COLLATE) {
                self.advance();
                Some(ColumnConstraint::Collate(self.consume_ident(
                    "https://www.sqlite.org/syntax/column-constraint.html",
                    "collation_name",
                )?))
            } else if self.is_keyword(Keyword::REFERENCES) {
                Some(ColumnConstraint::ForeignKey(self.foreign_key_clause()?))
            } else if self.is_keyword(Keyword::GENERATED) || self.is_keyword(Keyword::AS) {
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

                let stored_virtual =
                    if let Type::Keyword(k @ (Keyword::STORED | Keyword::VIRTUAL)) =
                        &self.cur().ttype
                    {
                        let k = *k;
                        self.advance();
                        Some(k)
                    } else {
                        None
                    };

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
            } else {
                None
            };

            if let Some(constraint) = constraint {
                def.constraints.push(constraint);
            }
        }

        Some(def)
    }
}
