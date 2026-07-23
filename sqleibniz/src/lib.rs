/// analyse contains AST-driven diagnostics that run after parsing
pub mod analyse;
/// error does formatting and highlighting for errors
pub use sqleibniz_ast::error;
/// explain provides metadata for rules and supported SQL statements
pub mod explain;
/// highlight implements logic for highlighting tokens found in a string
pub use sqleibniz_ast::highlight;
/// hooks executes user-defined lua hooks over AST and token contexts
pub mod hooks;
/// lexer converts the input into a stream of token for the parser
pub use sqleibniz_ast::lexer;
/// lsp implements the language server protocol to provide diagnostics, suggestions and snippets for sql based on the sqleibniz tooling
pub mod lsp;
/// parser converts the token stream into an abstract syntax tree
pub use sqleibniz_ast::parser;
/// sarif converts diagnostics into Static Analysis Results Interchange Format logs
pub mod sarif;
/// types holds all shared types between the above modules
pub mod types;
