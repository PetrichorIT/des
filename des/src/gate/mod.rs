//! Module-specific network ports.
//!
//! Gates act as communication endpoints attached to specific modules.
//! Dependent on their topological configuration, gates can be connected
//! to form gate chains. Members of gate-chains have either the kind
//! `Endpoint` or `Transit`. Non-connected gates have the kind `Standalone`.
//!
//! Gates are identified by a unique key and potentially an index. Gates with the same
//! key form clusters. When creating the topology, abstract gates can be used as a stand-in
//! for yet-to-be created gates within a defined clusters. Connecting to an abstract gate
//! will create a concrete gate within the cluster.

use fxhash::FxHashMap;

use crate::channel::IntoDuplexChannel;
use crate::module::ModuleContext;
use crate::module::ModuleRef;
use crate::prelude::ChannelRef;

mod abs;
mod concrete;

pub use self::abs::*;
pub use self::concrete::*;

/// A type that can be used in gate-operations.
pub trait IntoGate {
    /// Converts the value into a concrete gate reference.
    fn into_gate(self) -> GateRef;

    /// Connects two gates into a gate chain element.
    ///
    /// Gates can be organized into a bidirectional gate chain, that
    /// forwards messages two the other end. Using this function two gates
    /// are connected and both gates save their connection state. A gate
    /// can have up to two other gates connected to it, forming a full gate
    /// chain in response.
    ///
    /// If a channel was provided to enable message delaying on this chain element
    /// both direction will have unique instances of the channel, with identical
    /// configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # fn a() -> Option<()>{
    /// # return None;
    /// let a = current().gate(("out", 0))?;
    /// let b = current().parent().ok()?.gate(("in", 0))?;
    /// a.connect(b);
    /// # Some(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if either of the two gates is allready fully connected in a chain.
    /// This function also panics if only one gate is provided
    fn connect<G: IntoGate>(self, other: G)
    where
        Self: Sized,
    {
        self.connect_with::<G, (ChannelRef, ChannelRef)>(other, None);
    }

    /// Connects two gates into a gate chain element.
    ///
    /// Gates can be organized into a bidirectional gate chain, that
    /// forwards messages two the other end. Using this function two gates
    /// are connected and both gates save their connection state. A gate
    /// can have up to two other gates connected to it, forming a full gate
    /// chain in response.
    ///
    /// If a channel was provided to enable message delaying on this chain element
    /// both direction will have unique instances of the channel, with identical
    /// configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # fn a() -> Option<()>{
    /// # return None;
    /// let a = current().gate(("out", 0))?;
    /// let b = current().parent().ok()?.gate(("in", 0))?;
    /// a.connect(b);
    /// # Some(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if either of the two gates is allready fully connected in a chain.
    /// This function also panics if only one gate is provided
    fn connect_with<G: IntoGate, C: IntoDuplexChannel>(self, other: G, channel: Option<C>)
    where
        Self: Sized,
    {
        self.into_gate()
            .raw_connect_with(other.into_gate(), channel);
    }
}

impl<G: IntoGate + Clone> IntoGate for &G {
    fn into_gate(self) -> GateRef {
        self.clone().into_gate()
    }
}

// # IntoModuleGates

mod private {
    pub trait Sealed {}
}

/// A trait for a type to refrence a module specific gate.
pub trait IntoModuleGate: private::Sealed {
    /// Extracts a gate identifier from a module using the given
    /// value as implicit reference.
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef>;
}

impl<T: IntoModuleGate> IntoModuleGate for &T {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        T::as_gate(self, module)
    }
}
impl<T: IntoModuleGate> private::Sealed for &T {}

impl IntoModuleGate for GateRef {
    fn as_gate(&self, _: &ModuleContext) -> Option<GateRef> {
        Some(self.clone())
    }
}
impl private::Sealed for GateRef {}

impl IntoModuleGate for GateRefWeak {
    fn as_gate(&self, _: &ModuleContext) -> Option<GateRef> {
        self.upgrade()
    }
}
impl private::Sealed for GateRefWeak {}

impl IntoModuleGate for (&str, usize) {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        module
            .gates
            .read()
            .namespaces
            .get(self.0)
            .and_then(|entry| entry.gates.iter().find(|g| g.pos() == self.1))
            .cloned()
    }
}
impl private::Sealed for (&str, usize) {}

impl IntoModuleGate for &str {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        module
            .gates
            .read()
            .namespaces
            .get(*self)
            .and_then(|entry| {
                (entry.gates.len() == 1).then(|| entry.gates.iter().find(|g| g.pos() == 0).cloned())
            })
            .flatten()
    }
}
impl private::Sealed for &str {}

// # Gates

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Gates {
    namespaces: FxHashMap<String, Entry>,
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
struct Entry {
    prototype: Option<usize>, // if set, connect() req to pos=None may create new gates
    gates: Vec<GateRef>,
}

impl Gates {
    pub(crate) fn size_of(&self, name: &str) -> usize {
        self.namespaces[name].gates.len()
    }

    pub(crate) fn gates(&self) -> Vec<GateRef> {
        self.namespaces
            .values()
            .flat_map(|v| v.gates.iter())
            .cloned()
            .collect()
    }

    pub(crate) fn get_abstract(&self, owner: &ModuleRef, desc: &str) -> Option<AbstractGateRef> {
        self.namespaces.get(desc).and_then(|v| {
            v.prototype
                .is_some()
                .then(|| AbstractGate::new(owner, desc.to_owned()))
        })
    }
}
