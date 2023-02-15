use crate::Span;
use std::{collections::LinkedList, error, fmt, io};

mod errors;
mod root;

pub use self::errors::*;
pub use self::root::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub internal: Box<dyn error::Error + Send + Sync>,
    pub span: Option<Span>,
    pub hints: Vec<ErrorHint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
    InvalidConGateServiceTyp,
    InvalidConDefSizes,
    InvalidConClusterIndex,
    MissingEntryPoint,
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

    pub fn map(self, f: impl FnOnce(Error) -> Error) -> Error {
        f(self)
    }

    pub fn solution(&self) -> Option<&ErrorSolution> {
        self.hints.iter().find_map(|h| {
            if let ErrorHint::Solution(s) = h {
                Some(s)
            } else {
                None
            }
        })
    }

    pub fn spanned(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn override_internal(
        mut self,
        internal: impl Into<Box<dyn error::Error + Send + Sync>>,
    ) -> Self {
        self.internal = internal.into();
        self
    }

    pub fn add_hints(mut self, hint: impl Into<ErrorHint>) -> Self {
        self.hints.push(hint.into());
        let mut l = self.hints.len() - 1;
        for i in 0..self.hints.len() {
            if i >= l {
                break;
            }
            if let ErrorHint::Solution(_) = self.hints[i] {
                self.hints.swap(i, l);
                l -= 1;
            }
        }
        self
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
