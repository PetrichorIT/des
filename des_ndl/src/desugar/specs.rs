use crate::loc::Loc;
use crate::{GateDef, LinkDef, ModuleDef, NetworkDef, ParamDef};
use std::fmt::Display;

///
/// A specification of a defined include.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The alias of the included asset.
    pub path: String,
}

impl Display for IncludeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Include {{ {} }}", self.path)
    }
}

///
/// A specificiation for the creation of a module.
///
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The identifier in the module namespace.
    pub ident: String,
    /// A collection of child modules required by the this module.
    pub submodules: Vec<ChildModuleSpec>,
    /// A collection of connections between own gates and child gate.
    pub connections: Vec<ConSpec>,
    /// A collection of gates defined on this module.
    pub gates: Vec<GateSpec>,
    /// A collection of parameters.
    pub params: Vec<ParamSpec>,
}

impl ModuleSpec {
    ///
    /// Creates a partially initalized instance from a [ModuleDef].
    /// This means 'loc', 'ident' and 'gates' will be initalized
    /// while 'submodules' and 'connections' must be desugard manually.
    ///
    pub fn new(module_def: &ModuleDef) -> Self {
        // Do not use Vec::with_capacity()
        // since desugaring will increase the number of entries
        // significantly.
        Self {
            loc: module_def.loc,

            ident: module_def.name.clone(),
            submodules: Vec::new(),
            connections: Vec::new(),
            gates: module_def.gates.iter().map(GateSpec::new).collect(),
            params: module_def.parameters.iter().map(ParamSpec::new).collect(),
        }
    }
}

///
/// A specification for the creation of a network.
///
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The identifier in the network namespace.
    pub ident: String,
    /// The nodes that should be created for an instance of the network.
    pub nodes: Vec<ChildModuleSpec>,
    /// The connections between the nodes.
    pub connections: Vec<ConSpec>,
    /// A collection of parameters.
    pub params: Vec<ParamSpec>,
}

impl NetworkSpec {
    ///
    /// Creates a new partially initalized instance of Self.
    /// This means that 'loc' and 'ident' will be initalized,
    /// while 'nodes' and 'connections' must be desugared manually.
    ///
    pub fn new(network_def: &NetworkDef) -> Self {
        // Do not use Vec::with_capacity()
        // since desugaring will increase the number of entries
        // significantly.
        Self {
            loc: network_def.loc,

            ident: network_def.name.clone(),
            nodes: Vec::new(),
            connections: Vec::new(),
            params: network_def.parameters.iter().map(ParamSpec::new).collect(),
        }
    }
}

///
/// A child module specification in either a module or a network.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildModuleSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The local identifer for the parents local namespace.
    pub descriptor: String,
    /// The global type identifier for the type of the child module.
    pub ty: String,
}

impl Display for ChildModuleSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.descriptor, self.ty)
    }
}

///
/// A connection specification in either a module or a network.
///
#[derive(Debug, Clone, PartialEq)]
pub struct ConSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The source of the connection.
    pub source: ConSpecNodeIdent,
    /// The target of the connection.
    pub target: ConSpecNodeIdent,
    /// The delay characterisitcs of the channel.
    pub channel: Option<ChannelSpec>,
}

impl Display for ConSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(channel) = &self.channel {
            write!(f, "{} --> {} --> {}", self.source, channel, self.target)
        } else {
            write!(f, "{} --> {}", self.source, self.target)
        }
    }
}

///
/// A descriptor of a gate used in the namespace of a module
/// for creating connections.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConSpecNodeIdent {
    /// A local gate defined in the module which is being processed.
    Local {
        /// The tokens location in the [Source Map](crate::source::SourceMap).
        loc: Loc,
        /// The gate cluster name on the current module,
        gate_ident: String,
        /// The index of the gate inside its gate cluster.
        pos: usize,
    },
    /// A gate on the current modules/networks child node.
    Child {
        /// The tokens location in the [Source Map](crate::source::SourceMap).
        loc: Loc,
        /// The idenfiiert for the child module.
        child_ident: String,
        /// The gate cluster name on the child module,
        gate_ident: String,
        /// The index of the gate inside its gate cluster.
        pos: usize,
    },
}

impl ConSpecNodeIdent {
    /// Returns the location of the current [ConSpecNodeIdent].
    pub fn loc(&self) -> Loc {
        match self {
            Self::Local { loc, .. } => *loc,
            Self::Child { loc, .. } => *loc,
        }
    }
}

impl Display for ConSpecNodeIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local {
                gate_ident, pos, ..
            } => write!(f, "{}[{}]", gate_ident, pos),
            Self::Child {
                child_ident,
                gate_ident,
                pos,
                ..
            } => write!(f, "{}/{}[{}]", child_ident, gate_ident, pos),
        }
    }
}

///
/// A specification of the delay characteristics on
/// a given connectiom.
///
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// A debug symbol for referencing the used NDL Link definition.
    pub ident: String,

    /// The bitrate of the given connection.
    pub bitrate: usize,
    /// The latency of the given connection.
    pub latency: f64,
    /// The jitter of the given connection.
    pub jitter: f64,
}

impl ChannelSpec {
    ///
    /// Creates a fully initialized instance of Self
    /// using a [LinkDef] as reference point.
    ///
    pub fn new(link_def: &LinkDef) -> Self {
        Self {
            loc: link_def.loc,

            ident: link_def.name.clone(),
            bitrate: link_def.bitrate,
            latency: link_def.latency,
            jitter: link_def.jitter,
        }
    }
}

impl Display for ChannelSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {{ {} bit/s  Δ{}±{} }}",
            self.ident, self.bitrate, self.latency, self.jitter
        )
    }
}

///
/// A specification of a gate cluster on a module.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The local module-namespace identifier for the gate cluster.
    pub ident: String,
    /// The size of the gate cluster.
    pub size: usize,
}

impl GateSpec {
    ///
    /// Creates a new instance of Self
    /// using only a given [GateDef].
    ///
    pub fn new(gate_def: &GateDef) -> Self {
        Self {
            loc: gate_def.loc,

            ident: gate_def.name.clone(),
            size: gate_def.size,
        }
    }
}

impl Display for GateSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.ident, self.size)
    }
}

///
/// A specification about module / network parameters.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The identifer in the module namespace for the param.
    pub ident: String,
    /// The type of the param.
    pub ty: String,
}

impl ParamSpec {
    pub fn new(param_def: &ParamDef) -> Self {
        Self {
            loc: param_def.loc,
            ident: param_def.ident.clone(),
            ty: param_def.ty.clone(),
        }
    }
}

impl Display for ParamSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.ident, self.ty)
    }
}
