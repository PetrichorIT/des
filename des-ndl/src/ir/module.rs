use std::{
    fmt::{self, Display},
    sync::Arc,
};

use super::*;
use crate::{ast::ModuleStmt, Annotation, ClusterDefinition};

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub ast: Arc<ModuleStmt>,

    pub ident: RawSymbol,
    pub gates: Vec<Gate>,
    pub submodules: Vec<Submodule>,
    pub connections: Vec<Connection>,

    pub(crate) dirty: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
    pub ident: RawSymbol,
    pub cluster: Cluster,
    pub service_typ: GateServiceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateServiceType {
    None,
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Submodule {
    pub ident: RawSymbol,
    pub typ: Symbol,
    pub cluster: Cluster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cluster {
    Standalone,
    Clusted(usize),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub from: ConnectionEndpoint,
    pub to: ConnectionEndpoint,
    pub delay: Option<RawSymbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionEndpoint {
    Local(RawSymbol, Cluster),
    Nonlocal(RawSymbol, Cluster, (RawSymbol, Cluster)),
}

// # Impl

impl Cluster {
    pub fn contains(&self, other: &Self) -> bool {
        match self {
            Self::Standalone => matches!(other, Self::Standalone),
            Self::Clusted(cs) => match other {
                Self::Standalone => true,
                Self::Clusted(i) => i < cs,
            },
        }
    }
}

impl From<&ClusterDefinition> for Cluster {
    fn from(value: &ClusterDefinition) -> Self {
        Cluster::Clusted(value.lit.as_integer() as usize)
    }
}

impl From<&Annotation> for GateServiceType {
    fn from(value: &Annotation) -> Self {
        match value.raw.as_str() {
            "input" | "in" | "Input" | "In" => GateServiceType::Input,
            "output" | "out" | "Output" | "Out" => GateServiceType::Output,
            _ => unreachable!(),
        }
    }
}

impl Display for Cluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standalone => Ok(()),
            Self::Clusted(c) => write!(f, "[{c}]"),
        }
    }
}
