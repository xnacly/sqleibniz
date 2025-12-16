use nodes::{BindParameter, SchemaTableContainer};

use proc::trace;

use crate::{
    error::{Error, ImprovedLine},
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

    fn cur(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn err(&self, msg: &str, note: &str, start: &Token, rule: Rule) -> Error {
        Error {
            improved_line: None,
            file: self.name.to_string(),
            line: start.line,
            rule,
            note: note.into(),
            msg: msg.into(),
            start: start.start,
            end: self.cur().map_or(start.start, |tok| tok.end),
            doc_url: None,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn advance(&mut self) {
        if !self.is_eof() {
            self.pos += 1
        }
    }

    fn is(&self, t: Type) -> bool {
        self.cur().is_some_and(|tok| tok.ttype == t)
    }

    fn is_keyword(&self, keyword: Keyword) -> bool {
        self.cur()
            .map_or(false, |tok| tok.ttype == Type::Keyword(keyword))
    }

    fn skip_until_semicolon_or_eof(&mut self) {
        while !self.is_eof() && !self.is(Type::Semicolon) {
            self.advance();
        }
    }

    /// if current token in t advance, otherwise return false; finally advance
    fn matches_any(&mut self, t: Vec<Type>) -> Option<Token> {
        if let Some(cur) = &self.cur() {
            if t.contains(&cur.ttype) {
                let t = (*cur).clone();
                self.advance();
                return Some(t);
            }
            return None;
        }
        None
    }

    /// if current token in t advance, otherwise return false and push error; finally advance
    fn matches_one(&mut self, t: Vec<Type>) -> bool {
        if let Some(cur) = &self.cur() {
            if !t.contains(&cur.ttype) {
                self.errors.push(self.err(
                    "Unexpected Token",
                    &format!("Wanted any of {:?}, got {:?}", t, cur.ttype),
                    cur,
                    Rule::Syntax,
                ));
                return false;
            }
            self.advance();
            return true;
        }
        false
    }

    /// checks if type of current token is equal to t, otherwise pushs an error, advances either way
    fn consume(&mut self, t: Type) {
        let tt = t.clone();
        if !self.is(tt) {
            let cur = match self.cur() {
                None => {
                    let last = self.tokens.get(self.pos - 1).unwrap();
                    &Token {
                        ttype: Type::Eof,
                        start: last.end,
                        end: last.end,
                        line: last.line,
                    }
                }
                Some(c) => c,
            };
            let mut err = self.err(
                match cur.ttype {
                    Type::Eof => "Unexpected End of input",
                    _ => "Unexpected Token",
                },
                &format!("Wanted {:?}, got {:?}", t, cur.ttype),
                cur,
                Rule::Syntax,
            );
            if t == Type::Semicolon {
                err.msg = "Missing semicolon".into();
                err.note.push_str(", terminate statements with ';'");
                err.rule = Rule::Semicolon;
                err.improved_line = Some(ImprovedLine {
                    snippet: ";",
                    start: cur.end,
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
            .map_or(false, |tok| tok.ttype == t)
    }

    /// checks if current token is not semicolon, if it isnt pushes an error
    fn expect_end(&mut self, doc: &'static str) -> Option<()> {
        if !self.is(Type::Semicolon) {
            let mut err = self.err(
                "Unexpected Statement Continuation",
                &format!(
                    "End of statement via Semicolon expected, got {:?}",
                    self.cur()?.ttype
                ),
                self.cur()?,
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
        if let Type::Ident(ident) = &self.cur()?.ttype {
            let i = ident.clone();
            self.advance();
            Some(i)
        } else {
            let mut err = self.err(
                "Unexpected Token",
                &format!(
                    "Expected Ident(<{}>), got {:?}",
                    expected_ident_name,
                    self.cur()?.ttype
                ),
                self.cur()?,
                Rule::Syntax,
            );
            err.doc_url = Some(doc);
            self.errors.push(err);
            self.skip_until_semicolon_or_eof();
            None
        }
    }

    #[trace]
    pub fn parse(&mut self) -> Vec<Option<Box<dyn nodes::Node>>> {
        let r = self.sql_stmt_list();
        r
    }

    /// see: https://www.sqlite.org/syntax/sql-stmt-list.html
    #[trace]
    fn sql_stmt_list(&mut self) -> Vec<Option<Box<dyn nodes::Node>>> {
        let mut r = vec![];
        while !self.is_eof() {
            if let Some(Token {
                ttype: Type::InstructionExpect,
                ..
            }) = self.cur()
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
            let stmt = self.sql_stmt_prefix();
            if stmt.is_some() {
                r.push(stmt);
            }
            self.consume(Type::Semicolon);
        }
        r
    }

    #[trace]
    fn sql_stmt_prefix(&mut self) -> Option<Box<dyn nodes::Node>> {
        let r: Option<Box<dyn nodes::Node>> = match self.cur()?.ttype {
            Type::Keyword(Keyword::EXPLAIN) => {
                let t = self.cur()?.clone();
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
    #[trace]
    fn sql_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let r = match self.cur()?.ttype {
            // TODO: add new statement starts here
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
                self.errors.push(self.err(
                    "Unexpected Token",
                    "Semicolon makes no sense at this point",
                    self.cur()?,
                    Rule::Syntax,
                ));
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
                    &format!("Literal {:?} disallowed at this point.", self.cur()?.ttype),
                    self.cur()?,
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/syntax/sql-stmt.html");
                self.errors.push(err);
                self.advance();
                None
            }
            Type::Ident(_) => {
                if let Type::Ident(name) = &self.cur()?.ttype {
                    let suggestions = Keyword::suggestions(name);
                    if !suggestions.is_empty() {
                        self.errors.push(self.err(
                            "Unknown Keyword",
                            &format!(
                                "'{}' is not a known keyword, did you mean: \n\t- {}",
                                name,
                                suggestions.join("\n\t- ").as_str()
                            ),
                            self.cur()?,
                            Rule::UnknownKeyword,
                        ));
                    } else {
                        self.errors.push(self.err(
                            "Unknown Keyword",
                            &format!("'{name}' is not a known keyword"),
                            self.cur()?,
                            Rule::UnknownKeyword,
                        ));
                    }
                }
                self.skip_until_semicolon_or_eof();
                None
            }
            Type::Keyword(_) => {
                self.errors.push(self.err(
                    "Unimplemented",
                    &format!(
                        "sqleibniz can not yet analyse the token {:?}, skipping ahead to next statement",
                        self.cur()?.ttype
                    ),
                    self.cur()?,
                    Rule::Unimplemented,
                ));
                self.skip_until_semicolon_or_eof();
                None
            }
            _ => {
                self.errors.push(self.err(
                    "Unknown Token",
                    &format!(
                        "sqleibniz does not understand the token {:?}, skipping ahead to next statement",
                        self.cur()?.ttype
                    ),
                    self.cur()?,
                    Rule::Unimplemented,
                ));
                self.skip_until_semicolon_or_eof();
                None
            }
        };

        r
    }

    // TODO: add new statement function here *_stmt()
    // #[trace]
    // fn $1_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
    //
    //
    // }

    /// https://www.sqlite.org/lang_createindex.html
    #[trace]
    fn create_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        None
    }

    /// https://www.sqlite.org/lang_altertable.html
    #[trace]
    fn alter_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut a = nodes::Alter {
            t: self.cur()?.clone(),
            target: SchemaTableContainer::Table(String::new()),
            rename_to: None,
            rename_column_target: None,
            new_column_name: None,
            add_column: None,
            drop_column: None,
        };

        self.advance();
        self.consume(Type::Keyword(Keyword::TABLE));

        a.target =
            match self.schema_table_container_ok("https://www.sqlite.org/lang_altertable.html") {
                Ok(container) => container,
                Err(err) => {
                    self.errors.push(err);
                    a.target
                }
            };

        match self.cur()?.ttype {
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
                        self.cur()?.ttype
                    ),
                    self.cur()?,
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
    #[trace]
    fn reindex_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut r = nodes::Reindex {
            t: self.cur()?.clone(),
            target: None,
        };
        self.advance();

        // REINDEX has a path with no further nodes
        if self.is(Type::Semicolon) {
            return some_box!(r);
        }

        r.target = self.schema_table_container();

        self.expect_end("https://www.sqlite.org/syntax/reindex-stmt.html");

        some_box!(r)
    }

    /// https://www.sqlite.org/syntax/attach-stmt.html
    #[trace]
    fn attach_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let t = self.cur()?.clone();
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
    #[trace]
    fn release_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut r = nodes::Release {
            t: self.cur()?.clone(),
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
    #[trace]
    fn savepoint_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut s = nodes::Savepoint {
            t: self.cur()?.clone(),
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
    #[trace]
    fn drop_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut drop = nodes::Drop {
            t: self.cur()?.clone(),
            if_exists: false,
            // dummy value
            ttype: Keyword::NULL,
            argument: String::new(),
        };
        self.advance();

        match self.cur()?.ttype {
            Type::Keyword(Keyword::INDEX) => (),
            Type::Keyword(Keyword::TABLE) => (),
            Type::Keyword(Keyword::TRIGGER) => (),
            Type::Keyword(Keyword::VIEW) => (),
            _ => {
                let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "DROP requires either TRIGGER, TABLE, TRIGGER or VIEW at this point, got {:?}",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
                err.doc_url = Some("https://www.sqlite.org/lang.html");
                self.errors.push(err);
                self.advance();
                return None;
            }
        }

        // we checked if the keyword is valid above
        if let Type::Keyword(keyword) = &self.cur()?.ttype {
            drop.ttype = keyword.clone();
        }

        // skip either INDEX;TABLE;TRIGGER or VIEW
        self.advance();

        if self.is(Type::Keyword(Keyword::IF)) {
            self.advance();
            self.consume(Type::Keyword(Keyword::EXISTS));
            drop.if_exists = true;
        }

        if let Type::Ident(schema_name) = self.cur()?.ttype.clone() {
            // table/index/view/trigger of a schema_name
            drop.argument.push_str(&schema_name);
            if self.next_is(Type::Dot) {
                // skip Type::Ident from above
                self.advance();
                // skip Type::Dot
                self.advance();
                if let Type::Ident(index_trigger_table_view) = self.cur()?.ttype.clone() {
                    drop.argument.push('.');
                    drop.argument.push_str(&index_trigger_table_view);
                } else {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "DROP requires Ident(<index_or_trigger_or_table_or_view>) after Dot and Ident(<schema_name>), got {:?}",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
                    err.doc_url = Some(
                        "https://www.sqlite.org/lang_dropview.html https://www.sqlite.org/lang_droptrigger.html https://www.sqlite.org/lang_droptable.html https://www.sqlite.org/lang_dropindex.html",
                    );
                    self.advance();
                    self.errors.push(err);
                }
            }
            self.advance();
        } else {
            let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "DROP requires Ident(<index_or_trigger_or_table_or_view>) or Ident(<schema_name>).Ident(<index_or_trigger_or_table_or_view>), got {:?}",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
            err.doc_url = Some(
                "https://www.sqlite.org/lang_dropview.html https://www.sqlite.org/lang_droptrigger.html https://www.sqlite.org/lang_droptable.html https://www.sqlite.org/lang_dropindex.html",
            );
            self.errors.push(err);
            return None;
        }

        some_box!(drop)
    }

    /// https://www.sqlite.org/syntax/analyze-stmt.html
    #[trace]
    fn analyse_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut a = nodes::Analyze {
            t: self.cur()?.clone(),
            target: None,
        };

        self.advance();
        a.target = self.schema_table_container();

        self.expect_end("https://www.sqlite.org/lang_analyze.html");

        some_box!(a)
    }

    /// https://www.sqlite.org/syntax/detach-stmt.html
    #[trace]
    fn detach_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let t = self.cur()?.clone();
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
    #[trace]
    fn rollback_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut rollback = nodes::Rollback {
            t: self.cur()?.clone(),
            save_point: None,
        };
        self.advance();

        match self.cur()?.ttype {
            Type::Keyword(Keyword::TRANSACTION) | Type::Keyword(Keyword::TO) | Type::Semicolon => {}
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "ROLLBACK requires TRANSACTION, TO or to end at this point, got {:?}",
                        self.cur()?.ttype
                    ),
                    self.cur()?,
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

            match self.cur()?.ttype {
                Type::Keyword(Keyword::SAVEPOINT) | Type::Ident(_) | Type::Semicolon => {}
                _ => {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "ROLLBACK requires SAVEPOINT, Ident or to end at this point, got {:?}",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/lang_transaction.html");
                    self.errors.push(err);
                    self.advance();
                }
            }

            if let Type::Ident(str) = &self.cur()?.ttype {
                rollback.save_point = Some(String::from(str));
            } else {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "ROLLBACK wants Ident as <savepoint-name>, got {:?}",
                        self.cur()?.ttype
                    ),
                    self.cur()?,
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
    #[trace]
    fn commit_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let commit: Option<Box<dyn nodes::Node>> = some_box!(nodes::Commit {
            t: self.cur()?.clone(),
        });

        // skip either COMMIT or END
        self.advance();

        match self.cur()?.ttype {
            // expected end 1
            Type::Semicolon => (),
            // expected end 2, optional
            Type::Keyword(Keyword::TRANSACTION) => self.advance(),
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted Keyword(TRANSACTION) or Semicolon, got {:?}",
                        self.cur()?.ttype
                    ),
                    self.cur()?,
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
    #[trace]
    fn begin_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut begin: nodes::Begin = nodes::Begin {
            t: self.cur()?.clone(),
            transaction_kind: None,
        };

        // skip BEGIN
        self.advance();

        // skip modifiers
        let ct = &self.cur()?.ttype;
        match ct {
            // only BEGIN
            Type::Semicolon => return some_box!(begin),
            Type::Keyword(Keyword::DEFERRED)
            | Type::Keyword(Keyword::IMMEDIATE)
            | Type::Keyword(Keyword::EXCLUSIVE) => {
                begin.transaction_kind = if let Type::Keyword(word) = ct {
                    Some(word.clone())
                } else {
                    None
                };
                self.advance()
            }
            _ => {}
        }

        match self.cur()?.ttype {
            Type::Semicolon => return some_box!(begin),
            // ending
            Type::Keyword(Keyword::TRANSACTION) => self.advance(),
            Type::Keyword(Keyword::DEFERRED)
            | Type::Keyword(Keyword::IMMEDIATE)
            | Type::Keyword(Keyword::EXCLUSIVE) => {
                let mut err = self.err(
                    "Unexpected Token",
                    "BEGIN does not allow multiple transaction behaviour modifiers",
                    self.cur()?,
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
                        self.cur()?.ttype
                    ),
                    self.cur()?,
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
    #[trace]
    fn vacuum_stmt(&mut self) -> Option<Box<dyn nodes::Node>> {
        let mut v = nodes::Vacuum {
            t: self.cur()?.clone(),
            schema_name: None,
            filename: None,
        };
        self.consume(Type::Keyword(Keyword::VACUUM));

        match self.cur()?.ttype {
            Type::Semicolon | Type::Ident(_) | Type::Keyword(Keyword::INTO) => {}
            _ => {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted {:?} with {:?} or {:?} for VACUUM stmt, got {:?}",
                        Type::Keyword(Keyword::INTO),
                        Type::String("<filename>".to_string()),
                        Type::Ident("<schema_name>".to_string()),
                        self.cur()?.ttype.clone()
                    ),
                    &self.cur()?.clone(),
                    Rule::Syntax,
                );
                err.doc_url = Some("https://www.sqlite.org/lang_vacuum.html");
                self.errors.push(err);
                self.advance(); // skip error_token
            }
        }

        // first path
        if let Type::Semicolon = self.cur()?.ttype {
            return some_box!(v);
        }

        // if schema_name is specified
        if let Type::Ident(_) = self.cur()?.ttype {
            v.schema_name = Some(self.cur()?.clone());
            self.advance(); // skip schema_name
        }

        // if INTO keyword is given is specified
        if let Type::Keyword(Keyword::INTO) = self.cur()?.ttype {
            self.advance(); // skip INTO
            if let Type::String(_) = self.cur()?.ttype {
                v.filename = Some(self.cur()?.clone());
            } else {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted {:?} for VACUUM stmt with {:?}, got {:?}",
                        Type::String("<filename>".to_string()),
                        Type::Keyword(Keyword::INTO),
                        self.cur()?.ttype.clone()
                    ),
                    &self.cur()?.clone(),
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
    #[trace]
    fn literal_value(&mut self) -> Option<Box<dyn nodes::Node>> {
        let cur = self.cur()?;
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
            t: self.cur()?.clone(),
            literal: None,
            bind: None,
            schema: None,
            table: None,
            column: None,
        };
        match self.cur()?.ttype {
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
                    t: self.cur()?.clone(),

                    counter: None,
                    name: None,
                };
                self.advance();

                // question mark can have a number after them, but they are optional
                if let Some(Token {
                    ttype: Type::Number(_),
                    ..
                }) = self.cur()
                {
                    param.counter = self.literal_value();
                }
                e.bind = Some(param)
            }
            // bind parameter with required ident: [:@$]<ident>
            Type::Colon | Type::At | Type::Dollar => {
                let mut bind = BindParameter {
                    t: self.cur()?.clone(),
                    counter: None,
                    name: None,
                };
                self.advance();

                // all bind params need an identifier, because they need to be named
                if let Some(Token {
                    ttype: Type::Ident(ident),
                    ..
                }) = self.cur()
                {
                    bind.name = Some(ident.clone());
                    self.advance();
                } else {
                    self.errors.push(self.err(
                        "Invalid bind parameter",
                        &format!(
                            "Bind parameter with {:?} requires an identifier as a postfix",
                            bind.t.ttype
                        ),
                        &bind.t,
                        Rule::Syntax,
                    ));
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
                let t = self.cur()?;
                self.errors.push(self.err(
                    "Invalid construct",
                    &format!(
                        "At this point in an expression, {:?} is not a valid construct",
                        t.ttype
                    ),
                    t,
                    Rule::Syntax,
                ));
                self.advance();
                return None;
            }
        }
        Some(e)
    }

    /// wraps [Parser::schema_table_container], returns a Result containing an Error if [Parser::schema_table_container] returns Option::None
    #[trace]
    fn schema_table_container_ok(
        &mut self,
        doc: &'static str,
    ) -> Result<SchemaTableContainer, Error> {
        match self.schema_table_container() {
            Some(t) => Ok(t),
            None => {
                let cur = match self.cur() {
                    Some(cur) => cur,
                    None => match self.tokens.get(self.pos - 1) {
                        Some(prev) => prev,
                        None => panic!(
                            "Parser::tokens::get(Parser::pos-1) is Option::None at Parser::schema_table_container_ok(), this should not happen"
                        ),
                    },
                };
                let mut err = self.err(
                    "Missing schema_name or table_name",
                    &format!(
                        "expected either Ident(<schema_name.table_name>) or Ident(<table_name>) at this point, got {:?}",
                        cur.ttype
                    ),
                    cur,
                    Rule::Syntax,
                );
                err.doc_url = Some(doc);
                Err(err)
            }
        }
    }

    /// parses schema_name.table_name and table_name, this does only emit errors for syntax issues, otherwise, use [Parser::schema_table_container_ok]
    #[trace]
    fn schema_table_container(&mut self) -> Option<SchemaTableContainer> {
        match self.cur()?.ttype.clone() {
            Type::Ident(schema) if self.next_is(Type::Dot) => {
                // skip schema_name
                self.advance();
                // skip Type::Dot
                self.advance();
                if let Type::Ident(table) = self.cur()?.ttype.clone() {
                    // skip table_name
                    self.advance();
                    Some(SchemaTableContainer::SchemaAndTable { schema, table })
                } else {
                    // we got schema_name. but no identifier following? this is a syntax error (i
                    // think?)
                    self.errors.push(self.err(
                        "Missing table_name",
                        &format!(
                            "expected a Ident(<table_name>) after getting Ident(<schema_name>) and Type::Dot ('.'), got {:?}",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    ));
                    // skip wrong token
                    self.advance();
                    None
                }
            }
            Type::Ident(table_name) => {
                // skip table_name
                self.advance();
                Some(SchemaTableContainer::Table(table_name))
            }
            _ => None,
        }
    }

    /// https://www.sqlite.org/syntax/conflict-clause.html
    #[trace]
    fn conflict_clause(&mut self) -> Option<()> {
        if self.is_keyword(Keyword::ON) {
            self.advance();
            self.consume_keyword(Keyword::CONFLICT);
            if let Type::Keyword(keyword) = &self.cur()?.ttype {
                match keyword {
                    Keyword::ROLLBACK
                    | Keyword::ABORT
                    | Keyword::FAIL
                    | Keyword::IGNORE
                    | Keyword::REPLACE => (),
                    _ => {
                        let mut err = self.err(
                            "Unexpected Keyword",
                            &format!(
                                "Wanted either ROLLBACK, ABORT, FAIL, IGNORE or REPLACE after ON CONFLICT, got {:?}.",
                                self.cur()?.ttype
                            ),
                            self.cur()?,
                            Rule::Syntax,
                        );
                        err.doc_url = Some("https://www.sqlite.org/syntax/conflict-clause.html");
                        self.errors.push(err);
                    }
                }
            } else {
                let mut err = self.err(
                    "Unexpected Token",
                    &format!(
                        "Wanted a Keyword at this point, got {:?}.",
                        self.cur()?.ttype
                    ),
                    self.cur()?,
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
    /// blocks
    #[trace]
    fn foreign_key_clause_on_and_match(&mut self) -> Option<()> {
        if self.is_keyword(Keyword::ON) {
            self.advance();
            match self.cur()?.ttype {
                Type::Keyword(Keyword::DELETE) | Type::Keyword(Keyword::UPDATE) => (),
                _ => {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!("Wanted DELETE or UPDATE, got {:?}.", self.cur()?.ttype),
                        self.cur()?,
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                    self.errors.push(err);
                }
            };
            self.advance();
            match &self.cur()?.ttype {
                Type::Keyword(keyword) => match keyword {
                    Keyword::SET => {
                        self.advance();
                        if !(self.is_keyword(Keyword::NULL) || self.is_keyword(Keyword::DEFAULT)) {
                            let mut err = self.err(
                                "Unexpected Token",
                                &format!(
                                    "Wanted NULL or DEFAULT after SET, got {:?}.",
                                    self.cur()?.ttype
                                ),
                                self.cur()?,
                                Rule::Syntax,
                            );
                            err.doc_url =
                                Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                            self.errors.push(err);
                        }
                        self.advance();
                    }
                    Keyword::CASCADE | Keyword::RESTRICT => self.advance(),
                    Keyword::NO => {
                        self.advance();
                        self.consume_keyword(Keyword::ACTION);
                    }
                    _ => {
                        let mut err = self.err(
                            "Unexpected Token",
                            &format!(
                                "Wanted SET, CASCADE, RESTRICT or NO after ON DELETE/UPDATE, got {:?}.",
                                self.cur()?.ttype
                            ),
                            self.cur()?,
                            Rule::Syntax,
                        );
                        err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                        self.errors.push(err);
                        self.advance();
                    }
                },
                _ => {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "Wanted SET, CASCADE, RESTRICT or NO after ON DELETE/UPDATE, got {:?}.",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                    self.errors.push(err);
                    self.advance();
                }
            }
            self.foreign_key_clause_on_and_match()
        } else if self.is_keyword(Keyword::MATCH) {
            self.advance();
            self.consume_ident(
                "https://www.sqlite.org/syntax/foreign-key-clause.html",
                "name",
            );
            self.foreign_key_clause_on_and_match()
        } else {
            None
        }
    }

    /// https://www.sqlite.org/syntax/foreign-key-clause.html
    #[trace]
    fn foreign_key_clause(&mut self) -> Option<()> {
        self.consume_keyword(Keyword::REFERENCES);
        self.consume_ident(
            "https://www.sqlite.org/syntax/foreign-key-clause.html",
            "foreign_table",
        );

        if self.is(Type::BraceLeft) {
            self.advance();
            loop {
                self.consume_ident(
                    "https://www.sqlite.org/syntax/foreign-key-clause.html",
                    "column_name",
                );
                // if next token is an identifier, we require a comma
                if let Type::Ident(_) = self.tokens.get(self.pos + 1)?.ttype {
                    self.consume(Type::Comma);
                } else {
                    break;
                }
            }
            self.consume(Type::BraceRight);
        }

        self.foreign_key_clause_on_and_match();

        if self.is_keyword(Keyword::NOT) || self.is_keyword(Keyword::DEFERRABLE) {
            if self.is_keyword(Keyword::NOT) {
                self.advance();
            }
            self.consume_keyword(Keyword::DEFERRABLE);
            if self.is_keyword(Keyword::INITIALLY) {
                self.advance();
                if !(self.is_keyword(Keyword::DEFERRED) || self.is_keyword(Keyword::IMMEDIATE)) {
                    let mut err = self.err(
                        "Unexpected Keyword",
                        &format!(
                            "Wanted DEFERRED or IMMEDIATE after DEFERRABLE INITIALLY, got {:?}.",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/foreign-key-clause.html");
                    self.errors.push(err);
                }
                self.advance();
            }
            None
        } else {
            None
        }
    }

    /// https://www.sqlite.org/syntax/column-def.html
    #[trace]
    fn column_def(&mut self) -> Option<nodes::ColumnDef> {
        let mut def = nodes::ColumnDef {
            t: self.cur()?.clone(),
            name: String::new(),
            type_name: None,
        };

        def.name = self.consume_ident("https://www.sqlite.org/syntax/column-def.html", "name")?;

        // we got a type_name: https://www.sqlite.org/syntax/type-name.html
        while let Type::Ident(name) = &self.cur()?.ttype {
            def.type_name = Some(SqliteStorageClass::from_str(name));
            // skip ident
            self.advance();
            if self.is(Type::BraceLeft) {
                // skip Type::BraceLeft
                self.advance();
                if let Type::Number(_) = self.cur()?.ttype {
                    self.advance();
                } else {
                    let mut err = self.err(
                        "Unexpected Token",
                        &format!(
                            "Wanted a Number after Type::BraceLeft, got {:?}.",
                            self.cur()?.ttype
                        ),
                        self.cur()?,
                        Rule::Syntax,
                    );
                    err.doc_url = Some("https://www.sqlite.org/syntax/type-name.html");
                    self.errors.push(err);
                    self.advance();
                }

                if self.is(Type::Comma) {
                    self.advance();
                    if let Type::Number(_) = self.cur()?.ttype {
                        self.advance();
                    } else {
                        let mut err = self.err(
                            "Unexpected Token",
                            &format!(
                                "Wanted a Number after Type::BraceLeft, Type::Number and Type::Comma, got {:?}.",
                                self.cur()?.ttype
                            ),
                            self.cur()?,
                            Rule::Syntax,
                        );
                        err.doc_url = Some("https://www.sqlite.org/syntax/type-name.html");
                        self.errors.push(err);
                        self.advance();
                    }
                }
                self.consume(Type::BraceRight);
            }
        }

        // column_constraint: https://www.sqlite.org/syntax/column-constraint.html
        while !self.is_eof()
            && matches!(
                self.cur()?.ttype,
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
                // info
                self.consume_ident(
                    "https://www.sqlite.org/syntax/column-constraint.html",
                    "name",
                );
            }

            if self.is_keyword(Keyword::PRIMARY) {
                self.advance();
                self.consume_keyword(Keyword::KEY);
                if self.is_keyword(Keyword::ASC) || self.is_keyword(Keyword::DESC) {
                    self.advance();
                }
                self.conflict_clause();
                if self.is_keyword(Keyword::AUTOINCREMENT) {
                    self.advance();
                }
            } else if self.is_keyword(Keyword::NOT) {
                self.advance();
                self.consume_keyword(Keyword::NULL);
                self.conflict_clause();
            } else if self.is_keyword(Keyword::UNIQUE) {
                self.advance();
                self.conflict_clause();
            } else if self.is_keyword(Keyword::CHECK) {
                self.advance();
                self.consume(Type::BraceLeft);
                self.expr();
                self.consume(Type::BraceRight);
            } else if self.is_keyword(Keyword::DEFAULT) {
                self.advance();
                if self.is(Type::BraceLeft) {
                    self.advance();
                    self.expr();
                    self.consume(Type::BraceRight);
                } else {
                    self.literal_value();
                }
            } else if self.is_keyword(Keyword::COLLATE) {
                self.advance();
                self.consume_ident(
                    "https://www.sqlite.org/syntax/column-constraint.html",
                    "collation_name",
                );
            } else if self.is_keyword(Keyword::REFERENCES) {
                self.foreign_key_clause();
            } else if self.is_keyword(Keyword::GENERATED) || self.is_keyword(Keyword::AS) {
                if self.is_keyword(Keyword::GENERATED) {
                    self.advance();
                    self.consume_keyword(Keyword::ALWAYS);
                }
                self.consume_keyword(Keyword::AS);
                self.consume(Type::BraceLeft);
                self.expr();
                self.consume(Type::BraceRight);
                if self.is_keyword(Keyword::STORED) || self.is_keyword(Keyword::VIRTUAL) {
                    self.advance();
                }
            }
        }

        Some(def)
    }
}
