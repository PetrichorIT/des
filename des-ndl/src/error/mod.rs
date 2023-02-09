use std::{collections::LinkedList, error, fmt, io};

use crate::Span;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    pub(crate) kind: ErrorKind,
    pub(crate) internal: Box<dyn error::Error + Send + Sync>,
    pub(crate) span: Option<Span>,
    pub(crate) hints: Vec<ErrorHint>,
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
    LinkInheritanceDuplicatedSymbols,
    LinkKnownKeysInvalidValue,
    ModuleGatesDuplicatedSymbols,
    ModuleGatesInvalidClusterSize,
    ModuleSubDuplicatedSymbols,
    ModuleSubInvalidClusterSize,
    InvalidAnnotation,
    InvalidLitTyp,
    SymbolDuplication,
    IoError,
    CyclicDeps,
    RootError,
    SymbolNotFound,
    LinkMissingRequiredFields,
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

    pub fn spanned(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn add_hints(mut self, hint: impl Into<ErrorHint>) -> Self {
        self.hints.push(hint.into());
        self
    }

    pub fn root(errors: LinkedList<Error>) -> Self {
        Self {
            kind: ErrorKind::RootError,
            internal: Box::new(Errors { items: errors }),
            span: None,
            hints: Vec::new(),
        }
    }

    pub fn from_io(io: io::Error) -> Self {
        Self {
            kind: ErrorKind::IoError,
            internal: Box::new(io),
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

// # composite errors

#[derive(Debug)]
pub struct Errors {
    pub items: LinkedList<Error>,
}

impl error::Error for Errors {}
impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for error in self.items.iter() {
            <Error as fmt::Display>::fmt(error, f)?;
        }
        Ok(())
    }
}
