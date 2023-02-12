use std::{
    fmt::{self, Debug, Display},
    sync::Arc,
};

use super::*;
use crate::ast::{Annotation, ClusterDefinition, ModuleGateReference, ModuleStmt};

#[derive(Clone, PartialEq)]
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
    pub delay: Option<Symbol>,
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

impl From<&Option<ClusterDefinition>> for Cluster {
    fn from(value: &Option<ClusterDefinition>) -> Self {
        value
            .as_ref()
            .map(Cluster::from)
            .unwrap_or(Cluster::Standalone)
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

impl From<&ModuleGateReference> for ConnectionEndpoint {
    fn from(endp: &ModuleGateReference) -> Self {
        match endp {
            ModuleGateReference::Local(local) => ConnectionEndpoint::Local(
                RawSymbol {
                    raw: local.gate.raw.clone(),
                },
                Cluster::from(&local.gate_cluster),
            ),
            ModuleGateReference::Nonlocal(nonlocal) => ConnectionEndpoint::Nonlocal(
                RawSymbol {
                    raw: nonlocal.submodule.raw.clone(),
                },
                Cluster::from(&nonlocal.submodule_cluster),
                (
                    RawSymbol {
                        raw: nonlocal.gate.raw.clone(),
                    },
                    Cluster::from(&nonlocal.gate_cluster),
                ),
            ),
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

impl Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Module")
            .field("ident", &self.ident)
            .field("gates", &self.gates)
            .field("submodules", &self.submodules)
            .field("connections", &self.connections)
            .field("dirty", &self.dirty)
            .finish()
    }
}
