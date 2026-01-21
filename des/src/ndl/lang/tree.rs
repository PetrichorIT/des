//! The output types after the transformation into a module tree.
use std::ops::Deref;

use super::def::{FieldDef, GateDef, LinkDef};
use fxhash::FxHashSet;
use serde::{Deserialize, Serialize};

/// A module tree at its node
pub type Network = Node;

/// A node corresponding to a simulation node created through `SimBuilder::node`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// The typename of the node, used in the registry to assign a implementation.
    pub typ: Symbol,
    /// The submodules that this node should create.
    pub submodules: Vec<Submodule>,
    /// The gates present on this node.
    pub gates: FxHashSet<Gate>,
    /// The gate connections that should be created under the authority of this node.
    pub connections: Vec<Connection>,
}

/// A submodule definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Submodule {
    /// A name for the submodule, defining a new `ObjectPath`
    pub name: FieldDef,
    /// The typename of the submodule
    pub typ: Node,
}

///A gate definition.
pub type Gate = GateDef;

/// A connection definition after desugaring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    /// The endpoints that need to be connected.
    pub peers: [ConnectionEndpoint; 2],
    /// The channel that should be created between the endpoints.
    pub link: Option<Link>,
}

/// A endpoint defined by its relative position to the calling node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEndpoint {
    /// A sequence of accessors that define the relative position of the endpoint.
    pub accessors: Vec<ConnectionEndpointAccessor>,
}

/// A part of a definition sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEndpointAccessor {
    /// A plaintext identifier to identify the submodule / gate
    pub name: String,
    /// A index to index into complex clusters.
    pub index: Option<usize>,
}

/// A link.
pub type Link = LinkDef;

/// A symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol(String);

impl Node {
    #[must_use]
    pub(super) fn conform_to(&self, interface: &Node) -> bool {
        if !interface.gates.is_subset(&self.gates) {
            return false;
        }

        if !interface
            .submodules
            .iter()
            .all(|submod| self.submodules.iter().any(|other| other == submod))
        {
            return false;
        }

        if !interface
            .connections
            .iter()
            .all(|con| self.connections.iter().any(|other| other == con))
        {
            return false;
        }

        true
    }
}

impl Deref for Symbol {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl ConnectionEndpointAccessor {
    #[must_use]
    pub(crate) fn as_name(&self) -> String {
        if let Some(index) = self.index {
            format!("{}[{}]", self.name, index)
        } else {
            self.name.clone()
        }
    }
}

impl From<&String> for Symbol {
    fn from(value: &String) -> Self {
        Self(value.clone())
    }
}
