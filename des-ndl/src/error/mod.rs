use std::{error, fmt};

use crate::Span;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    internal: Box<dyn error::Error + Send + Sync>,
    span: Option<Span>,
    hints: Vec<ErrorHint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    ParseLitError,
    MissingDelim,
    UnexpectedToken,
    UnexpectedDelim,
    ExpectedSingleFoundJoint,
    ExpectedDelimited,
    ExpectedInModuleKeyword,
    ExpectedIdentFoundKeyword,
    MissingToken,
    UnexpectedEOF,
}

#[derive(Debug)]
pub enum ErrorHint {
    Note(String),
    Help(String),
    Solution(ErrorSolution),
}

#[derive(Debug)]
pub struct ErrorSolution {
    pub description: String,
    pub span: Span,
    pub replacement: String,
}

impl Error {
    pub fn new(kind: ErrorKind, internal: impl Into<Box<dyn error::Error + Send + Sync>>) -> Self {
        Self {
            kind,
            internal: internal.into(),
            span: None,
            hints: Vec::new(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.internal, self.kind)
    }
}

impl error::Error for Error {}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}
