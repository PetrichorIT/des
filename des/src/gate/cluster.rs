use std::{fmt::Debug, hash::Hash, sync::Arc};

use des_sync_utils::RwLock;

use crate::{
    ObjectPath,
    gate::{GateRef, IntoGate},
    module::{ModuleRef, ModuleRefWeak},
};

/// A ref to a gate cluster.
pub type GateClusterRef = Arc<GateCluster>;

/// An abstract gate.
pub struct GateCluster {
    pub(super) owner: ModuleRefWeak,
    pub(super) name: String,
    pub(super) inner: RwLock<Inner>,
}

pub(crate) struct Inner {
    pub(crate) automatic: bool,
    pub(crate) members: Vec<GateRef>, // sorted vec
}

impl Inner {
    pub(crate) fn get(&self, pos: usize) -> Option<&GateRef> {
        match self.members.binary_search_by_key(&pos, |g| g.pos()) {
            Ok(i) => Some(&self.members[i]),
            Err(_) => None,
        }
    }

    pub(crate) fn insert(&mut self, gate: GateRef) {
        match self.members.binary_search_by_key(&gate.pos(), |g| g.pos()) {
            Ok(i) | Err(i) => self.members.insert(i, gate),
        }
    }

    pub(crate) fn next(&self) -> Option<usize> {
        self.automatic
            .then(|| self.members.last().map_or(0, |g| g.pos() + 1))
    }
}

impl GateCluster {
    /// Creates a new abstract gate in a detached state.
    ///
    /// # Panics
    ///
    /// This function panics if the chosen namespace is already occupied.
    #[must_use]
    #[track_caller]
    pub fn new(owner: &ModuleRef, name: String, automatic: bool) -> GateClusterRef {
        let mut handle = owner.gates.write();
        handle.create_gate_cluster(name, automatic)
    }

    /// Indicates whether a gate cluster is automatic (aka. can create gates on-demand or just manually).
    pub fn is_automatic(&self) -> bool {
        self.inner.read().automatic
    }

    /// Sets whether a gate cluster is automatic (aka. can create gates on-demand or just manually).
    pub fn set_automatic(&self, automatic: bool) {
        self.inner.write().automatic = automatic;
    }

    /// The position index of the gate within the descriptor cluster.
    /// The human-readable name for the allocated gate cluster.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a short identifcator that holds all nessecary information.
    #[must_use]
    pub fn str(&self) -> String {
        self.name().to_string()
    }

    /// The full tree path of the gate.
    #[must_use]
    pub fn path(&self) -> ObjectPath {
        self.owner().ctx.path.appended_gate(self.name())
    }

    /// Returns the owner module by reference of this gate.
    ///
    /// # Panics
    ///
    /// May panic when called in Drop, since the owner may allready
    /// be dropped.
    #[must_use]
    pub fn owner(&self) -> ModuleRef {
        self.owner
            .upgrade()
            .expect("cannot refer to gate owner during drop")
    }

    /// Returns all concrete gates contained in this cluster.
    pub fn size(&self) -> usize {
        self.inner.read().members.len()
    }

    /// Returns all concreate gates contained in this cluster.
    pub fn members(&self) -> Vec<GateRef> {
        self.inner.read().members.clone()
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for GateCluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateCluster")
            .field("path", &self.path())
            .field("next", &self.inner.read().next())
            .finish()
    }
}

impl PartialEq for GateCluster {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner().ctx, &other.owner().ctx) && self.name == other.name
    }
}

impl Eq for GateCluster {}

impl Hash for GateCluster {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.owner.upgrade().hash(state);
    }
}

impl IntoGate for GateClusterRef {
    fn into_gate(self) -> GateRef {
        if self.is_automatic() {
            let module = self.owner.upgrade().expect("could not access owner");
            module.gates.write().create_gate(self.name(), None)
        } else {
            // one member expection
            let mut members = self.members();
            if members.len() == 1
                && let Some(first) = members.pop()
                && !first.is_standalone()
            {
                first
            } else {
                panic!("expected 1 member gate cluster, got {}", members.len())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Weak;

    use crate::{gate::GateKind, module::ModuleContext};

    use super::*;

    #[test]
    fn abstract_gate_produced_real_gates() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let agate = owner.create_gate_cluster("port");

        let other = ModuleContext::new_root("other".into(), Weak::new());
        let o_gate_a = other.create_singular_gate("gate-a");

        assert_eq!(owner.gates().len(), 0);
        assert_eq!(other.gates(), [o_gate_a.clone()]);

        o_gate_a.clone().connect(agate);

        assert_eq!(owner.gates().len(), 1);
        assert_eq!(other.gates(), [o_gate_a.clone()]);

        assert_eq!(owner.gates()[0].next_gate(), Some(other.gates()[0].clone()));
        assert_eq!(other.gates()[0].next_gate(), Some(owner.gates()[0].clone()));
    }

    #[test]
    fn abstract_2_abstract() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let agate = owner.create_gate_cluster("port");

        let other = ModuleContext::new_root("other".into(), Weak::new());
        let o_gate_a = other.create_gate_cluster("gate");

        assert_eq!(owner.gates().len(), 0);
        assert_eq!(other.gates().len(), 0);
        o_gate_a.clone().connect(agate);

        assert_eq!(owner.gates().len(), 1);
        assert_eq!(other.gates().len(), 1);

        assert_eq!(owner.gates()[0].next_gate(), Some(other.gates()[0].clone()));
        assert_eq!(other.gates()[0].next_gate(), Some(owner.gates()[0].clone()));
    }

    #[test]
    fn abstract_gate_creates_multiple_real_gates() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let agate = owner.create_gate_cluster("port");

        let mut modules = Vec::new();
        for i in 0..3 {
            let other = ModuleContext::new_root(format!("other-{i}").into(), Weak::new());
            let o_gate_a = other.create_singular_gate("gate-a");
            o_gate_a.clone().connect(agate.clone());
            modules.push(other); // Prevent disconnect at drop
        }

        assert_eq!(owner.gates().len(), 3);
        assert!(owner.gates().iter().all(|g| g.kind() == GateKind::Endpoint));
        assert!(owner.gates().iter().all(|g| g.size() == 3));

        drop(modules);
    }

    #[test]
    #[should_panic = "cannot declare abstract gate in existing namespace"]
    fn abstract_gate_panics_on_duplicate_key() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let _ = owner.create_gate_cluster("port");
        let _ = owner.create_gate_cluster("port");
    }

    #[test]
    #[should_panic = "cannot declare abstract gate in existing namespace"]
    fn abstract_gate_panics_on_duplicate_key_from_normal_gates() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let _ = owner.create_singular_gate("port");
        let _ = owner.create_gate_cluster("port");
    }

    #[test]
    fn abstract_gate_respect_manual_gate_creation() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let agate = owner.create_gate_cluster("port");

        let _ = owner.create_gate("port", 1);
        let _ = owner.create_gate("port", 4);

        let g = agate.into_gate();
        assert_eq!(g.pos(), 5);
    }
}
