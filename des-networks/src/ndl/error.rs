use std::fmt::Display;

use super::def::FieldDef;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Other,
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
        write!(f, "Error")
    }
}

impl std::error::Error for Error {}
