//! Errors that can occur during NDL parsing.

use super::def::{FieldDef, ModuleGenericsDef, TypClause};
use std::fmt::Display;

/// A result with an NDL error.
pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur during NDL parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    /// The kind of error that has occurred.
    pub kind: ErrorKind,
    /// Context information about the error.
    pub span: Box<Span>,
}

/// A categorization of all possible errors that can occur during NDL parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Unknown / Uncategorizable
    Other,
    /// Upon building a simulation, a symbol could not be connected to a implementation, since the registry
    /// has not provided a valid implementation for the symbol.
    MissingRegistrySymbol(String, String),
    /// A symbol was already defined in the current scope.
    SymbolAlreadyDefined(String),
    /// An IO error occurred.
    Io(String),
    /// An unknown link was encountered.
    UnknownLink(String),
    /// An unknown module was encountered
    UnknownModule(String),
    /// A set of types cannot be resolved, since they are defined cyclically.
    UnresolvableDependency(Vec<String>),
    /// A gate is invalid for the requested usage.
    InvalidGate(String, String),
    /// A submodule is invalid for the requested usage.
    InvalidSubmodule(String, String),
    /// A referenced gate does not exist.
    UnknownGateInConnection(FieldDef),
    /// A referenced submodule does not exist.
    UnknownSubmoduleInConnection(FieldDef),
    /// The indexing of a connection element is out of bounds.
    ConnectionIndexOutOfBounds(FieldDef),
    /// The peers of a requested connection are sets of different sizes.
    UnequalPeers(usize, usize),
    /// A type statement is invalid
    InvalidTypStatement(TypClause<String>, Vec<ModuleGenericsDef>),
    /// A generic bound was not satisfied.
    AssignedTypDoesNotConformToInterface(TypClause<String>),
}

/// Context information about the error.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    /// The module where the error occurred.
    pub module: Option<String>,
    /// The submodule where the error occurred.
    pub submodule: Option<String>,
    /// The gate where the error occurred.
    pub gate: Option<String>,
    /// The connection where the error occurred.
    pub connection: Option<usize>,
}

impl Error {
    #[must_use]
    pub(super) fn span_module(mut self, module: &str) -> Self {
        self.span.module = Some(module.to_string());
        self
    }

    #[must_use]
    pub(super) fn span_submodule(mut self, submodule: &str) -> Self {
        self.span.submodule = Some(submodule.to_string());
        self
    }

    #[must_use]
    pub(super) fn span_gate(mut self, gate: &str) -> Self {
        self.span.gate = Some(gate.to_string());
        self
    }

    #[must_use]
    pub(super) fn span_connection(mut self, connection: usize) -> Self {
        self.span.connection = Some(connection);
        self
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error {
            kind,
            span: Box::new(Span::default()),
        }
    }
}

impl PartialEq<ErrorKind> for Error {
    fn eq(&self, other: &ErrorKind) -> bool {
        self.kind == *other
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.span, self.kind)
    }
}

impl std::error::Error for Error {}

#[allow(clippy::enum_glob_use)]
impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorKind::*;
        match self {
            MissingRegistrySymbol(path, symbol) => write!(
                f,
                "Could not find registry entry for node '{path}' with symbol '{symbol}'"
            ),
            SymbolAlreadyDefined(msg) => write!(f, "Symbol '{msg}' was already defined"),
            Io(msg) => write!(f, "IO: {msg}"),
            UnknownLink(symbol) => write!(f, "Could not find referenced link '{symbol}'"),
            UnknownModule(symbol) => write!(f, "Could not find referenced module '{symbol}'"),
            UnresolvableDependency(deps) => {
                write!(
                    f,
                    "Cloud not resolve dependencies: '{} (TODO)'",
                    deps.iter().fold(String::new(), |a, b| a + ", " + b)
                )
            }
            InvalidGate(module, gate) => {
                write!(f, "Invalid gate definition '{gate}' in module '{module}'")
            }
            InvalidSubmodule(module, submodule) => {
                write!(
                    f,
                    "Invalid submodule definition '{submodule}' in module '{module}'"
                )
            }
            UnknownGateInConnection(symbol) => {
                write!(f, "Could not find referenced gate '{symbol}'")
            }
            UnknownSubmoduleInConnection(symbol) => {
                write!(f, "Could not find referenced submodule '{symbol}'")
            }
            ConnectionIndexOutOfBounds(symbol) => {
                write!(f, "Cannot index into '{symbol}', index out of bounds")
            }
            UnequalPeers(lhs, rhs) => {
                write!(
                    f,
                    "Cannot connect peers, clusters have different sizes: {lhs} != {rhs}"
                )
            }
            InvalidTypStatement(assign, defs) => write!(
                f,
                "Invalid type assigment '{}' for type with generics '{}'",
                assign,
                TypClause {
                    ident: assign.ident.clone(),
                    args: defs.clone()
                }
            ),
            AssignedTypDoesNotConformToInterface(clause) => write!(
                f,
                "Invalid assignment, '{clause}' does not conform to all required interfaces"
            ),
            Other => write!(f, "Error"),
        }
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref module) = self.module {
            if let Some(ref submodule) = self.submodule {
                return write!(f, "modules > {module} > submodules > {submodule}");
            }

            if let Some(ref gate) = self.gate {
                return write!(f, "modules > {module} > gates > {gate}");
            }

            if let Some(ref connection) = self.connection {
                return write!(f, "modules > {module} > connections > {connection}");
            }

            return write!(f, "modules > {module}");
        }

        write!(f, "<no-span>")
    }
}
