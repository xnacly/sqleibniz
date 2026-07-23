#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize)]
/// Rule is attached to each diagnostic and can be disabled from the CLI or leibniz.lua.
#[derive(clap::ValueEnum)]
pub enum Rule {
    /// Source file is empty
    #[value(name = "file/no-content")]
    NoContent,
    /// Source file is not empty but holds no statements
    #[value(name = "file/no-statements")]
    NoStatements,
    /// Source file contains constructs sqleibniz does not yet understand
    #[value(name = "sqleibniz/unimplemented")]
    Unimplemented,
    /// Source file contains an unknown keyword
    #[value(name = "sql/unknown-keyword")]
    UnknownKeyword,
    /// Source file contains invalid sqleibniz instruction
    #[value(name = "sqleibniz/bad-instruction")]
    BadSqleibnizInstruction,
    /// User-defined Lua hook reported a diagnostic
    #[value(name = "sqleibniz/hook")]
    Hook,
    /// Source file uses sql features sqlite does not support
    #[value(name = "sqlite/unsupported")]
    SqliteUnsupported,
    /// Source file uses a PRAGMA not documented by SQLite
    #[value(name = "sqlite/unknown-pragma")]
    UnknownPragma,
    /// Source file defines the same table-like object more than once
    #[value(name = "sqlite/duplicate-relation")]
    DuplicateRelation,
    /// Source file references a table-like object not defined earlier in the file
    #[value(name = "sqlite/unknown-relation")]
    UnknownRelation,
    /// Source file references a column not defined on a known table
    #[value(name = "sqlite/unknown-column")]
    UnknownColumn,
    /// Sqlite or SQL quirk: https://www.sqlite.org/quirks.html; anything where SQLite deviates
    /// from a stricter, conventional SQL model
    #[value(name = "sqlite/quirk")]
    Quirk,
    /// Source file contains an unterminated string
    #[value(name = "sql/unterminated-string")]
    UnterminatedString,
    /// The source file contains an unknown character
    #[value(name = "sql/unknown-character")]
    UnknownCharacter,
    /// The source file contains an invalid numeric literal, either overflow or incorrect syntax
    #[value(name = "sql/invalid-numeric-literal")]
    InvalidNumericLiteral,
    /// The source file contains an invalid blob literal, either bad hex data (a-f,A-F,0-9) or
    /// incorrect syntax
    #[value(name = "sql/invalid-blob")]
    InvalidBlob,
    /// The source file contains a structure with incorrect syntax
    #[value(name = "sql/syntax")]
    Syntax,
    /// The source file is missing a semicolon
    #[value(name = "sql/missing-semicolon")]
    Semicolon,
}

impl mlua::UserData for Rule {}

pub struct RuleExample {
    pub sql: &'static str,
    pub explanation: &'static str,
}

impl mlua::FromLua for Rule {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let value: String = lua.unpack(value)?;

        Ok(match value.as_str() {
            "file/no-content" => Self::NoContent,
            "file/no-statements" => Self::NoStatements,
            "sqleibniz/unimplemented" => Self::Unimplemented,
            "sql/unterminated-string" => Self::UnterminatedString,
            "sql/unknown-character" => Self::UnknownCharacter,
            "sql/invalid-numeric-literal" => Self::InvalidNumericLiteral,
            "sql/invalid-blob" => Self::InvalidBlob,
            "sql/syntax" => Self::Syntax,
            "sql/missing-semicolon" => Self::Semicolon,
            "sqleibniz/bad-instruction" => Self::BadSqleibnizInstruction,
            "sqleibniz/hook" => Self::Hook,
            "sql/unknown-keyword" => Self::UnknownKeyword,
            "sqlite/unsupported" => Self::SqliteUnsupported,
            "sqlite/unknown-pragma" => Self::UnknownPragma,
            "sqlite/duplicate-relation" => Self::DuplicateRelation,
            "sqlite/unknown-relation" => Self::UnknownRelation,
            "sqlite/unknown-column" => Self::UnknownColumn,
            "sqlite/quirk" => Self::Quirk,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: "string",
                    to: "sqleibniz::rules::Rule".into(),
                    message: Some(format!("Unknown rule name '{value}'")),
                });
            }
        })
    }
}

impl Rule {
    pub fn name(&self) -> &str {
        match self {
            Self::NoContent => "file/no-content",
            Self::NoStatements => "file/no-statements",
            Self::Unimplemented => "sqleibniz/unimplemented",
            Self::UnterminatedString => "sql/unterminated-string",
            Self::UnknownCharacter => "sql/unknown-character",
            Self::InvalidNumericLiteral => "sql/invalid-numeric-literal",
            Self::InvalidBlob => "sql/invalid-blob",
            Self::Syntax => "sql/syntax",
            Self::Quirk => "sqlite/quirk",
            Self::Semicolon => "sql/missing-semicolon",
            Self::BadSqleibnizInstruction => "sqleibniz/bad-instruction",
            Self::Hook => "sqleibniz/hook",
            Self::UnknownKeyword => "sql/unknown-keyword",
            Self::SqliteUnsupported => "sqlite/unsupported",
            Self::UnknownPragma => "sqlite/unknown-pragma",
            Self::DuplicateRelation => "sqlite/duplicate-relation",
            Self::UnknownRelation => "sqlite/unknown-relation",
            Self::UnknownColumn => "sqlite/unknown-column",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::NoContent => "Source file is empty",
            Self::NoStatements => "Source file is not empty but holds no statements",
            Self::Unimplemented => {
                "Source file contains constructs sqleibniz does not yet understand"
            }
            Self::UnterminatedString => "Source file contains an unterminated string",
            Self::UnknownCharacter => "The source file contains an unknown character",
            Self::InvalidNumericLiteral => "The source file contains an invalid numeric literal",
            Self::InvalidBlob => "The source file contains an invalid blob literal",
            Self::Syntax => "The source file contains a structure with incorrect syntax",
            Self::Semicolon => "The source file is missing a semicolon",
            Self::BadSqleibnizInstruction => {
                "The source file contains an invalid sqleibniz instruction"
            }
            Self::Hook => "User-defined Lua hook reported a diagnostic",
            Self::Quirk => "Sqlite or SQL quirk: https://www.sqlite.org/quirks.html",
            Self::UnknownKeyword => "Source file contains an unknown keyword",
            Self::SqliteUnsupported => "Source file uses sql features sqlite does not support",
            Self::UnknownPragma => "Source file uses a PRAGMA not documented by SQLite",
            Self::DuplicateRelation => {
                "Source file defines the same table-like object more than once"
            }
            Self::UnknownRelation => {
                "Source file references a table-like object not defined earlier in the file"
            }
            Self::UnknownColumn => "Source file references a column not defined on a known table",
        }
    }

    pub fn examples(&self) -> &'static [RuleExample] {
        match self {
            Self::NoContent => &[RuleExample {
                sql: "",
                explanation: "An empty file has no content to lex or analyse.",
            }],
            Self::NoStatements => &[RuleExample {
                sql: "-- a file with comments but no SQL statements",
                explanation: "Comments are content, but they do not contain executable SQL statements.",
            }],
            Self::Unimplemented => &[
                RuleExample {
                    sql: "CREATE INDEX idx_users_name ON users ((lower(name)));",
                    explanation: "Expression indexes are valid SQLite syntax, but sqleibniz does not model them yet.",
                },
                RuleExample {
                    sql: "SELECT id FROM users RIGHT JOIN teams;",
                    explanation: "RIGHT JOIN is valid in modern SQLite, but this parser currently supports only a smaller JOIN subset.",
                },
            ],
            Self::UnknownKeyword => &[RuleExample {
                sql: "SELEKT id FROM users;",
                explanation: "SELEKT is not a known SQL keyword; SELECT was probably intended.",
            }],
            Self::BadSqleibnizInstruction => &[RuleExample {
                sql: "-- @sqleibniz::unknown",
                explanation: "sqleibniz instructions must use a supported instruction name such as expect.",
            }],
            Self::Hook => &[RuleExample {
                sql: "VACUUM UpperName;",
                explanation: "A user-defined leibniz.lua hook can report this, for example when enforcing lowercase identifiers.",
            }],
            Self::SqliteUnsupported => &[RuleExample {
                sql: "CREATE VIRTUAL TABLE items USING json_each(value);",
                explanation: "json_each is a table-valued function, not a CREATE VIRTUAL TABLE module supported by SQLite.",
            }],
            Self::UnknownPragma => &[RuleExample {
                sql: "PRAGMA some_extension_pragma;",
                explanation: "SQLite ignores unknown PRAGMAs, so sqleibniz reports names not documented by SQLite.",
            }],
            Self::DuplicateRelation => &[RuleExample {
                sql: "CREATE TABLE users (id INTEGER) STRICT;\nCREATE TABLE users (id INTEGER) STRICT;",
                explanation: "The second CREATE TABLE defines the same relation name again without IF NOT EXISTS.",
            }],
            Self::UnknownRelation => &[RuleExample {
                sql: "CREATE TABLE users (id INTEGER) STRICT;\nSELECT * FROM usres;",
                explanation: "The query references usres, but only users has been defined earlier in the file.",
            }],
            Self::UnknownColumn => &[RuleExample {
                sql: "CREATE TABLE users (id INTEGER) STRICT;\nSELECT name FROM users;",
                explanation: "The users relation is known, but its explicit column list contains id and not name.",
            }],
            Self::Quirk => &[
                RuleExample {
                    sql: "CREATE TABLE users (id INTEGER);",
                    explanation: "SQLite allows non-STRICT tables, but sqleibniz recommends STRICT for more predictable typing.",
                },
                RuleExample {
                    sql: "PRAGMA foreign_keys = false;",
                    explanation: "Disabling foreign key enforcement is legal SQLite behavior, but it weakens relational integrity.",
                },
            ],
            Self::UnterminatedString => &[RuleExample {
                sql: "SELECT 'unterminated;",
                explanation: "The string literal starts with a single quote but never closes it.",
            }],
            Self::UnknownCharacter => &[RuleExample {
                sql: "SELECT 1 # 2;",
                explanation: "# is not a token sqleibniz accepts in SQL source.",
            }],
            Self::InvalidNumericLiteral => &[RuleExample {
                sql: "SELECT 0x;",
                explanation: "A hexadecimal numeric literal needs at least one hexadecimal digit after 0x.",
            }],
            Self::InvalidBlob => &[RuleExample {
                sql: "SELECT x'zz';",
                explanation: "Blob literals must contain hexadecimal digits; z is not valid hex data.",
            }],
            Self::Syntax => &[RuleExample {
                sql: "SELECT id, FROM users;",
                explanation: "The trailing comma means the SELECT result column list is missing another expression before FROM.",
            }],
            Self::Semicolon => &[RuleExample {
                sql: "VACUUM",
                explanation: "sqleibniz expects statements to be terminated with a semicolon.",
            }],
        }
    }
}
