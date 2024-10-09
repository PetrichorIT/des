use super::def::{FieldDef, ModuleGenericsDef, TypClause};
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Box<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    Other,
    /// (Path, Symbol)
    MissingRegistrySymbol(String, String),
    SymbolAlreadyDefined(String),
    Io(String),
    UnknownLink(String),
    UnknownModule(String),
    UnresolvableDependency(Vec<String>),
    InvalidGate(String, String),
    InvalidSubmodule(String, String),
    UnknownGateInConnection(FieldDef),
    UnknownSubmoduleInConnection(FieldDef),
    ConnectionIndexOutOfBounds(FieldDef),
    UnequalPeers(usize, usize),
    InvalidTypStatement(TypClause<String>, Vec<ModuleGenericsDef>),
    AssignedTypDoesNotConformToInterface(TypClause<String>),
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub module: Option<String>,
    pub submodule: Option<String>,
    pub gate: Option<String>,
    pub connection: Option<usize>,
}

impl Error {
    pub fn span_module(mut self, module: &str) -> Self {
        self.span.module = Some(module.to_string());
        self
    }

    pub fn span_submodule(mut self, submodule: &str) -> Self {
        self.span.submodule = Some(submodule.to_string());
        self
    }

    pub fn span_gate(mut self, gate: &str) -> Self {
        self.span.gate = Some(gate.to_string());
        self
    }

    pub fn span_connection(mut self, connection: usize) -> Self {
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
            _ => write!(f, "Error"),
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
