use crate::{
    error::Color,
    types::{Token, Type},
};

pub mod builder;

trait Highlight {
    fn lookup(ttype: &Type) -> Color;
    fn as_bytes(&self) -> Vec<u8>;
}

impl Highlight for Color {
    fn lookup(ttype: &Type) -> Color {
        match ttype {
            Type::Keyword(_) => Self::Magenta,
            // atoms
            Type::String(_) | Type::Number(_) | Type::Blob(_) | Type::Boolean(_) => Self::Orange,
            // special symbols
            Type::Dollar
            | Type::Colon
            | Type::Asterisk
            | Type::Question
            | Type::Param(_)
            | Type::Percent
            | Type::ParamName(_) => Self::Red,
            // symbols
            Type::Dot
            | Type::Ident(_)
            | Type::Semicolon
            | Type::Comma
            | Type::Equal
            | Type::At
            | Type::BraceLeft
            | Type::BraceRight
            | Type::BracketLeft
            | Type::BracketRight => Self::White,
            _ => Self::Grey,
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        self.as_str().as_bytes().to_vec()
    }
}

/// highlight performs syntax highlighting on the given [line], depending on the tokens in
/// [token_on_line]. The generated output is writen to the [builder::Builder], thats passed into
/// the function
pub fn highlight(builder: &mut builder::Builder, token_on_line: &[&Token], line: &str) {
    // no tokens on a line means: either comment or empty line
    if token_on_line.is_empty() {
        builder.write_str(Color::Grey.as_str());
        builder.write_str(line);
        builder.write_str(Color::Reset.as_str());
        return;
    }

    let reset = Color::Reset.as_bytes();

    let mut buf = line
        .split("")
        .skip(1)
        .take(line.len())
        .map(|s| s.as_bytes().to_vec())
        .collect::<Vec<Vec<u8>>>();

    let original_length = buf.len();
    for tok in token_on_line {
        let offset = buf.len() - original_length;
        let color = Color::lookup(&tok.ttype);
        buf.insert(tok.start + offset, color.as_bytes());
        if tok.start == tok.end {
            buf.insert(tok.end + offset, reset.clone());
        } else {
            buf.insert(tok.end + offset + 1, reset.clone());
        }
    }

    // INFO: used to inspect the text
    // dbg!(&buf
    //     .iter()
    //     .map(|s| String::from_utf8(s.to_vec()).unwrap())
    //     .collect::<Vec<String>>());

    for element in buf {
        builder.write_buf(element.to_vec());
    }
}
