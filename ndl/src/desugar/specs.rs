use crate::common::*;
use crate::loc::Loc;
use crate::parser::{
    ConNodeIdent, GateDef, LinkDef, ModuleDef, ParamDef, ProtoImplDef, SubsystemDef, TyDef,
};
use crate::AssetDescriptor;
use std::fmt::Display;
use std::path::PathBuf;

///
/// A specification of a defined include.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The alias of the included asset.
    pub path: AssetDescriptor,
}

impl Display for IncludeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Include {{ {} }}", self.path.alias)
    }
}

///
/// A specificiation for the creation of a module.
///
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleSpec<I: Display> {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The identifier in the module namespace.
    pub ident: OIdent,
    /// A collection of child modules required by the this module.
    pub submodules: Vec<ChildNodeSpec>,
    /// A collection of connections between own gates and child gate.
    pub connections: Vec<ConSpec<I>>,
    /// A collection of gates defined on this module.
    pub gates: Vec<GateSpec>,
    /// A collection of parameters.
    pub params: Vec<ParamSpec>,
    /// Indicator whether the module was constructed from a prototype.
    pub derived_from: Option<String>,

    pub(crate) is_prototype: bool,
}

impl<I: Display> ModuleSpec<I> {
    pub fn degrees_of_freedom(&self) -> impl Iterator<Item = (&str, &str)> {
        self.submodules.iter().filter_map(|c| {
            if let TySpec::Dynamic(ref s) = c.ty {
                Some((&c.descriptor[..], s.inner()))

                // Some((&c.descriptor, s.as_ref().unwrap()))
            } else {
                None
            }
        })
    }

    ///
    /// Creates a partially initalized instance from a [ModuleDef].
    /// This means 'loc', 'ident' and 'gates' will be initalized
    /// while 'submodules' and 'connections' must be desugard manually.
    ///
    pub(crate) fn new(module_def: &ModuleDef) -> Self {
        // Do not use Vec::with_capacity()
        // since desugaring will increase the number of entries
        // significantly.

        Self {
            loc: module_def.loc,
            ident: module_def.ident.clone(),
            derived_from: module_def.derived_from.clone(),
            is_prototype: module_def.is_prototype,

            submodules: Vec::new(),
            connections: Vec::new(),
            gates: Vec::new(),

            params: module_def.parameters.iter().map(ParamSpec::new).collect(),
        }
    }
}

impl<I: Display> Display for ModuleSpec<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(derived_from) = &self.derived_from {
            writeln!(f, "module {} like {} {{", self.ident, derived_from)?;
        } else {
            writeln!(
                f,
                "{} {} {{",
                if self.is_prototype {
                    "prototype"
                } else {
                    "module"
                },
                self.ident
            )?;
        }

        if !self.submodules.is_empty() {
            writeln!(f, "\tsubmodules: ")?;
            for submodule in self.submodules.iter() {
                writeln!(f, "\t\t{}", submodule)?
            }
        }

        if !self.gates.is_empty() {
            writeln!(f, "\tgates: ")?;
            for gate in self.gates.iter() {
                writeln!(f, "\t\t{}", gate)?
            }
        }

        if !self.connections.is_empty() {
            writeln!(f, "\tconnections: ")?;
            for connection in self.connections.iter() {
                writeln!(f, "\t\t{}", connection)?
            }
        }

        write!(f, "}}")
    }
}

///
/// A specification for the creation of a network.
///
#[derive(Debug, Clone, PartialEq)]
pub struct SubsystemSpec<I: Display> {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The identifier in the network namespace.
    pub ident: OIdent,
    /// The nodes that should be created for an instance of the network.
    pub nodes: Vec<ChildNodeSpec>,
    /// The connections between the nodes.
    pub connections: Vec<ConSpec<I>>,
    /// A collection of parameters.
    pub params: Vec<ParamSpec>,
    /// The exports
    pub exports: Vec<ExportSpec>,
}

impl<I: Display> SubsystemSpec<I> {
    pub fn degrees_of_freedom(&self) -> impl Iterator<Item = (&String, &String)> {
        self.nodes.iter().filter_map(|c| {
            if let TySpec::Dynamic(ref _s) = c.ty {
                todo!()
                // Some((&c.descriptor, s.as_ref().unwrap()))
            } else {
                None
            }
        })
    }

    ///
    /// Creates a new partially initalized instance of Self.
    /// This means that 'loc' and 'ident' will be initalized,
    /// while 'nodes' and 'connections' must be desugared manually.
    ///
    pub fn new(subsys_def: &SubsystemDef) -> Self {
        // Do not use Vec::with_capacity()
        // since desugaring will increase the number of entries
        // significantly.

        Self {
            loc: subsys_def.loc,
            ident: subsys_def.ident.clone(),
            params: subsys_def.parameters.iter().map(ParamSpec::new).collect(),

            nodes: Vec::new(),
            connections: Vec::new(),
            exports: Vec::new(),
        }
    }
}

impl SubsystemSpec<ConSpecNodeIdent> {
    ///
    /// Fills all expect connections and exports.
    ///
    pub fn from_spec(spec: &SubsystemSpec<ConNodeIdent>) -> Self {
        Self {
            loc: spec.loc,
            ident: spec.ident.clone(),
            params: spec.params.clone(),

            nodes: spec.nodes.clone(),
            connections: Vec::new(),
            exports: Vec::new(),
        }
    }
}

impl<I: Display> Display for SubsystemSpec<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "network {} {{", self.ident)?;

        if !self.nodes.is_empty() {
            writeln!(f, "\tnodes: ")?;
            for node in self.nodes.iter() {
                writeln!(f, "\t\t{}", node)?
            }
        }

        if !self.connections.is_empty() {
            writeln!(f, "\tconnections: ")?;
            for connection in self.connections.iter() {
                writeln!(f, "\t\t{}", connection)?
            }
        }

        if !self.exports.is_empty() {
            writeln!(f, "\texports: ")?;
            for exp in self.exports.iter() {
                writeln!(f, "\t\t{}", exp)?
            }
        }

        write!(f, "}}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSpec {
    pub loc: Loc,
    pub node_ident: String,
    pub node_ty: TySpec,
    pub gate_ident: GateSpec,
}

impl Display for ExportSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.node_ident, self.gate_ident)
    }
}
///
/// A child module specification in either a module or a network.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildNodeSpec {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The local identifer for the parents local namespace.
    pub descriptor: String,
    /// The global type identifier for the type of the child module.
    pub ty: TySpec,
    /// proto impl block
    pub proto_impl: Option<ProtoImplSpec>,
}

impl Display for ChildNodeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ", self.descriptor, self.ty)?;
        if let Some(ref proto) = self.proto_impl {
            write!(f, "{{ ")?;
            for p in &proto.values {
                write!(f, "{} = {}, ", p.0, p.1)?
            }
            write!(f, "}}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoImplSpec {
    // HashMap<ident, ty>
    pub values: Vec<(String, String)>,
}

impl ProtoImplSpec {
    pub(crate) fn get(&self, key: &str) -> Option<&String> {
        self.values
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    pub fn sorted_according_to(mut self, ty: Option<&ModuleDef>) -> Self {
        if let Some(ty) = ty {
            let mut ret = Vec::with_capacity(self.values.len());

            for child in ty.submodules.iter() {
                if matches!(child.ty, TyDef::Dynamic(_)) {
                    // Expecte impls for dynamic types

                    if let Some((from, to)) = &child.desc.cluster_bounds {
                        for idx in *from..*to {
                            // if not found asserted elsewhere
                            if let Some(v) = self
                                .values
                                .iter()
                                .find(|v| v.0 == format!("{}[{}]", child.desc.descriptor, idx))
                            {
                                ret.push(v.clone());
                            } else {
                                // DEBUG WARN
                            }
                        }
                    } else {
                        // if not found asserted elsewhere
                        if let Some(v) = self.values.iter().find(|v| v.0 == child.desc.descriptor) {
                            ret.push(v.clone());
                        } else {
                            // DEBUG WARN
                        }
                    }
                }
            }

            // assert_eq!(ret.len(), self.values.len());
            self.values = ret;
        }

        self
    }

    pub fn new(def: &ProtoImplDef) -> Self {
        Self {
            values: def
                .defs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

///
/// A specificication of a submodules type.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TySpec {
    Static(TyPath),
    Dynamic(TyPath),
}

impl TySpec {
    pub fn unwrap(&self) -> &str {
        self.valid_ident().unwrap().raw()
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic(_))
    }

    pub fn valid_ident(&self) -> Option<&OIdent> {
        match self {
            Self::Static(ref p) => p.valid_ident(),
            Self::Dynamic(ref p) => p.valid_ident(),
        }
    }
}

impl Display for TySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(ref s) => write!(f, "{}", s),
            Self::Dynamic(ref s) => write!(f, "some {}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyPath {
    InScope(OIdent),
    OutOfScope(OIdent),
    Invalid(String),
}

impl TyPath {
    pub fn exists(&self) -> bool {
        matches!(self, Self::InScope(_) | Self::OutOfScope(_))
    }

    pub fn valid_ident(&self) -> Option<&OIdent> {
        match self {
            Self::InScope(ref i) => Some(i),
            Self::OutOfScope(ref i) => Some(i),
            _ => None,
        }
    }

    pub fn inner(&self) -> &str {
        match self {
            Self::InScope(ref s) => s.raw(),
            Self::OutOfScope(ref s) => s.raw(),
            Self::Invalid(ref s) => s,
        }
    }
}

impl Display for TyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InScope(ref s) => write!(f, "{}", s),
            Self::OutOfScope(ref s) => write!(f, "global::{}", s),
            Self::Invalid(ref s) => write!(f, "invalid::{}", s),
        }
    }
}
///
/// A connection specification in either a module or a network.
///
#[derive(Debug, Clone, PartialEq)]
pub struct ConSpec<I: Display> {
    /// The tokens location in the [Source Map](crate::source::SourceMap).
    pub loc: Loc,

    /// The source of the connection.
    pub source: I,
    /// The target of the connection.
    pub target: I,
    /// The delay characterisitcs of the channel.
    pub channel: Option<ChannelSpec>,
}

impl<I: Display> Display for ConSpec<I> {
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
    pub ident: OIdent,

    /// The bitrate of the given connection.
    pub bitrate: usize,
    /// The latency of the given connection.
    pub latency: f64,
    /// The jitter of the given connection.
    pub jitter: f64,
    /// The cost of the link.
    pub cost: f64,
}

impl ChannelSpec {
    pub(crate) fn dummy() -> Self {
        Self {
            loc: Loc::new(0, 0, 0),
            ident: OIdent::new(
                OType::Link,
                AssetDescriptor::new(PathBuf::new(), "Dummy".to_string()),
                "DummyLink".to_string(),
            ),
            bitrate: 1,
            latency: 1.0,
            jitter: 1.0,
            cost: 1.0,
        }
    }

    ///
    /// Creates a fully initialized instance of Self
    /// using a [LinkDef] as reference point.
    ///
    pub(crate) fn new(link_def: &LinkDef) -> Self {
        Self {
            loc: link_def.loc,

            ident: link_def.ident.clone(),
            bitrate: link_def.bitrate,
            latency: link_def.latency,
            jitter: link_def.jitter,
            cost: link_def.cost,
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
    /// The type annotation,
    pub annotation: GateAnnotation,
}

impl GateSpec {
    ///
    /// Creates a new instance of Self
    /// using only a given [GateDef].
    ///
    pub(crate) fn new(gate_def: &GateDef) -> Self {
        Self {
            loc: gate_def.loc,

            ident: gate_def.name.clone(),
            size: gate_def.size,
            annotation: gate_def.annotation,
        }
    }
}

impl Display for GateSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}] {}", self.ident, self.size, self.annotation)
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
