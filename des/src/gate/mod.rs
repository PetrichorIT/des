//! Module-specific network ports.
//!
//! Gates act as communication endpoints attached to specific modules.
//! Dependent on their topological configuration, gates can be connected
//! to form gate chains. Members of gate-chains have either the kind
//! `Endpoint` or `Transit`. Non-connected gates have the kind `Standalone`.
//!
//! Gates are uniquely identified by a name and an index. Gates with the same
//! name form clusters. Gate clusters can be configured to automatically create
//! gates when topology operations require them. If not set to automatic, gate
//! clusters only act as a collection of related gates.

use std::cell::Cell;

use fxhash::FxHashMap;

use crate::channel::IntoDuplexChannel;
use crate::module::ModuleContext;
use crate::module::ModuleRef;
use crate::module::ModuleRefWeak;
use crate::prelude::ChannelRef;

mod cluster;
mod concrete;

pub use self::cluster::*;
pub use self::concrete::*;

/// A type that can be used in gate-operations.
///
/// This can either be a [`GateRef`] or a [`GateClusterRef`].
/// Further implementations may be added in the future.
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
    /// Dependent on the input different semantics appear:
    /// - if the operands are [`GateRef`]s, the gates are connected as a chain element
    /// - if at least one operand is an automatic [`GateClusterRef`], new gates are create to form a chain element
    /// - else the operation fails
    ///
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

//
//  # Gates
//

#[derive(Debug)]
pub(crate) struct Gates {
    owner: ModuleRefWeak,
    namespaces: FxHashMap<String, Entry>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Entry {
    cluster: GateClusterRef,
    gates: Vec<GateRef>,
}

impl Gates {
    pub(crate) fn attach(&mut self, owner: &ModuleRef) {
        self.owner = ModuleRefWeak::new(owner);
    }

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

    pub(crate) fn create_gate(&mut self, name: &str, pos: Option<usize>) -> GateRef {
        // TODO: either disallow default gate creation for abstract clusters,
        // or make counter more resillient to random choices
        match (self.namespaces.get_mut(name), pos) {
            (Some(entry), Some(pos)) => {
                let gate = Gate::raw(self.owner.clone(), name, pos);
                entry.gates.push(gate.clone());
                entry.gates.sort_by_key(|g| g.pos()); // Expensive
                if entry.cluster.prototype.get().is_some() {
                    entry.cluster.prototype.set(Some(
                        entry
                            .gates
                            .iter()
                            .map(|g| g.pos())
                            .max()
                            .unwrap_or_default()
                            + 1,
                    ));
                }

                gate
            }
            (None, Some(pos)) => {
                // Since GateCluster::new requires write() access, but API does not allow for lock-transfer
                let gate = Gate::raw(self.owner.clone(), name, pos);
                let cluster = self.create_gate_cluster(name.to_owned(), false);
                self.namespaces.insert(
                    name.to_owned(),
                    Entry {
                        cluster,
                        gates: vec![gate.clone()],
                    },
                );
                gate
            }
            // We have ensured that counter i is an unoccupied number
            (Some(entry), None) => {
                if let Some(counter) = &mut entry.cluster.prototype.get() {
                    let gate = Gate::raw(self.owner.clone(), name, *counter);
                    entry.gates.push(gate.clone());
                    entry.gates.sort_by_key(|g| g.pos()); // Expensive
                    *counter += 1;
                    gate
                } else {
                    unreachable!(
                        "calls with pos=None should only come from abstract gates, but this namespace is not abstract"
                    )
                }
            }
            (None, None) => unreachable!(
                "calls with pos=None should only come from abstract gates, but none was found"
            ),
        }
    }

    pub(crate) fn create_gate_cluster(&mut self, name: String, automatic: bool) -> GateClusterRef {
        assert!(
            !self.namespaces.contains_key(&name),
            "cannot declare abstract gate in existing namespace"
        );

        let cluster = GateClusterRef::new(GateCluster {
            owner: self.owner.clone(),
            name: name.clone(),
            prototype: Cell::new(if automatic { Some(0) } else { None }),
        });
        self.namespaces.insert(
            name,
            Entry {
                cluster: cluster.clone(),
                gates: Vec::new(),
            },
        );
        cluster
    }

    pub(crate) fn get_cluster(&self, _: &ModuleRef, desc: &str) -> Option<GateClusterRef> {
        self.namespaces.get(desc).map(|v| v.cluster.clone())
    }
}

impl Default for Gates {
    fn default() -> Self {
        Self {
            owner: ModuleRefWeak::empty(),
            namespaces: FxHashMap::default(),
        }
    }
}
