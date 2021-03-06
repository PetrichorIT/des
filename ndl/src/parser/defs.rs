use crate::{common::OIdent, GateAnnotation, Loc};
use std::{collections::HashMap, fmt::Display};

///
/// A definition of a include statement.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDef {
    /// The token location of the include.
    pub loc: Loc,
    /// The imported modules alias.
    pub path: String,
}

impl Display for IncludeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

///
/// A definition of a channel.
///
#[derive(Debug, Clone, PartialEq)]
pub struct LinkDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier of the channel.
    pub ident: OIdent,

    /// The defining metric for the channel.
    pub bitrate: usize,

    /// The defining metric for the channel.
    pub latency: f64,

    /// The defining metric for the channel.
    pub jitter: f64,

    /// The cost of the link.
    pub cost: f64,
}

impl Eq for LinkDef {}

impl Display for LinkDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(bitrate: {}, latency: {}, jitter: {})",
            self.ident, self.bitrate, self.latency, self.jitter
        )
    }
}

///
/// A definition of a module.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier of the module.
    pub ident: OIdent,
    /// The local submodules defined for this module.
    pub submodules: Vec<ChildNodeDef>,
    /// The gates exposed on this module.
    pub gates: Vec<GateDef>,
    /// The connections defined by this module.
    pub connections: Vec<ConDef>,
    /// The parameters expected by this module.
    pub parameters: Vec<ParamDef>,
    /// Indicate whether this type will actually be instantiated
    pub is_prototype: bool,

    pub derived_from: Option<String>,
}

impl ModuleDef {
    // pub fn full_path<'a>(&self, smap: &'a SourceMap) -> (&str, &'a str) {
    //     let asset = smap.get_asset_for_loc(self.loc);
    //     (&self.ident, &asset.alias)
    // }
}

///
/// A definition of a local submodule, in a modules definition.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildNodeDef {
    /// The location of the source tokens.
    pub loc: Loc,

    /// The type of the submodule.
    pub ty: TyDef,
    /// A module internal descriptor for the created submodule.
    pub desc: LocalDescriptorDef,
    /// A block for proto impls
    pub proto_impl: Option<ProtoImplDef>,
}

///
/// A definition of a local descriptor
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDescriptorDef {
    pub loc: Loc,

    // ensure that descriptor is NOT terminated with any numeric.
    pub descriptor: String,
    pub cluster_bounds: Option<(usize, usize)>,
}

impl LocalDescriptorDef {
    pub(crate) fn cluster_bounds_contain(&self, index: usize) -> bool {
        let (from, to) = self.cluster_bounds.unwrap();
        from <= index && index <= to
    }

    pub fn new_non_cluster(descriptor: String, loc: Loc) -> Self {
        Self {
            loc,
            descriptor,
            cluster_bounds: None,
        }
    }

    pub fn cluster_bounds_overlap(&self, other: &Self) -> bool {
        if let Some(self_bounds) = &self.cluster_bounds {
            if let Some(other_bounds) = &other.cluster_bounds {
                // Three cases:
                // - overlap
                // - no overlap
                //  -> self << other
                //  -> other << self

                let self_lt_other = self_bounds.1 < other_bounds.0;
                let other_lt_self = other_bounds.1 < self_bounds.0;

                return !self_lt_other && !other_lt_self;
            }
        }

        // is a sense they do,
        true
    }
}

impl Display for LocalDescriptorDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((from, to)) = &self.cluster_bounds {
            write!(f, "{}[{}...{}]", self.descriptor, from, to)
        } else {
            write!(f, "{}", self.descriptor)
        }
    }
}

///
/// A definition of a submodules type.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyDef {
    Static(String),
    Dynamic(String),
}

impl TyDef {
    pub fn inner(&self) -> &str {
        match self {
            Self::Static(ref s) => s,
            Self::Dynamic(ref s) => s,
        }
    }
}

impl Display for TyDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(ref s) => write!(f, "{}", s),
            Self::Dynamic(ref s) => write!(f, "some {}", s),
        }
    }
}

///
/// A proto impl block
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoImplDef {
    pub defs: HashMap<String, String>,
}

impl ProtoImplDef {
    pub fn new() -> Self {
        Self {
            defs: HashMap::new(),
        }
    }
}

impl Default for ProtoImplDef {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ProtoImplDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for def in &self.defs {
            writeln!(f, "{} = {}", def.0, def.1)?
        }

        Ok(())
    }
}

///
/// A definition of a Gate, in a modules definition.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier of the local gate cluster.
    pub name: String,
    /// The size of the local gate cluster.
    pub size: usize,
    /// A annotation indicating a service type.
    pub annotation: GateAnnotation,
}

impl Display for GateDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}] {}", self.name, self.size, self.annotation)
    }
}

///
/// A description of a connection, in a modules definition.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The origin gate cluster the connection starts from.
    pub from: ConNodeIdent,
    /// The channel that is used to creat delays on this connection.
    pub channel: Option<String>,
    /// The target gate cluster the connection points to.
    pub to: ConNodeIdent,
}

impl Display for ConDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(channel) = &self.channel {
            write!(f, "{} --> {} --> {}", self.from, channel, self.to)
        } else {
            write!(f, "{} --> {}", self.from, self.to)
        }
    }
}

///
/// A gate cluster definition, that may reference a submodules gate cluster,
/// inside a modules connection definition.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConNodeIdent {
    Local {
        loc: Loc,
        ident: Ident,
    },
    Child {
        loc: Loc,
        child: Ident,
        ident: Ident,
    },
}

impl ConNodeIdent {
    pub fn loc(&self) -> Loc {
        match self {
            Self::Local { loc, .. } => *loc,
            Self::Child { loc, .. } => *loc,
        }
    }
}

impl Display for ConNodeIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local { ident, .. } => write!(f, "{}", ident),
            Self::Child { child, ident, .. } => write!(f, "{}/{}", child, ident),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ident {
    Direct { ident: String },
    Clustered { ident: String, index: usize },
}

impl Ident {
    pub fn is_clustered(&self) -> bool {
        matches!(self, Self::Clustered { .. })
    }

    pub(crate) fn unwrap_direct(self) -> String {
        match self {
            Self::Direct { ident } => ident,
            _ => panic!("Unwraped Ident expecting direct, but got clustered"),
        }
    }

    pub(crate) fn raw_ident(&self) -> &str {
        match self {
            Self::Direct { ident } => ident,
            Self::Clustered { ident, .. } => ident,
        }
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Direct { ident } => write!(f, "{}", ident),
            Self::Clustered { ident, index } => write!(f, "{}[{}]", ident, index),
        }
    }
}

///
/// A parameter for a module.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier for the parameter.
    pub ident: String,
    /// The type of the parameter.
    pub ty: String,
}

///
/// A definition of a Network.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsystemDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier of the network.
    pub ident: OIdent,
    /// The local submodules defined for this module.
    pub nodes: Vec<ChildNodeDef>,
    /// The connections defined by this module.
    pub connections: Vec<ConDef>,
    /// The parameters expected by this module.
    pub parameters: Vec<ParamDef>,
    /// The exported interfaces
    pub exports: Vec<ExportDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasDef {
    /// The tokens location in the source asset.
    pub loc: Loc,

    /// The identifier of the alias.
    pub ident: OIdent,
    /// The identifier of the network.
    pub prototype: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportDef {
    pub loc: Loc,

    pub module: String,
    pub gate: String,
}

impl Display for ExportDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.module, self.gate)
    }
}
