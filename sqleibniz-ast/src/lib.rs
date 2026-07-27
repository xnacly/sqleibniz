//! sqleibniz-ast lexes and parses the sql syntax sqlite understands into tokens, an abstract
//! syntax tree and recoverable diagnostics. It is the syntax layer of sqleibniz, without the
//! diagnostics, lua hooks and language server sqleibniz adds on top.
//!
//! [parse] does the lexing and parsing in one step and is the entry point for the usual case:
//!
//! ```
//! let parsed = sqleibniz_ast::parse("SELECT id FROM users;", "query.sql");
//!
//! assert!(parsed.is_ok());
//! assert_eq!(parsed.ast.len(), 1);
//! ```
//!
//! Lexing and parsing recover where they can, thus [ParseResult::ast] can hold statements even
//! while [ParseResult::errors] holds diagnostics. Editors and other tools that have to work with
//! incomplete sql can therefore keep the syntax the parser did understand:
//!
//! ```
//! let parsed = sqleibniz_ast::parse("SELECT id FROM users; SELECT FROM", "query.sql");
//!
//! assert_eq!(parsed.ast.len(), 1);
//! assert!(!parsed.errors.is_empty());
//! ```
//!
//! Statements are [Node] implementations behind `Box<dyn Node>`, see [parser::nodes] for the node
//! types and [Node::as_any] for getting from a node back to its concrete type. Enable the `serde`
//! feature to serialise nodes and tokens.
//!
//! For lower level control, drive [Lexer] and [Parser] yourself, as [parse] does.
//!
//! ## See:
//!
//! - <https://www.sqlite.org/lang.html>
//! - <https://github.com/xnacly/sqleibniz>
#![warn(missing_docs)]

use std::fmt;

/// error holds the diagnostics the lexer and parser emit, their source locations and their
/// terminal rendering
pub mod error;
/// highlight performs syntax highlighting of sql source lines for terminal output
pub mod highlight;
/// lev computes the Levenshtein distance used to suggest keywords for misspelled input
pub mod lev;
/// lexer turns sql source into the token stream the parser consumes
pub mod lexer;
/// parser turns a token stream into the abstract syntax tree, see [parser::nodes] for its nodes
pub mod parser;
/// types holds the tokens, keywords, storage classes and diagnostic rules shared between lexer and
/// parser
pub mod types;

pub use error::{Error, ImprovedLine, Location};
pub use lexer::Lexer;
pub use parser::Parser;
pub use parser::nodes::Node;
pub use types::{Keyword, Token, Type};

/// The complete result of parsing one SQL source file.
///
/// Parsing is recoverable: [`ParseResult::ast`] may contain nodes even when
/// [`ParseResult::errors`] contains lexer or parser diagnostics.
pub struct ParseResult {
    /// Tokens produced by the lexer.
    pub tokens: Vec<Token>,
    /// Parsed SQL statements.
    pub ast: Vec<Box<dyn Node>>,
    /// Diagnostics reported by the lexer and parser.
    pub errors: Vec<Error>,
}

impl ParseResult {
    /// Returns whether lexing and parsing completed without diagnostics.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseResult")
            .field("tokens", &self.tokens)
            .field("ast", &self.ast)
            .field("errors", &self.errors)
            .finish()
    }
}

/// Lexes and parses SQL in one step.
///
/// The returned [`ParseResult`] preserves tokens and recoverable diagnostics
/// alongside the AST, so callers can choose whether to reject malformed input
/// or work with the successfully parsed portion.
pub fn parse(source: impl AsRef<[u8]>, file_name: impl AsRef<str>) -> ParseResult {
    let source = source.as_ref().to_vec();
    let file_name = file_name.as_ref();
    let mut lexer = Lexer::new(&source, file_name);
    let tokens = lexer.run();
    let mut errors = lexer.errors;

    let ast = if tokens.is_empty() {
        Vec::new()
    } else {
        let mut parser = Parser::new(tokens.clone(), file_name);
        let ast = parser.parse();
        errors.extend(parser.errors);
        ast
    };

    ParseResult {
        tokens,
        ast,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parse_returns_the_tokens_ast_and_diagnostics() {
        let parsed = parse("SELECT 1;", "example.sql");

        assert!(parsed.is_ok());
        assert_eq!(parsed.tokens.len(), 3);
        assert_eq!(parsed.ast.len(), 1);
    }
}
