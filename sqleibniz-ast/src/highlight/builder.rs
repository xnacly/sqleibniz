use std::fmt::Display;

/// builder implements a string builder, in its api similar to [strings.Builder](https://pkg.go.dev/strings#Builder)
#[derive(Default)]
pub struct Builder {
    buffer: Vec<u8>,
}

impl Builder {
    /// with_capacity creates a Builder with a buffer of cap bytes
    pub fn with_capacity(cap: usize) -> Self {
        Builder {
            buffer: Vec::with_capacity(cap),
        }
    }

    /// write_char appends a single char to the buffer
    pub fn write_char(&mut self, char: char) {
        self.buffer.push(char as u8);
    }

    /// write_byte appends a single byte to the buffer
    pub fn write_byte(&mut self, byte: u8) {
        self.buffer.push(byte);
    }

    /// write_str appends str to the buffer
    pub fn write_str(&mut self, str: &str) {
        self.buffer.append(&mut str.as_bytes().to_vec());
    }

    /// write_string appends string to the buffer, consuming it
    pub fn write_string(&mut self, string: String) {
        self.buffer.append(&mut string.into_bytes())
    }

    /// write_buf appends buf to the buffer, consuming it
    pub fn write_buf(&mut self, buf: Vec<u8>) {
        let mut b = buf;
        self.buffer.append(&mut b)
    }

    /// string consumes the Builder
    pub fn string(self) -> String {
        match String::from_utf8(self.buffer) {
            Ok(string) => string,
            Err(_) => String::from("<failed to stringify Builder::buffer>"),
        }
    }
}

impl Display for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match String::from_utf8(self.buffer.clone()) {
            Ok(string) => string,
            Err(_) => String::from("<failed to stringify Builder::buffer"),
        };
        write!(f, "{}", string)
    }
}
