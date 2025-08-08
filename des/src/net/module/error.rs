use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    io,
};

/// An error while resolving a reference to another module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleReferencingError {
    /// No reference exists.
    NoEntry(String),
    /// The reference is not of the given type.
    TypeError(String),
    /// The load order dicates that the parent is not yet ready.
    NotYetInitalized(String),
    /// The reference module is currently inactive, so should not be accessed.
    CurrentlyInactive(String),
}

impl Display for ModuleReferencingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl StdError for ModuleReferencingError {}

impl From<ModuleReferencingError> for io::Error {
    fn from(err: ModuleReferencingError) -> Self {
        io::Error::new(io::ErrorKind::InvalidInput, err)
    }
}
