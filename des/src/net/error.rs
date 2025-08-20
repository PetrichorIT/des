use std::{
    any::Any,
    error::Error as StdError,
    fmt::{Debug, Display},
    io,
};

use crate::net::ObjectPath;

/// An simulation error produced by the `net` feature.
#[derive(Debug)]
pub struct Error {
    /// The origin of the error.
    pub origin: ObjectPath,
    /// The kind of error.
    pub kind: ErrorKind,
}

/// The kind of error.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An error that occured at the end of the simulation, when joining the remaining tasks
    #[cfg(feature = "async")]
    JoinError(JoinErrorKind),
    /// An error that occurs when a requested simulation object is not found.
    ModuleNotFound(String),
    /// An error that occurs when a simulation object panicked.
    ModulePanic(Box<dyn Any + Send + 'static>),
    /// A property error.
    PropError(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.origin, self.kind)
    }
}

impl StdError for Error {}

impl From<Error> for io::Error {
    fn from(value: Error) -> Self {
        match value.kind {
            ErrorKind::PropError(io) => io,
            _ => io::Error::other(value.to_string()),
        }
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
