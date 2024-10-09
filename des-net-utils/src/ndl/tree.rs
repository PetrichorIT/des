use std::ops::Deref;

use super::def::{FieldDef, GateDef, LinkDef};
use fxhash::FxHashSet;
use serde::{Deserialize, Serialize};

pub type Network = Node;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub typ: Symbol,
    pub submodules: Vec<Submodule>,
    pub gates: FxHashSet<Gate>,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Submodule {
    pub name: FieldDef,
    pub typ: Node,
}

pub type Gate = GateDef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub peers: [ConnectionEndpoint; 2],
    pub link: Option<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEndpoint {
    pub accessors: Vec<ConnectionEndpointAccessor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEndpointAccessor {
    pub name: String,
    pub index: Option<usize>,
}

pub type Link = LinkDef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol(String);

impl Node {
    pub fn conform_to(&self, interface: &Node) -> bool {
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
    pub fn as_name(&self) -> String {
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
