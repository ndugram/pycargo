use std::fmt;

#[derive(Debug)]
pub enum Kind {
    Syntax,
    Runtime,
}

#[derive(Debug)]
pub struct PyError {
    pub kind: Kind,
    pub msg: String,
    pub line: usize,
}

impl PyError {
    pub fn syntax(msg: impl Into<String>, line: usize) -> Self {
        PyError { kind: Kind::Syntax, msg: msg.into(), line }
    }
    pub fn runtime(msg: impl Into<String>) -> Self {
        PyError { kind: Kind::Runtime, msg: msg.into(), line: 0 }
    }
}

impl fmt::Display for PyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            Kind::Syntax if self.line > 0 => write!(f, "SyntaxError (line {}): {}", self.line, self.msg),
            Kind::Syntax => write!(f, "SyntaxError: {}", self.msg),
            Kind::Runtime => write!(f, "RuntimeError: {}", self.msg),
        }
    }
}

impl std::error::Error for PyError {}

pub type Result<T> = std::result::Result<T, PyError>;
