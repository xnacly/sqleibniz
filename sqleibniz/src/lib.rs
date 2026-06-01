/// error does formatting and highlighting for errors
pub mod error;
/// highlight implements logic for highlighting tokens found in a string
pub mod highlight;
/// lev implements the levenshtein distance for all sql keywords, this is used to recommend a keyword based on a misspelled word or any
/// unknown keyword at an arbitrary location in the source statement - mainly used at the start of a new statement
pub mod lev;
/// lexer converts the input into a stream of token for the parser
pub mod lexer;
/// lsp implements the language server protocol to provide diagnostics, suggestions and snippets for sql based on the sqleibniz tooling
pub mod lsp;
/// parser converts the token stream into an abstract syntax tree
pub mod parser;
/// sarif converts diagnostics into Static Analysis Results Interchange Format logs
pub mod sarif;
/// types holds all shared types between the above modules
pub mod types;
