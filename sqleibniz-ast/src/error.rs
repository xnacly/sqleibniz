use std::{fs, io::BufRead, path::PathBuf};

use crate::{
    highlight::{builder, highlight},
    types::{Token, rules::Rule},
};

/// ImprovedLine holds the fix a diagnostic suggests for the line it points at
#[derive(Debug, Clone, PartialEq)]
pub struct ImprovedLine {
    /// sql to insert into the faulty line
    pub snippet: &'static str,
    /// offset into the faulty line the snippet is inserted at
    pub start: usize,
}

/// Location points into the source file a token or node came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// zero based line index, add one before displaying it
    pub line: usize,
    /// zero based offset into [Location::line] the token or node starts at
    pub start: usize,
    /// zero based offset into [Location::line] the token or node ends at, used to underline the
    /// faulty snippet in a diagnostic
    pub end: usize,
}

impl Location {
    /// new creates a Location from a zero based line index and the offsets into that line
    pub fn new(line: usize, start: usize, end: usize) -> Self {
        Self { line, start, end }
    }
}

impl From<&Token> for Location {
    fn from(token: &Token) -> Self {
        Self {
            line: token.line,
            start: token.start,
            end: token.end,
        }
    }
}

/// Error is a diagnostic the lexer or the parser reported, it does not necessarily stop either of
/// them, see [crate::ParseResult]
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    /// name of the source file the diagnostic was reported for
    pub file: String,
    /// position in the source file the diagnostic points at
    pub location: Location,
    /// group the diagnostic belongs to, can be disabled from the CLI or leibniz.lua
    pub rule: Rule,
    /// explains why the diagnostic was reported and how to fix it
    pub note: String,
    /// short description of what is wrong
    pub msg: String,
    /// fix for the faulty line, if the lexer or parser knows one
    pub improved_line: Option<ImprovedLine>,
    /// link to the sqlite documentation for the construct the diagnostic was reported for
    pub doc_url: Option<&'static str>,
}

/// Color is an ansi escape code used for diagnostics and syntax highlighting
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Color {
    Reset,

    // used for error display:
    Red,
    Blue,
    Cyan,
    Green,
    Yellow,

    // used for syntax highlighting
    Grey,
    Magenta,
    Orange,
    White,
}

impl Color {
    /// as_str returns the ansi escape code for self
    pub fn as_str(&self) -> &str {
        match self {
            Self::Reset => "\x1b[0m",
            Self::Red => "\x1b[31m",
            Self::Blue => "\x1b[94m",
            Self::Green => "\x1b[92m",
            Self::Yellow => "\x1b[93m",
            Self::Cyan => "\x1b[96m",
            Self::Grey => "\x1b[37m",
            Self::Magenta => "\x1b[95m",
            Self::Orange => "\x1b[33m",
            Self::White => "\x1b[97m",
        }
    }
}

/// warn writes s to the builder, prefixed with a yellow `warn`
pub fn warn(b: &mut builder::Builder, s: &str) {
    print_str_colored(b, "warn", Color::Yellow);
    b.write_str(": ");
    b.write_str(s);
    b.write_char('\n');
}

/// err writes s to the builder, prefixed with a red `error`
pub fn err(b: &mut builder::Builder, s: &str) {
    print_str_colored(b, "error", Color::Red);
    b.write_str(": ");
    b.write_str(s);
    b.write_char('\n');
}

/// print_str_colored writes s to the builder, wrapped in the escape codes for c
pub fn print_str_colored(b: &mut builder::Builder, s: &str, c: Color) {
    b.write_str(c.as_str());
    b.write_str(s);
    b.write_str(Color::Reset.as_str());
}

impl Error {
    /// new creates a diagnostic without a documentation link and without a suggested fix
    pub fn new(
        file: impl Into<String>,
        location: impl Into<Location>,
        rule: Rule,
        msg: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            location: location.into(),
            rule,
            note: note.into(),
            msg: msg.into(),
            improved_line: None,
            doc_url: None,
        }
    }

    /// with_doc_url attaches the sqlite documentation for the faulty construct to the diagnostic
    pub fn with_doc_url(mut self, doc_url: &'static str) -> Self {
        self.doc_url = Some(doc_url);
        self
    }

    /// print renders the diagnostic into the builder: message, file position, the faulty snippet
    /// with two lines of context, the note and the documentation link
    ///
    /// content is the source of [Error::file], tokens are used to syntax highlight the snippet
    pub fn print(&mut self, b: &mut builder::Builder, content: &[u8], tokens: &[Token]) {
        print_str_colored(b, self.rule.name(), Color::Red);
        b.write_str(": ");
        b.write_str(&self.msg);
        b.write_char('\n');

        if content.is_empty() {
            return;
        }

        print_str_colored(b, " -> ", Color::Blue);
        // the file is not absolute, this resolves symlinks and stuff
        let file_path = match fs::canonicalize(PathBuf::from(&self.file)) {
            Ok(path) => path.into_os_string().into_string().unwrap_or_default(),
            _ => self.file.clone(),
        };
        print_str_colored(b, &file_path, Color::Cyan);
        // zero based indexing, we need human friendly numbers here
        print_str_colored(
            b,
            &format!(":{}:{}", self.location.line + 1, self.location.start + 1),
            Color::Yellow,
        );
        b.write_char('\n');

        let lines = content.lines().map(|x| x.unwrap()).collect::<Vec<_>>();

        // eof should always highlight the last line
        if let &Rule::NoStatements = &self.rule {
            self.location.line = lines.len() - 1;
            self.location.end = 0;
        }

        let start_line = self.location.line.saturating_sub(2);
        let end_line = usize::min(self.location.line + 2, lines.len() - 1);

        for (i, line) in lines.iter().enumerate().take(end_line + 1).skip(start_line) {
            print_str_colored(b, &format!(" {:02} | ", i + 1), Color::Blue);
            let line_tokens = tokens.iter().filter(|t| t.line == i).collect::<Vec<_>>();
            highlight(b, &line_tokens, line);
            b.write_char('\n');

            if i == self.location.line {
                let repeat = if self.location.end > self.location.start {
                    self.location.end - self.location.start
                } else {
                    1
                };

                print_str_colored(b, "    | ", Color::Blue);
                print_str_colored(
                    b,
                    &format!(
                        "{}{} error occurs here.\n",
                        " ".repeat(self.location.start),
                        "~".repeat(repeat)
                    ),
                    Color::Red,
                );
            }
        }

        print_str_colored(b, "    |\n", Color::Blue);
        print_str_colored(b, "    ~ note: ", Color::Blue);

        let mut line_len = 0;
        for word in self.note.split_whitespace() {
            let word_len = word.len();
            if line_len + word_len + if line_len > 0 { 1 } else { 0 } > 55 {
                b.write_str("\n            ");
                b.write_str(word);
                line_len = word_len;
            } else {
                if line_len > 0 {
                    b.write_char(' ');
                    line_len += 1;
                }
                b.write_str(word);
                line_len += word_len;
            }
        }
        b.write_char('\n');

        if self.doc_url.is_some() {
            print_str_colored(b, "    ~ docs: ", Color::Blue);
            b.write_str(self.doc_url.unwrap());
            b.write_char('\n');
        }

        print_str_colored(b, " * ", Color::Blue);
        print_str_colored(b, self.rule.name(), Color::Blue);
        b.write_str(": ");
        b.write_str(self.rule.description());
        b.write_char('\n');
    }
}
