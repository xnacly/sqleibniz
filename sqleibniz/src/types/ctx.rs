use std::collections::HashSet;

use super::{Type, storage::SqliteStorageClass};

pub struct Table {
    pub name: String,
    pub columns: Vec<SqliteStorageClass>,
}

/// Context holds information necessary for the analysis of sql statements.
pub struct Context {
    pub tables: Vec<Table>,
    pub save_points: HashSet<String>,
    pub databases: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct HookContext {
    pub node: String,
    /// [Self::kind] holds the token kind for token-backed contexts and the node name otherwise.
    pub kind: String,
    /// [Self::content] holds the textual representation of token-backed contexts.
    pub content: Option<String>,
    pub line: usize,
    pub start: usize,
    pub finish: usize,
    pub children: Vec<HookContext>,
}

impl HookContext {
    pub fn node(node: impl Into<String>, line: usize, start: usize, finish: usize) -> Self {
        let node = node.into();
        Self {
            kind: node.clone(),
            node,
            content: None,
            line,
            start,
            finish,
            children: vec![],
        }
    }

    pub fn token(token: &super::Token) -> Self {
        let (kind, content) = match &token.ttype {
            Type::Keyword(keyword) => {
                let keyword: &str = (*keyword).into();
                ("Keyword", Some(keyword.to_string()))
            }
            Type::Ident(ident) => ("Ident", Some(ident.clone())),
            Type::Number(number) => ("Number", Some(number.to_string())),
            Type::String(string) => ("String", Some(string.clone())),
            Type::Blob(bytes) => ("Blob", Some(format!("{bytes:x?}"))),
            Type::Boolean(boolean) => ("Boolean", Some(boolean.to_string())),
            Type::ParamName(name) => ("ParamName", Some(name.clone())),
            Type::Param(param) => ("Param", Some(param.to_string())),
            Type::Dot => ("Dot", Some(".".into())),
            Type::Asterisk => ("Asterisk", Some("*".into())),
            Type::Semicolon => ("Semicolon", Some(";".into())),
            Type::Percent => ("Percent", Some("%".into())),
            Type::Comma => ("Comma", Some(",".into())),
            Type::Equal => ("Equal", Some("=".into())),
            Type::Question => ("Question", Some("?".into())),
            Type::Colon => ("Colon", Some(":".into())),
            Type::At => ("At", Some("@".into())),
            Type::Dollar => ("Dollar", Some("$".into())),
            Type::BraceLeft => ("BraceLeft", Some("(".into())),
            Type::BraceRight => ("BraceRight", Some(")".into())),
            Type::BracketLeft => ("BracketLeft", Some("[".into())),
            Type::BracketRight => ("BracketRight", Some("]".into())),
            Type::InstructionExpect => ("InstructionExpect", None),
            Type::Eof => ("Eof", None),
        };

        Self {
            node: "Token".into(),
            kind: kind.into(),
            content,
            line: token.line,
            start: token.start,
            finish: token.end,
            children: vec![],
        }
    }
}

impl mlua::IntoLua for HookContext {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;
        table.set("node", self.node)?;
        table.set("kind", self.kind)?;
        let content = self.content.unwrap_or_default();
        table.set("content", content.clone())?;
        table.set("text", content)?;
        table.set("line", self.line)?;
        table.set("start", self.start)?;
        table.set("finish", self.finish)?;
        table.set("children", self.children)?;
        lua.pack(table)
    }
}
