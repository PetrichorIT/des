use crate::net::module::State;
use crate::net::processing::{ModuleImpl, ProcessingStack};
use crate::prelude::{Gate, GateRef};
use crate::tracing::{enter_scope, leave_scope};

use super::{DummyModule, Module, ModuleContext};
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::{Arc, Weak};

#[derive(Clone)]
pub(crate) struct ModuleRefWeak {
    ctx: Weak<ModuleContext>,
    handler: Weak<RefCell<ModuleImpl>>,
}

impl ModuleRefWeak {
    pub(crate) fn empty() -> Self {
        Self {
            ctx: Weak::new(),
            handler: Weak::new(),
        }
    }

    pub(crate) fn new(strong: &ModuleRef) -> Self {
        Self {
            ctx: Arc::downgrade(&strong.ctx),
            handler: Arc::downgrade(&strong.processing),
        }
    }

    pub(crate) fn upgrade(&self) -> Option<ModuleRef> {
        Some(ModuleRef {
            ctx: self.ctx.upgrade()?,
            processing: self.handler.upgrade()?,
        })
    }
}

impl Debug for ModuleRefWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(mref) = self.upgrade() {
            mref.fmt(f)
        } else {
            f.debug_struct("Weak").finish_non_exhaustive()
        }
    }
}

/// A reference to a module
#[derive(Clone)]
pub struct ModuleRef {
    pub(crate) ctx: Arc<ModuleContext>,
    pub(crate) processing: Arc<RefCell<ModuleImpl>>,
}

impl Deref for ModuleRef {
    type Target = ModuleContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl ModuleRef {
    #[allow(unused)]
    pub(crate) fn dummy(ctx: Arc<ModuleContext>) -> Self {
        // Create the dummy module explicitly not with ::new since
        // all dyn Module calls would panic
        let module = Box::new(DummyModule {});
        let stack = ModuleImpl::new(module.stack(ProcessingStack::default()), module);
        let processing = Arc::new(RefCell::new(stack));
        let this = Self { ctx, processing };
        *this.ctx.me.write() = ModuleRefWeak::new(&this);
        this
    }

    #[allow(unused)]
    // Caller must ensure that handler is indeed a dummy
    #[doc(hidden)]
    pub fn upgrade_dummy(&self, module: ModuleImpl) {
        let celled = RefCell::new(module);
        self.processing.swap(&celled);
        self.ctx.state.set(State::Initialized);
    }

    /// Borrows the referenced module as a readonly reference
    /// to the provided type T.
    ///
    /// # Panics
    ///
    /// Panics if either the module is not of type T,
    /// or the module is allready borrowed mutably.
    #[must_use]
    pub fn as_ref<T: Any>(&self) -> Ref<'_, T> {
        self.try_as_ref::<T>()
            .expect("Failed to cast ModuleRef to readonly reference to type T")
    }

    ///
    /// Tries to borrow the referenced module as an readonly
    /// reference to the provided type T.
    ///
    /// This function will return `None` is the contained module
    /// is not of type T.
    ///
    /// # Panics
    ///
    /// This function panics if the contained module is allready borrowed
    /// mutably. This may be the case if another borrow has allready occured
    /// or the reference module is `self` and a module-specific function is called.
    ///
    #[must_use]
    pub fn try_as_ref<T: Any>(&self) -> Option<Ref<'_, T>> {
        Ref::filter_map(
            self.processing.try_borrow()
                .expect("could not aquire handle to node implementation, since the implementation is currently active"),
            |processor| {
                processor.downcast_element_ref::<T>()
            }
        ).ok()
    }

    /// Borrows the referenced module as a mutable reference
    /// to the provided type T.
    ///
    /// # Panics
    ///
    /// Panics if either the module is not of type T,
    /// or the module is allready borrowed on any way.
    #[must_use]
    pub fn as_mut<T: Any>(&self) -> RefMut<'_, T> {
        self.try_as_mut()
            .expect("Failed to cast ModuleRef to mutable reference to type T")
    }

    ///
    /// Tries to borrow the referenced module as an mutable
    /// reference to the provided type T.
    ///
    /// This function will return `None` is the contained module
    /// is not of type T.
    ///
    /// # Panics
    ///
    /// This function panics if the contained module is allready borrowed
    /// in any way. This may be the case if another borrow has allready occured
    /// or the reference module is `self` and a module-specific function is called.
    ///
    #[must_use]
    pub fn try_as_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        RefMut::filter_map(
            self.processing.try_borrow_mut()
                .expect("could not aquire handle to node implementation, since the implementation is currently active"),
            |processor| {
                processor.downcast_element_mut::<T>()
            }
        ).ok()
    }
}

impl ModuleRef {
    /// Whether the module is currently active or shut down.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.ctx.state.get() != State::Shutdown
    }

    pub(crate) fn scope_token(&self) -> crate::tracing::ScopeToken {
        self.ctx.scope_token
    }

    /// INTERNAL
    #[doc(hidden)]
    #[must_use]
    pub fn activate(&self) -> Option<Arc<ModuleContext>> {
        enter_scope(self.scope_token());
        let prev = ModuleContext::place(Arc::clone(&self.ctx));
        #[cfg(debug_assertions)]
        if let Some(prev) = &prev {
            eprintln!("pushed-off ctx from {}", prev.path());
        }
        prev
    }

    /// INTERNAL
    #[doc(hidden)]
    #[allow(unused, clippy::unused_self)]
    pub(crate) fn deactivate(&self) {
        let _ = ModuleContext::take();
        leave_scope();
    }

    /// Creates a gate on the current module, returning its ID.
    ///
    #[must_use]
    pub fn create_gate(&self, name: &str) -> GateRef {
        self.create_gate_cluster(name, 1).remove(0)
    }

    ///
    /// Createas a cluster of gates on the current module returning their IDs.
    ///
    #[must_use]
    pub fn create_gate_cluster(&self, name: &str, size: usize) -> Vec<GateRef> {
        (0..size)
            .map(|id| self.create_raw_gate(name, size, id))
            .collect()
    }

    /// Creates a gate on the current module, returning its ID.
    ///
    #[must_use]
    pub fn create_raw_gate(&self, name: &str, size: usize, pos: usize) -> GateRef {
        let gate = Gate::new(self, name, size, pos);
        self.ctx.gates.write().push(gate.clone());
        gate
    }
}

impl PartialEq for ModuleRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.ctx, &other.ctx)
    }
}

impl Hash for ModuleRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ctx.hash(state);
    }
}

impl Debug for ModuleRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRef")
            .field("name", &self.ctx.path.to_string())
            .field("state", &self.ctx.state.get())
            .field("handler", &Arc::strong_count(&self.processing))
            .field("ctx", &Arc::strong_count(&self.ctx))
            .finish()
    }
}

unsafe impl Send for ModuleRef {}
unsafe impl Send for ModuleRefWeak {}

unsafe impl Sync for ModuleRef {}
unsafe impl Sync for ModuleRefWeak {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt() {
        let module = ModuleContext::new_standalone("root.a.b".into());
        let m2 = module.clone();
        let weak = ModuleRefWeak::new(&module);

        assert_eq!(module.path.as_str(), "root.a.b");
        assert_eq!(
            format!("{module:?}"),
            "ModuleRef { name: \"root.a.b\", state: Created, handler: 2, ctx: 2 }"
        );
        assert_eq!(
            format!("{weak:?}"),
            "ModuleRef { name: \"root.a.b\", state: Created, handler: 3, ctx: 3 }"
        );

        assert_eq!(module, m2);

        drop((m2, module));

        assert_eq!(format!("{weak:?}"), "Weak { .. }");
    }

    #[test]
    fn as_typed_ref() {
        #[derive(Debug, PartialEq)]
        struct A {
            inner: i32,
        }
        impl Module for A {}

        let module = ModuleContext::new_standalone("root".into());
        module.upgrade_dummy(ModuleImpl::new(
            ProcessingStack::default(),
            Box::new(A { inner: 42 }),
        ));

        assert!(module.try_as_ref::<i32>().is_none());
        assert!(module.try_as_mut::<i32>().is_none());

        module.as_mut::<A>().inner += 1;
        assert_eq!(*module.as_ref::<A>(), A { inner: 43 });
    }
}
