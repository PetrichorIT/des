use std::{
    any::Any,
    backtrace::{Backtrace, BacktraceStatus},
    convert::Infallible,
    error::Error as StdError,
    fmt::{Debug, Display},
    io,
    ops::{Deref, Index},
    panic::UnwindSafe,
};

use tracing_error::{SpanTrace, SpanTraceStatus};

use crate::{net::ObjectPath, prelude::try_current};

/// An simulation error produced by the `net` feature.
#[derive(Debug)]
pub struct Error {
    /// Boxed internal representation, otherwise the error struct would be too large.
    pub repr: Box<Repr>,
    next: Option<Box<Error>>,
}

/// The internal representation of an error.
#[derive(Debug)]
pub struct Repr {
    /// The origin of the error.
    pub origin: ObjectPath,
    /// The kind of error.
    pub kind: ErrorKind,
    /// The backtrace of the error.
    pub backtrace: Backtrace,
    /// The span trace of the error.
    pub context: SpanTrace,
}

impl Error {
    /// Gets the current error as a list.
    pub fn as_list(&self) -> ErrorList<'_> {
        ErrorList { error: self }
    }

    /// Appends an error to the current error as a list.
    pub fn append(&mut self, error: Error) {
        match self.next {
            None => self.next = Some(Box::new(error)),
            Some(ref mut next) => next.append(error),
        }
    }

    /// Creates a new error.
    #[inline]
    #[must_use]
    pub fn new(origin: ObjectPath, kind: ErrorKind) -> Self {
        Self {
            repr: Box::new(Repr {
                origin,
                kind,
                backtrace: Backtrace::capture(),
                context: SpanTrace::capture(),
            }),
            next: None,
        }
    }

    /// Creates a new error.
    #[inline]
    #[must_use]
    pub fn new_current(kind: ErrorKind) -> Self {
        Self {
            repr: Box::new(Repr {
                origin: try_current().map(|v| v.path()).unwrap_or_default(),
                kind,
                backtrace: Backtrace::capture(),
                context: SpanTrace::capture(),
            }),
            next: None,
        }
    }

    /// Creates a new other error.
    #[inline]
    #[must_use]
    pub fn other<E>(error: E) -> Self
    where
        E: Into<Box<dyn StdError + Send + Sync>>,
    {
        let boxed = error.into();
        Self {
            repr: Box::new(Repr {
                origin: try_current().map(|v| v.path()).unwrap_or_default(),
                kind: ErrorKind::Other(boxed),
                backtrace: Backtrace::capture(),
                context: SpanTrace::capture(),
            }),
            next: None,
        }
    }
}

/// The kind of error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Uncategoried
    Other(Box<dyn StdError + Send + Sync>),
    /// An error that occured at the end of the simulation, when joining the remaining tasks
    #[cfg(feature = "async")]
    JoinError(JoinErrorKind),
    /// An error that occurs when a requested simulation object is not found.
    ModuleNotFound(String),
    /// An error that occurs when a simulation object panicked.
    ModulePanic(Box<dyn Any + Send + 'static>),
    /// A property error.
    PropParsingError(Box<dyn StdError + Send + Sync>),
    /// A property error.
    PropTypeError(String),
}

impl Deref for Error {
    type Target = Repr;
    fn deref(&self) -> &Self::Target {
        &self.repr
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.repr, f)?;
        if let Some(next) = &self.next {
            writeln!(f)?;
            Display::fmt(next, f)
        } else {
            Ok(())
        }
    }
}

impl Display for Repr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.origin)?;
        self.kind.display(f)?;
        if self.backtrace.status() == BacktraceStatus::Captured {
            write!(f, "\nin:\n{}", self.backtrace)?;
        }
        if self.context.status() == SpanTraceStatus::CAPTURED {
            write!(f, "\nin:\n{}", self.context)?;
        }
        Ok(())
    }
}

impl ErrorKind {
    fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(e) => write!(f, "{e}"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl StdError for Error {}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::new(
            try_current().map_or(ObjectPath::from(""), |c| c.path.clone()),
            ErrorKind::PropParsingError(Box::new(value)),
        )
    }
}

impl From<Error> for io::Error {
    fn from(value: Error) -> Self {
        io::Error::other(value.to_string())
    }
}

cfg_async! {
    /// The kind of join error.
    #[derive(Debug)]
    pub enum JoinErrorKind {
        /// The task is not yet finished
        NotFinished,
        /// A panic occurred in the task
        Paniced(Box<dyn Any + Send + 'static>), // < this is not Sync thus we cannot pretend to be an IO error without to_string
        /// The join failed with an tokio error.
        Tokio(tokio::task::JoinError),
    }
}

impl From<Vec<Error>> for Error {
    fn from(value: Vec<Error>) -> Self {
        assert!(!value.is_empty());
        value
            .into_iter()
            .rev()
            .reduce(|a, mut b| {
                b.append(a);
                b
            })
            .expect("iter cannot be empty")
    }
}

impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl Extend<Error> for Error {
    fn extend<T: IntoIterator<Item = Error>>(&mut self, iter: T) {
        // TODO: inefficient
        for val in iter {
            self.append(val);
        }
    }
}

impl UnwindSafe for Error {}

/// A set of failures
pub struct Failure(pub Vec<Error>);

impl Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in &self.0 {
            Display::fmt(error, f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl From<Vec<Error>> for Failure {
    fn from(value: Vec<Error>) -> Self {
        Failure(value)
    }
}

impl From<Error> for Failure {
    fn from(value: Error) -> Self {
        Failure(vec![value])
    }
}

/// A list of errors.
#[derive(Debug)]
pub struct ErrorList<'a> {
    error: &'a Error,
}

impl<'a> ErrorList<'a> {
    /// Gets the i-th error in the list.
    pub fn get(&self, index: usize) -> Option<&'a Error> {
        self.error.get_index(index)
    }
}

impl Index<usize> for Error {
    type Output = Error;
    fn index(&self, index: usize) -> &Self::Output {
        self.as_list().get(index).expect("index out of bounds")
    }
}

impl Error {
    fn get_index(&self, offset: usize) -> Option<&Error> {
        if offset == 0 {
            Some(self)
        } else {
            self.next.as_ref().and_then(|e| e.get_index(offset - 1))
        }
    }
}

/// An iterator over the errors in a list.
#[derive(Debug)]
pub struct ErrorListIter<'a> {
    error: Option<&'a Error>,
}

impl<'a> Iterator for ErrorListIter<'a> {
    type Item = &'a Error;
    fn next(&mut self) -> Option<Self::Item> {
        let error = self.error?;
        self.error = error.next.as_ref().map(|v| &**v);
        Some(error)
    }
}

impl<'a> IntoIterator for ErrorList<'a> {
    type Item = &'a Error;
    type IntoIter = ErrorListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ErrorListIter {
            error: Some(self.error),
        }
    }
}
