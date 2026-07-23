use sqleibniz_ast::{error, lexer, parser};

/// analyse contains AST-driven diagnostics that run after parsing
pub mod analyse;
/// explain provides metadata for rules and supported SQL statements
pub mod explain;
/// hooks executes user-defined lua hooks over AST and token contexts
pub mod hooks;
/// lsp implements the language server protocol to provide diagnostics, suggestions and snippets for sql based on the sqleibniz tooling
pub mod lsp;
/// sarif converts diagnostics into Static Analysis Results Interchange Format logs
pub mod sarif;
/// types holds all shared types between the above modules
pub mod types;
