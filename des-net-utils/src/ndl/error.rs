use super::def::FieldDef;
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Other,
    /// (Path, Symbol)
    MissingRegistrySymbol(String, String),
    Io(String),
    UnknownLink(String),
    UnknownModule(String),
    UnresolvableDependency(Vec<String>),
    InvalidGate(String, String),
    InvalidSubmodule(String, String),
    UnknownGateInConnection(usize, FieldDef),
    UnknownSubmoduleInConnection(usize, FieldDef),
    ConnectionIndexOutOfBounds(usize, FieldDef),
    UnequalPeers(usize, usize, usize),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            MissingRegistrySymbol(path, symbol) => write!(
                f,
                "Could not find registry entry for node '{path}' with symbol '{symbol}'"
            ),
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
            UnknownGateInConnection(idx, symbol) => {
                write!(
                    f,
                    "Could not find referenced gate '{symbol}' (Connection #{idx})"
                )
            }
            UnknownSubmoduleInConnection(idx, symbol) => {
                write!(
                    f,
                    "Could not find referenced submodule '{symbol}' (Connection #{idx})"
                )
            }
            ConnectionIndexOutOfBounds(idx, symbol) => {
                write!(
                    f,
                    "Cannot index into '{symbol}', index out of bounds (Connection #{idx})"
                )
            }
            UnequalPeers(idx, lhs, rhs) => {
                write!(
                    f,
                    "Cannot connect peers, clusters have different sizes: {lhs} != {rhs} (Connection #{idx})"
                )
            }
            _ => write!(f, "Error"),
        }
    }
}

impl std::error::Error for Error {}
