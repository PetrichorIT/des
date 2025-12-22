use std::{
    any::Any,
    backtrace::{Backtrace, BacktraceStatus},
    error::Error as StdError,
    fmt::{Debug, Display},
    io,
    ops::Deref,
};

use tracing_error::{SpanTrace, SpanTraceStatus};

use crate::{net::ObjectPath, prelude::try_current, runtime::LikeRuntimeError};

/// An simulation error produced by the `net` feature.
#[derive(Debug)]
pub struct Error {
    /// Boxed internal representation, otherwise the error struct would be too large.
    pub repr: Box<Repr>,
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
        }
    }

    /// Creates a new other error.
    #[inline]
    #[must_use]
    pub fn other(error: impl LikeRuntimeError) -> Self {
        if error.as_any().is::<Self>() {
            // Dirty runtime casting hack, to make a generic check 'T == Error'
            let boxed: Box<dyn Any> = Box::new(error);
            return *boxed.downcast().expect("illegal state");
        }

        Self {
            repr: Box::new(Repr {
                origin: try_current().map(|v| v.path()).unwrap_or_default(),
                kind: ErrorKind::Other(Box::new(error)),
                backtrace: Backtrace::capture(),
                context: SpanTrace::capture(),
            }),
        }
    }
}

/// The kind of error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Uncategoried
    Other(Box<dyn LikeRuntimeError>),
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
