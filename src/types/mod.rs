pub mod config;
pub mod ctx;
mod keyword;
pub mod rules;
pub mod storage;

#[allow(unused_imports)]
/// this shit is really fucking idiotic, but i have to reexport
/// private identifiers, rust you are fucking weird
pub use self::keyword::Keyword;

#[derive(Debug, Clone, serde::Serialize)]
pub enum Type {
    /// Any and all keywords sqlite3 allows for, such as `SELECT`, `FROM`, etc.
    ///
    /// ## See:
    /// - https://www.sqlite.org/lang_keywords.html
    Keyword(keyword::Keyword),
    Ident(String),
    /// If a numeric literal has a decimal point or an exponentiation clause or if it is less than -9223372036854775808 or greater than 9223372036854775807, then it is a floating point literal.
    /// Otherwise is it is an integer literal. The "E" character that begins the exponentiation clause of a floating point literal can be either upper or lower case.
    ///
    /// The "." character is always used as the decimal point even if the locale setting specifies "," for this role - the use of "," for the decimal point would result in syntactic ambiguity.
    /// Beginning in SQLite version 3.46.0 (2024-05-23), a single extra underscore ("_") character can be added between any two digits.
    /// The underscores are purely for human readability and are ignored by SQLite.
    ///
    /// Hexadecimal integer literals follow the C-language notation of "0x" or "0X" followed by hexadecimal digits.
    /// For example, 0x1234 means the same as 4660 and 0x8000000000000000 means the same as -9223372036854775808.
    /// Hexadecimal integer literals are interpreted as 64-bit two's-complement integers and are thus limited to sixteen significant digits of precision.
    /// Support for hexadecimal integers was added to SQLite version 3.8.6 (2014-08-15).
    /// For backwards compatibility, the "0x" hexadecimal integer notation is only understood by the SQL language parser, not by the type conversions routines.
    /// String variables that contain text formatted like hexadecimal integers are not interpreted as hexadecimal integers when coercing the string value into an integer due to a CAST expression or for a column affinity transformation or prior to performing a numeric operation or for any other run-time conversions.
    /// When coercing a string value in the format of a hexadecimal integer into an integer value, the conversion process stops when the 'x' character is seen so the resulting integer value is always zero.
    /// SQLite only understands the hexadecimal integer notation when it appears in the SQL statement text, not when it appears as part of the content of the database.
    ///
    /// ## See:
    /// - https://www.sqlite.org/lang_expr.html#literal_values_constants_
    /// - https://www.sqlite.org/syntax/numeric-literal.html
    Number(f64),
    ///  A string constant is formed by enclosing the string in single quotes (').
    ///  C-style escapes using the backslash character are not supported because they are not standard SQL.
    ///
    /// ## See:
    /// - https://www.sqlite.org/lang_expr.html#literal_values_constants_
    String(String),
    ///
    /// BLOB literals are string literals containing hexadecimal data and preceded by a single "x" or "X" character.
    ///
    /// ## Example:
    ///
    /// - `X'53514C697465'`
    ///
    /// ## See:
    ///
    /// - https://www.sqlite.org/lang_expr.html#literal_values_constants_
    Blob(Vec<u8>),
    /// The boolean identifiers TRUE and FALSE are usually just aliases for the integer values 1 and 0, respectively.
    ///
    /// ## See:
    ///
    /// - https://www.sqlite.org/lang_expr.html#boolean_expressions
    Boolean(bool),
    /// Parameters for sqlite3_bind() function, expression to be filled at runtime with parameter number.
    /// Count can not exceed `SQLITE_MAX_VARIABLE_NUMBER`.
    ///
    ///
    /// ```text
    ///     :&<str>
    ///     @<&str>
    ///     $<&str>
    /// ```
    ///
    /// ## See:
    /// - https://www.sqlite.org/limits.html#max_variable_number
    /// - https://www.sqlite.org/c3ref/bind_blob.html
    /// - https://www.sqlite.org/lang_expr.html#parameters
    ParamName(String),
    /// Parameters for sqlite3_bind() function, expression to be filled at runtime with parameter number.
    /// Only '?' increments the parameter counter by one. Count can not exceed `SQLITE_MAX_VARIABLE_NUMBER`.
    ///
    /// ```text
    ///     ?
    ///     ?<usize>
    /// ```
    ///
    /// ## See:
    /// - https://www.sqlite.org/limits.html#max_variable_number
    /// - https://www.sqlite.org/c3ref/bind_blob.html
    /// - https://www.sqlite.org/lang_expr.html#parameters
    Param(usize),

    Dot,
    Asterisk,
    Semicolon,
    Percent,
    Comma,
    Equal,
    Question,
    Colon,
    At,
    Dollar,
    BraceLeft,
    BraceRight,
    BracketLeft,
    BracketRight,

    /// Instructs the parser to skip all token until Type::Semicolon is hit
    InstructionExpect,

    Eof,
}

use std::cmp::PartialEq;

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        use Type::*;
        match (self, other) {
            (Keyword(a), Keyword(b)) => a == b,
            (Ident(a), Ident(b)) => a == b,
            (Number(a), Number(b)) => a == b,
            (String(a), String(b)) => a == b,
            (Blob(a), Blob(b)) => a == b,
            (Boolean(a), Boolean(b)) => a == b,
            (ParamName(a), ParamName(b)) => a == b,
            (Param(a), Param(b)) => a == b,
            (Dot, Dot) => true,
            (Asterisk, Asterisk) => true,
            (Semicolon, Semicolon) => true,
            (Percent, Percent) => true,
            (Comma, Comma) => true,
            (Equal, Equal) => true,
            (Question, Question) => true,
            (Colon, Colon) => true,
            (At, At) => true,
            (Dollar, Dollar) => true,
            (BraceLeft, BraceLeft) => true,
            (BraceRight, BraceRight) => true,
            (BracketLeft, BracketLeft) => true,
            (BracketRight, BracketRight) => true,
            (InstructionExpect, InstructionExpect) => true,
            (Eof, Eof) => true,
            _ => false,
        }
    }
}

impl Eq for Type {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Token {
    pub ttype: Type,
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

impl Token {
    // #[cfg(test)]
    pub fn new(ttype: Type) -> Self {
        Self {
            ttype,
            start: 0,
            end: 0,
            line: 0,
        }
    }
}
