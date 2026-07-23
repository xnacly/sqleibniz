//! Compatibility re-exports for syntax types now owned by `sqleibniz-ast`.

pub mod config;

pub mod ctx;

pub mod rules {
    pub use sqleibniz_ast::types::rules::*;
}

pub mod storage {
    pub use sqleibniz_ast::types::storage::*;
}

pub use sqleibniz_ast::types::{Keyword, Token, Type};
