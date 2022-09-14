use crate::prelude::{ChannelRef, Gate, GateDescription, GateRef, GateServiceType, Message};

use super::{Module, ModuleContext};
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Debug;
use std::sync::Arc;

/// A reference to a module
#[derive(Clone)]
pub struct ModuleRef {
    pub(crate) ctx: Arc<ModuleContext>,
    pub(crate) handler: Arc<RefCell<dyn Module + 'static>>,
}

impl std::ops::Deref for ModuleRef {
    type Target = ModuleContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl ModuleRef {
    /// Casts self as ref
    pub fn ref_as<T: Any>(&self) -> Ref<T> {
        let brw = self.handler.borrow();
        let brw: Ref<T> = Ref::map(brw, |v| {
            let r = v as &dyn Any;
            r.downcast_ref::<T>().unwrap()
        });
        brw
    }

    /// Casts self as mut
    pub fn mut_as<T: Any>(&self) -> RefMut<T> {
        let brw = self.handler.borrow_mut();
        let brw: RefMut<T> = RefMut::map(brw, |v| {
            let r = v as &mut dyn Any;
            r.downcast_mut::<T>().unwrap()
        });
        brw
    }

    pub(crate) fn str(&self) -> &str {
        self.ctx.path.path()
    }

    pub(crate) fn activiate(&self) {
        ModuleContext::place(Arc::clone(&self.ctx));
    }

    pub(crate) fn deactivate(&self) {

        // NOP
    }

    /// internal
    // pub fn create_gate_cluster<A>(
    //     &self,
    //     name: &str,
    //     size: usize,
    //     typ: GateServiceType,
    // ) -> Vec<GateRef> {
    //     let ptr = self.clone();
    //     let descriptor = GateDescription::new(name.to_owned(), size, ptr, typ);
    //     let mut ids = Vec::new();

    //     for i in 0..size {
    //         let gate = Gate::new(descriptor.clone(), i, None, None);
    //         ids.push(GateRef::clone(&gate));

    //         self.ctx.gates.borrow_mut().push(gate)
    //         // self.deref_mut().gates.push(gate);
    //     }

    //     ids
    // }

    /// Creates a gate on the current module, returning its ID.
    ///
    pub fn create_gate(&self, name: &str, typ: GateServiceType) -> GateRef {
        self.create_gate_cluster(name, 1, typ).remove(0)
    }

    ///
    /// Creates a gate on the current module that points to another gate as its
    /// next hop, returning the ID of the created gate.
    ///
    pub fn create_gate_into(
        &self,
        name: &str,
        typ: GateServiceType,
        channel: Option<ChannelRef>,
        next_hop: Option<GateRef>,
    ) -> GateRef {
        self.create_gate_cluster_into(name, 1, typ, channel, vec![next_hop])
            .remove(0)
    }

    ///
    /// Createas a cluster of gates on the current module returning their IDs.
    ///
    pub fn create_gate_cluster(
        &self,
        name: &str,
        size: usize,
        typ: GateServiceType,
    ) -> Vec<GateRef> {
        self.create_gate_cluster_into(name, size, typ, None, vec![None; size])
    }

    ///
    /// Creates a cluster of gates on the current module, pointing to the given next hops,
    /// returning the new IDs.
    ///
    /// # Panics
    ///
    /// This function will panic should size != `next_hops.len`()
    ///
    pub fn create_gate_cluster_into(
        &self,
        name: &str,
        size: usize,
        typ: GateServiceType,
        channel: Option<ChannelRef>,
        next_hops: Vec<Option<GateRef>>,
    ) -> Vec<GateRef> {
        assert!(
            size == next_hops.len(),
            "The value 'next_hops' must be equal to the size of the gate cluster"
        );

        let ptr = self.clone();
        let descriptor = GateDescription::new(name.to_owned(), size, ptr, typ);
        let mut ids = Vec::new();

        for (i, item) in next_hops.into_iter().enumerate() {
            let gate = Gate::new(descriptor.clone(), i, channel.clone(), item);
            ids.push(GateRef::clone(&gate));

            self.ctx.gates.borrow_mut().push(gate);
        }

        ids
    }

    /// Handles a message
    pub fn handle_message(&self, msg: Message) {
        self.handler.borrow_mut().handle_message(msg)
    }
}

impl Debug for ModuleRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRef").finish()
    }
}
