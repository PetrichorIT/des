use super::super::ModuleExt;
use crate::{
    net::processing::ProcessingStack,
    prelude::{Gate, Module, ModuleRef},
};

use super::ModuleContext;

/// A spawner.
#[derive(Debug)]
pub struct Spawner<'a> {
    pub(super) ctx: &'a ModuleContext,
}

impl Spawner<'_> {
    pub fn gate(&self, name: impl AsRef<str>, size: usize) {
        let mut gates = Vec::new();

        let name = name.as_ref();
        let sref = self.ctx.sref.read().as_ref().unwrap().upgrade().unwrap();
        for i in 0..size {
            gates.push(Gate::new(&sref, name, size, i))
        }

        self.ctx.gates.write().extend(gates);
    }

    pub fn child<T: Module>(
        &self,
        name: impl AsRef<str>,
        module: T,
        stack: ProcessingStack,
    ) -> ModuleRef {
        let sref = self.ctx.sref.read().as_ref().unwrap().upgrade().unwrap();
        let ctx = ModuleContext::child_of(name.as_ref(), sref);

        ctx.activate();
        let pe = module.to_processing_chain(stack);
        ctx.upgrade_dummy(pe);
        ctx.deactivate(&mut vec![]);

        // TODO: start()

        self.ctx
            .children
            .write()
            .insert(name.as_ref().to_string(), ctx.clone());

        ctx
    }

    pub fn terminate(&self, name: &str) {
        let Some(handle) = self.ctx.children.write().remove(name) else {
            panic!("")
        };

        // TODO: must prevent message sending
        handle.at_sim_end();
    }
}
