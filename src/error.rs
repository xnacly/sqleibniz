use std::{fs, io::BufRead, path::PathBuf};

use crate::{
    highlight::{builder, highlight},
    types::{Token, rules::Rule},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ImprovedLine {
    pub snippet: &'static str,
    pub start: usize,
}

#[derive(Debug, Clone, PartialEq)]
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
            Self::Grey => "\x1b[90m",
            Self::Magenta => "\x1b[35m",
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
    pub fn print(&mut self, b: &mut builder::Builder, content: &Vec<u8>, tokens: &[Token]) {
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
        // the file is not absolut, this resolves symlinks and stuff
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

        if self.line >= 2 {
            if let Some(first_line) = lines.get(self.line - 2) {
                print_str_colored(b, &format!(" {:02} | ", self.line - 1), Color::Blue);
                highlight(
                    b,
                    &tokens
                        .iter()
                        .filter(|t| t.line == self.line - 2)
                        .collect::<Vec<&Token>>(),
                    first_line,
                );
                b.write_char('\n');
            }

            if let Some(sec_line) = lines.get(self.line - 1) {
                print_str_colored(b, &format!(" {:02} | ", self.line), Color::Blue);
                highlight(
                    b,
                    &tokens
                        .iter()
                        .filter(|t| t.line == self.line - 1)
                        .collect::<Vec<&Token>>(),
                    sec_line,
                );
                b.write_char('\n');
            }
        }

        let offending_line = lines.get(self.line).unwrap();
        print_str_colored(b, &format!(" {:02} | ", self.line + 1), Color::Blue);
        highlight(
            b,
            &tokens
                .iter()
                .filter(|t| t.line == self.line)
                .collect::<Vec<&Token>>(),
            offending_line,
        );
        print_str_colored(b, "\n    |", Color::Blue);

        let mut repeat = 1;
        if self.end > self.start {
            repeat = self.end - self.start;
        }

        print_str_colored(
            b,
            &format!(
                " {}{} error occurs here.\n",
                " ".repeat(self.start),
                "~".repeat(repeat)
            ),
            Color::Red,
        );

        // TODO: rework this, inconsistently corret
        // if let Some(new) = &self.improved_line {
        //     print_str_colored("    + ", Color::Green);
        //     print_str!(offending_line);
        //     print_str_colored(&new.snippet, Color::Green);
        //     print_str_colored("\n    | ", Color::Blue);
        //     print_str_colored(
        //         &format!(
        //             " {}{} possible fix.",
        //             " ".repeat(new.start),
        //             "^".repeat(new.snippet.len())
        //         ),
        //         Color::Green,
        //     );
        //     println!()
        // }

        if let Some(first_line) = lines.get(self.line + 1) {
            print_str_colored(b, &format!(" {:02} | ", self.line + 2), Color::Blue);
            highlight(
                b,
                &tokens
                    .iter()
                    .filter(|t| t.line == self.line + 1)
                    .collect::<Vec<&Token>>(),
                first_line,
            );
            b.write_char('\n');
        }

        if let Some(sec_line) = lines.get(self.line + 2) {
            print_str_colored(b, &format!(" {:02} | ", self.line + 3), Color::Blue);
            highlight(
                b,
                &tokens
                    .iter()
                    .filter(|t| t.line == self.line + 2)
                    .collect::<Vec<&Token>>(),
                sec_line,
            );
            b.write_char('\n');
        }

        print_str_colored(b, "    |\n", Color::Blue);
        print_str_colored(b, "    ~ note: ", Color::Blue);
        b.write_str(&self.note);
        b.write_char('\n');

        print_str_colored(b, "  * ", Color::Blue);
        print_str_colored(b, self.rule.name(), Color::Blue);
        b.write_str(": ");
        b.write_str(self.rule.description());
        b.write_char('\n');

        if self.doc_url.is_some() {
            print_str_colored(b, " docs", Color::Blue);
            b.write_str(": ");
            b.write_str(self.doc_url.unwrap());
            b.write_char('\n');
        }
    }
}
