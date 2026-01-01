use std::{fs, io::BufRead, path::PathBuf};

use crate::{
    highlight::{builder, highlight},
    types::{Token, rules::Rule},
};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ImprovedLine {
    pub snippet: &'static str,
    pub start: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Error {
    pub file: String,
    pub line: usize,
    pub rule: Rule,
    pub note: String,
    pub msg: String,
    pub start: usize,
    pub end: usize,
    pub improved_line: Option<ImprovedLine>,
    pub doc_url: Option<&'static str>,
}

#[derive(Debug)]
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

pub fn warn(b: &mut builder::Builder, s: &str) {
    print_str_colored(b, "warn", Color::Yellow);
    b.write_str(": ");
    b.write_str(s);
    b.write_char('\n');
}

pub fn err(b: &mut builder::Builder, s: &str) {
    print_str_colored(b, "error", Color::Red);
    b.write_str(": ");
    b.write_str(s);
    b.write_char('\n');
}

pub fn print_str_colored(b: &mut builder::Builder, s: &str, c: Color) {
    b.write_str(c.as_str());
    b.write_str(s);
    b.write_str(Color::Reset.as_str());
}

impl Error {
    pub fn print(&mut self, b: &mut builder::Builder, content: &[u8], tokens: &[Token]) {
        print_str_colored(b, "error", Color::Red);
        b.write_char('[');
        print_str_colored(b, self.rule.name(), Color::Red);
        b.write_str("]: ");
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
            &format!(":{}:{}", self.line + 1, self.start + 1),
            Color::Yellow,
        );
        b.write_char('\n');

        let lines = content.lines().map(|x| x.unwrap()).collect::<Vec<_>>();

        // eof should always highlight the last line
        if let &Rule::NoStatements = &self.rule {
            self.line = lines.len() - 1;
            self.end = 0;
        }

        let start_line = self.line.saturating_sub(2);
        let end_line = usize::min(self.line + 2, lines.len() - 1);

        for (i, line) in lines.iter().enumerate().take(end_line + 1).skip(start_line) {
            print_str_colored(b, &format!(" {:02} | ", i + 1), Color::Blue);
            let line_tokens = tokens.iter().filter(|t| t.line == i).collect::<Vec<_>>();
            highlight(b, &line_tokens, line);
            b.write_char('\n');

            if i == self.line {
                let repeat = if self.end > self.start {
                    self.end - self.start
                } else {
                    1
                };

                print_str_colored(b, "    | ", Color::Blue);
                print_str_colored(
                    b,
                    &format!(
                        "{}{} error occurs here.\n",
                        " ".repeat(self.start),
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
