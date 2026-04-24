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

use des_sync_utils::RwLock;
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

/// A trait for a type to refrence a module specific gate.
pub trait IntoModuleGate {
    /// Extracts a gate identifier from a module using the given
    /// value as implicit reference.
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef>;
}

impl<T: IntoModuleGate> IntoModuleGate for &T {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        T::as_gate(self, module)
    }
}

impl IntoModuleGate for GateRef {
    fn as_gate(&self, _: &ModuleContext) -> Option<GateRef> {
        Some(self.clone())
    }
}

impl IntoModuleGate for GateRefWeak {
    fn as_gate(&self, _: &ModuleContext) -> Option<GateRef> {
        self.upgrade()
    }
}

impl IntoModuleGate for (&str, usize) {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        module.gates.read().get_gate(self.0, self.1)
    }
}

impl IntoModuleGate for &str {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        module
            .gates
            .read()
            .get_gate(self, 0) // limit single gate exception only to index 0
            .and_then(|g| (g.size() == 1).then_some(g))
    }
}

//
//  # Gates
//

#[derive(Debug)]
pub(crate) struct Gates {
    owner: ModuleRefWeak,
    namespaces: FxHashMap<String, GateClusterRef>,
}

impl Gates {
    pub(crate) fn attach(&mut self, owner: &ModuleRef) {
        self.owner = ModuleRefWeak::new(owner);
    }

    pub(crate) fn gates(&self) -> Vec<GateRef> {
        self.namespaces
            .values()
            .flat_map(|v| v.members().into_iter())
            .collect()
    }

    pub(crate) fn create_gate(&mut self, name: &str, pos: Option<usize>) -> GateRef {
        match (self.namespaces.get_mut(name), pos) {
            (Some(entry), Some(pos)) => {
                let gate = Gate::raw(self.owner.clone(), entry, name, pos);
                entry.inner.write().insert(gate.clone());
                gate
            }
            (None, Some(pos)) => {
                let cluster = self.create_gate_cluster(name.to_owned(), false);
                let gate = Gate::raw(self.owner.clone(), &cluster, name, pos);
                cluster.inner.write().insert(gate.clone());
                self.namespaces.insert(name.to_owned(), cluster);
                gate
            }
            // We have ensured that counter i is an unoccupied number
            (Some(entry), None) => {
                let mut inner = entry.inner.write();
                if let Some(counter) = inner.next() {
                    let gate = Gate::raw(self.owner.clone(), entry, name, counter);
                    inner.insert(gate.clone());
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
            inner: RwLock::new(Inner {
                automatic,
                members: Vec::new(),
            }),
        });
        self.namespaces.insert(name, cluster.clone());
        cluster
    }

    pub(crate) fn get_gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        self.namespaces
            .get(name)
            .and_then(|entry| entry.inner.read().get(pos).cloned())
    }

    pub(crate) fn get_cluster(&self, desc: &str) -> Option<GateClusterRef> {
        self.namespaces.get(desc).cloned()
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
