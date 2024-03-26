use std::{error::Error, fmt::Display};

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
        match self {
            Self::NoEntry(str)
            | Self::TypeError(str)
            | Self::NotYetInitalized(str)
            | Self::CurrentlyInactive(str) => {
                write!(f, "{str}")
            }
        }
    }
}

impl Error for ModuleReferencingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        assert_eq!(
            ModuleReferencingError::NoEntry("Hello world!".to_string()).to_string(),
            "Hello world!"
        );
    }
}
