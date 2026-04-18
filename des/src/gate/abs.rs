use std::sync::Arc;

use crate::{
    ObjectPath,
    gate::{Entry, Gate, GateRef, IntoGate},
    module::{ModuleRef, ModuleRefWeak},
};

// TECHNICALLY not Arc is needed since only the pair (owner, name) must be shared
// but that can be cloned freely: decide on complexity to unify API,
// cell could be offloaded to storage interface

/// A  reference to a gate.
pub type AbstractGateRef = Arc<AbstractGate>;

/// An abstract gate.
#[derive(Debug)]
pub struct AbstractGate {
    owner: ModuleRefWeak,
    name: String,
}

impl AbstractGate {
    /// Creates a new abstract gate in a detached state.
    ///
    /// # Panics
    ///
    /// This function panics if the chosen namespace is already occupied.
    #[must_use]
    #[track_caller]
    pub fn new(owner: &ModuleRef, name: String) -> AbstractGateRef {
        let mut handle = owner.gates.write();
        assert!(
            !handle.namespaces.contains_key(&name),
            "cannot declare abstract gate in existing namespace"
        );
        handle.namespaces.insert(
            name.clone(),
            Entry {
                prototype: Some(0),
                gates: Vec::new(),
            },
        );
        AbstractGateRef::new(Self {
            owner: ModuleRefWeak::new(owner),
            name,
        })
    }

    /// The position index of the gate within the descriptor cluster.
    /// The human-readable name for the allocated gate cluster.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn name_with_pos(&self) -> String {
        format!("{}[]", self.name())
    }

    /// Returns a short identifcator that holds all nessecary information.
    #[must_use]
    pub fn str(&self) -> String {
        self.name_with_pos()
    }

    /// The full tree path of the gate.
    #[must_use]
    pub fn path(&self) -> ObjectPath {
        self.owner().ctx.path.appended_gate(self.name_with_pos())
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
}

impl IntoGate for AbstractGateRef {
    fn into_gate(self) -> GateRef {
        let module = self.owner.upgrade().expect("could not access owner");
        Gate::new(&module, self.name(), None)
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
        let agate = owner.create_abstract_gate("port");

        let other = ModuleContext::new_root("other".into(), Weak::new());
        let o_gate_a = other.create_gate("gate-a");

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
        let agate = owner.create_abstract_gate("port");

        let other = ModuleContext::new_root("other".into(), Weak::new());
        let o_gate_a = other.create_abstract_gate("gate");

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
        let agate = owner.create_abstract_gate("port");

        let mut modules = Vec::new();
        for i in 0..3 {
            let other = ModuleContext::new_root(format!("other-{i}").into(), Weak::new());
            let o_gate_a = other.create_gate("gate-a");
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
        let _ = owner.create_abstract_gate("port");
        let _ = owner.create_abstract_gate("port");
    }

    #[test]
    #[should_panic = "cannot declare abstract gate in existing namespace"]
    fn abstract_gate_panics_on_duplicate_key_from_normal_gates() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let _ = owner.create_gate("port");
        let _ = owner.create_abstract_gate("port");
    }

    #[test]
    fn abstract_gate_respect_manual_gate_creation() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let agate = owner.create_abstract_gate("port");
        let _ = owner.create_raw_gate("port", 1);
        let _ = owner.create_raw_gate("port", 4);

        let g = agate.into_gate();
        assert_eq!(g.pos(), 5);
    }
}
