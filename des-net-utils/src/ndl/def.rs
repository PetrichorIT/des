use std::{fmt::Display, marker::PhantomData, str::FromStr};

use fxhash::{FxHashMap, FxHashSet};
use serde::{de::Visitor, Deserialize, Serialize};

/// A full network description definition.
///
/// This file should define a full node-tree for an entiere simulation, starting
/// at the entry symbol. This definitions can be expanded into a full network
/// specification, when resolving syntatic sugar, variables etc.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Def {
    /// The entry symbol of the network. This defines the root module for the simulation.
    /// If no symbol is specified, this Def cannot be used standalone, but rather as a dependecie of
    /// a Def with an entry symbol.
    pub entry: String,
    /// The module blueprints defined in this network. Module blueprints can be used at multiple positions
    /// in the resulting node-tree.
    #[serde(default)]
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub modules: FxHashMap<TypClause<ModuleGenericsDef>, ModuleDef>,
    /// The link configurations defined in this network. These links can be used in the modules connection
    /// definition to add metrics to a connecting link.
    #[serde(default)]
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub links: FxHashMap<String, LinkDef>,
}

/// The definition of a link blueprint. This blueprint defines the links
/// core properties, but additional key-value pairs can also be supplied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkDef {
    /// The guaranteed latency of all packets moving over this link,
    /// defined in seconds.
    #[serde(default)]
    pub latency: f64,
    /// The jitter factor, that changes links latency and bitrate,
    /// defined in seconds.
    #[serde(default)]
    pub jitter: f64,
    /// The bitrate of the link.
    #[serde(default)]
    pub bitrate: i32,
    /// Other key-value pairs defined for this link. Link implementations might
    /// choose to ignore these options.
    #[serde(flatten)]
    pub other: FxHashMap<String, String>,
}

/// The typ definition of a module. This name contains a identifier and a list of
/// potential generic arguments, to be used in the module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TypClause<Arg> {
    /// The identifier.
    pub ident: String,
    /// The args
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleGenericsDef {
    pub binding: String,
    pub bound: String,
}

/// The definition of a module blueprint.
///
/// A module corresponds to a **des::net** module. Therefore this definition contains
/// gates local to this modules, submodules created under the namespace of this module
/// and connections between gates of either this module or its children. Additionally
/// a module can inherit definitions from another prototyp. This relation is not represented
/// in **des**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDef {
    /// The inherit symbol. If set, all definitions of this module are automatically included
    /// in this modules definitions. This applies recusivly to chains of inheritence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherit: Option<String>,
    /// A collection of gates defined locally on this module.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<GateDef>,
    /// A collection of submodules associated with this module. The submodules are automatically created
    /// under the modules own namespace.
    ///
    /// E.g. if a module blueprint 'A' defines 'a' submodule 'b' of type 'B', and 'A' is created a the namespace
    /// 'lan.alice' then a module 'lan.alice.b' is created with the type 'B'.
    #[serde(default)]
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub submodules: FxHashMap<FieldDef, TypClause<String>>,
    /// A collection of connections between local gates and the gates of children
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<ConnectionDef>,
}

/// A gate or gate-cluster on a module.
pub type GateDef = FieldDef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmoduleTypDef {
    Concrete(String),
    Dyn(String),
}

/// A connection between gates within a module definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDef {
    /// The peers of the connection. All elements are automatically connected bidirectional.
    pub peers: [ConnectionEndpointDef; 2],
    /// A link-symbol that will apply channel behaviour to a connection.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// A connection endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionEndpointDef {
    pub accessors: Vec<FieldDef>,
}

/// A generic field definition, with an optional cluster/index definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldDef {
    /// The primary identify of the field that is defined.
    pub ident: String,
    /// The cluster / index definition if existent
    pub kardinality: Kardinality,
}

/// A cluster / index definition of field defs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kardinality {
    /// No cluster definition, only a simple field.
    Atom,
    /// A cluster or index defintion with a size or index.
    Cluster(usize),
}

//
// # Imps
//

impl TypClause<ModuleGenericsDef> {
    pub fn inner_ty_to_outer_ty<'a>(&'a self, s: &'a String) -> &'a String {
        for arg in &self.args {
            if arg.binding == *s {
                return &arg.bound;
            }
        }
        s
    }
}

impl<Arg: Display> Serialize for TypClause<Arg> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<Arg: Display> Display for TypClause<Arg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}", self.ident)
        } else {
            write!(
                f,
                "{}({})",
                self.ident,
                self.args
                    .iter()
                    .map(Arg::to_string)
                    .reduce(|a, b| a + ", " + &b)
                    .expect("args is non empty")
            )
        }
    }
}

impl<'de, Arg: FromStr> Deserialize<'de> for TypClause<Arg>
where
    Arg::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FromStringVisitor(PhantomData))
    }
}

impl<Arg: FromStr> FromStr for TypClause<Arg>
where
    Arg::Err: Display,
{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((ident, rem)) = s.split_once('(') else {
            return Ok(TypClause {
                ident: s.to_string(),
                args: Vec::new(),
            });
        };

        assert!(rem.ends_with(')'));
        let rem = rem.trim_end_matches(')');
        let args = rem
            .split(", ")
            .map(Arg::from_str)
            .collect::<Result<Vec<_>, Arg::Err>>()
            .map_err(|e| e.to_string())?;

        Ok(TypClause {
            ident: ident.trim().to_string(),
            args,
        })
    }
}

impl Display for ModuleGenericsDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <- {}", self.binding, self.bound)
    }
}

impl FromStr for ModuleGenericsDef {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (binding, bound) = s
            .split_once("<-")
            .ok_or_else(|| format!("invalid arg: {s}"))?;
        Ok(ModuleGenericsDef {
            binding: binding.trim().to_string(),
            bound: bound.trim().to_string(),
        })
    }
}

impl ModuleDef {
    pub fn required_symbols<'a>(
        &'a self,
        typ: &'a TypClause<ModuleGenericsDef>,
    ) -> FxHashSet<&'a String> {
        let mut set = FxHashSet::default();
        // (0) Require all submodul symbols AND the subrequired deeper args
        set.extend(self.submodules.values().map(|typ| &typ.ident));
        set.extend(self.submodules.values().flat_map(|typ| typ.args.iter()));

        // (1) Except the ones provided by generics
        for arg in &typ.args {
            set.remove(&arg.binding);
        }

        // (2) But require the generic bounds
        set.extend(typ.args.iter().map(|arg| &arg.bound));

        // (3) Require the parent
        set.extend(self.inherit.iter());

        set
    }
}

impl Serialize for ConnectionEndpointDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for ConnectionEndpointDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.accessors
                .iter()
                .map(|v| v.to_string())
                .reduce(|a, b| a + "/" + &b)
                .unwrap_or(String::new()),
        )
    }
}

impl<'de> Deserialize<'de> for ConnectionEndpointDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FromStringVisitor(PhantomData))
    }
}

impl FromStr for ConnectionEndpointDef {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let accessors = s
            .split('/')
            .map(FieldDef::from_str)
            .collect::<std::result::Result<Vec<_>, Self::Err>>()?;
        Ok(ConnectionEndpointDef { accessors })
    }
}

impl Serialize for FieldDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for FieldDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kardinality {
            Kardinality::Atom => write!(f, "{}", self.ident),
            Kardinality::Cluster(n) => write!(f, "{}[{}]", self.ident, n),
        }
    }
}

impl<'de> Deserialize<'de> for FieldDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FromStringVisitor(PhantomData))
    }
}

impl FromStr for FieldDef {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with(']') {
            let (ident, cluster) = s
                .split_once('[')
                .ok_or("invalid syntax: expected opening bracket")?;
            let cluster = cluster.trim_end_matches(']');
            Ok(FieldDef {
                ident: ident.to_string(),
                kardinality: Kardinality::Cluster(
                    cluster.parse::<usize>().map_err(|e| e.to_string())?,
                ),
            })
        } else {
            Ok(FieldDef {
                ident: s.to_string(),
                kardinality: Kardinality::Atom,
            })
        }
    }
}

impl Kardinality {
    pub fn as_size(&self) -> usize {
        match self {
            Kardinality::Atom => 1,
            Kardinality::Cluster(n) => *n,
        }
    }

    pub fn index_iter(&self) -> Box<dyn Iterator<Item = Option<usize>>> {
        match self {
            Kardinality::Atom => Box::new(std::iter::once(None)),
            Kardinality::Cluster(n) => Box::new((0..*n).map(Some)),
        }
    }
}

struct FromStringVisitor<T>(PhantomData<T>);
impl<'de, T: FromStr> Visitor<'de> for FromStringVisitor<T>
where
    T::Err: Display,
{
    type Value = T;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        T::from_str(v).map_err(|e| serde::de::Error::custom(e))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_str(&v)
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_str(v)
    }
}
