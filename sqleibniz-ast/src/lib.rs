//! SQLite syntax primitives, lexer, parser, and abstract syntax tree.

use std::fmt;

pub mod error;
pub mod highlight;
pub mod lev;
pub mod lexer;
pub mod parser;
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
