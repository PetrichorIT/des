use std::{
    fmt::{self, Debug, Display},
    sync::Arc,
};

use super::*;
use crate::{
    ast::{Annotation, ClusterDefinition, ModuleStmt},
    ir::GateRef,
    Span,
};

#[derive(Clone, PartialEq)]
pub struct Module {
    pub ast: Arc<ModuleStmt>,

    pub ident: RawSymbol,
    pub inherited: Vec<Symbol>,
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
    pub span: Span,
    pub ident: RawSymbol,
    pub typ: Symbol,
    pub cluster: Cluster,
    pub dynamic: bool,
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

impl Module {
    pub fn all_modules(this: Arc<Module>) -> Vec<Arc<Module>> {
        let mut result = Vec::new();
        Self::_all_modules(this, &mut result);
        result
    }

    fn _all_modules(this: Arc<Module>, result: &mut Vec<Arc<Module>>) {
        if !result.iter().any(|r| Arc::ptr_eq(r, &this)) {
            result.push(this.clone());
            for submod in &this.submodules {
                let Some(sub) = submod.typ.as_module_arc() else {
                    continue;
                };
                Self::_all_modules(sub, result);
            }
        }
    }
}

// impl ConnectionEndpoint {
//     pub fn new(endp: &ModuleGateReference, gate: &GateRef) -> Self {
//         match endp {
//             ModuleGateReference::Local(local) => ConnectionEndpoint::Local(
//                 RawSymbol {
//                     raw: local.gate.raw.clone(),
//                 },
//                 Cluster::new(gate),
//             ),
//             ModuleGateReference::Nonlocal(nonlocal) => ConnectionEndpoint::Nonlocal(
//                 RawSymbol {
//                     raw: nonlocal.submodule.raw.clone(),
//                 },
//                 Cluster::new(gate),
//                 (
//                     RawSymbol {
//                         raw: nonlocal.gate.gate.raw.clone(),
//                     },
//                     Cluster::new(gate),
//                 ),
//             ),
//         }
//     }
// }

impl Cluster {
    pub fn new(gate: &GateRef) -> Self {
        match gate.pos {
            Some(pos) => Self::Clusted(pos),
            None => Self::Standalone,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        match self {
            Self::Standalone => matches!(other, Self::Standalone),
            Self::Clusted(cs) => match other {
                Self::Standalone => true,
                Self::Clusted(i) => i < cs,
            },
        }
    }

    pub fn as_size(&self) -> usize {
        match self {
            Self::Standalone => 1,
            Self::Clusted(n) => *n,
        }
    }

    pub fn as_index(&self) -> usize {
        match self {
            Self::Standalone => 0,
            Self::Clusted(n) => *n,
        }
    }
}

impl From<GateRef<'_>> for ConnectionEndpoint {
    fn from(value: GateRef<'_>) -> Self {
        let gc = Cluster::new(&value);
        match value.submod {
            Some((pos, submod, cl)) => ConnectionEndpoint::Nonlocal(
                RawSymbol { raw: submod },
                if pos == 0 && cl == Cluster::Standalone {
                    Cluster::Standalone
                } else {
                    Cluster::Clusted(pos)
                },
                (value.def.ident.clone(), gc),
            ),
            None => ConnectionEndpoint::Local(value.def.ident.clone(), gc),
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
            .field(
                "inherited",
                &self.inherited.iter().map(|v| v.raw()).collect::<Vec<_>>(),
            )
            .field("gates", &self.gates)
            .field("submodules", &self.submodules)
            .field("connections", &self.connections)
            .field("dirty", &self.dirty)
            .finish()
    }
}
