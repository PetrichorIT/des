use crate::net::processing::ProcessingElements;
use crate::net::NetEvents;
use crate::prelude::{Gate, GateRef};
use crate::runtime::EventSink;
use crate::tracing::{enter_scope, leave_scope};

use super::{DummyModule, Module, ModuleContext};
use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, Weak};

#[derive(Clone)]
pub(crate) struct ModuleRefWeak {
    ctx: Weak<ModuleContext>,
    handler: Weak<RefCell<ProcessingElements>>,
    // handler_ptr: *mut u8,
}

impl ModuleRefWeak {
    pub(crate) fn new(strong: &ModuleRef) -> Self {
        Self {
            ctx: Arc::downgrade(&strong.ctx),
            handler: Arc::downgrade(&strong.processing),
            // handler_ptr: strong.handler_ptr,
        }
    }

    pub(crate) fn upgrade(&self) -> Option<ModuleRef> {
        Some(ModuleRef {
            ctx: self.ctx.upgrade()?,
            processing: self.handler.upgrade()?,
            // handler_ptr: self.handler_ptr,
        })
    }
}

impl Debug for ModuleRefWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRefWeak").finish()
    }
}

/// A reference to a module
#[derive(Clone)]
pub struct ModuleRef {
    pub(crate) ctx: Arc<ModuleContext>,
    pub(crate) processing: Arc<RefCell<ProcessingElements>>,
}

impl Deref for ModuleRef {
    type Target = ModuleContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl ModuleRef {
    #[allow(clippy::explicit_deref_methods)]
    pub(crate) fn new<T: Module>(ctx: Arc<ModuleContext>, module: T) -> Self {
        let procesing = module.to_processing_chain();
        let handler = Arc::new(RefCell::new(procesing));
        Self {
            ctx,
            processing: handler,
        }
    }

    #[allow(unused)]
    pub(crate) fn dummy(ctx: Arc<ModuleContext>) -> Self {
        // Create the dummy module explicitly not with ::new since
        // all dyn Module calls would panic
        Self::new(ctx, DummyModule {})
    }

    #[allow(unused)]
    // Caller must ensure that handler is indeed a dummy
    #[doc(hidden)]
    pub fn upgrade_dummy(&self, module: ProcessingElements) {
        let celled = RefCell::new(module);
        let celled: RefCell<ProcessingElements> = celled;
        self.processing.swap(&celled);
    }

    // NOTE / TODO
    // Once feature(trait_upcasting) is stabalized, use traitupcasting for
    // safe interactions with the v-table.
    // For now us raw pointer casts.

    /// Borrows the referenced module as a readonly reference
    /// to the provided type T.
    ///
    /// # Panics
    ///
    /// Panics if either the module is not of type T,
    /// or the module is allready borrowed mutably.
    #[must_use]
    pub fn as_ref<T: Any>(&self) -> Ref<T> {
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
    pub fn try_as_ref<T: Any>(&self) -> Option<Ref<T>> {
        let brw = self.processing.borrow();
        let rf = &*brw.handler;
        let ty = rf.type_id();
        if ty == TypeId::of::<T>() {
            // SAFTEY:
            // The pointer 'handler_ptr' will allways point to the object
            // refered to by the 'handler': Since 'handler' is owned through
            // an 'Arc' its memory position will NOT changed. Thus 'handler_ptr'
            // allways points to valid memory. Pointer aligment is guranteed.
            //
            // Since the created &T is encapluslated in a Ref<&T> this functions acts as
            // a call of 'RefCell::borrow' thus upholding the borrowing invariants.
            //
            // Should the type check fail, the Ref is dropped so the borrow is freed.
            Some(Ref::map(brw, |brw| unsafe {
                let hpt: *const dyn Module = &*brw.handler;
                // hpt.cast::<T>()
                // &*(hpt as *const T)
                &*(hpt.cast::<T>())
            }))
        } else {
            None
        }
    }

    /// Borrows the referenced module as a mutable reference
    /// to the provided type T.
    ///
    /// # Panics
    ///
    /// Panics if either the module is not of type T,
    /// or the module is allready borrowed on any way.
    #[must_use]
    pub fn as_mut<T: Any>(&self) -> RefMut<T> {
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
    pub fn try_as_mut<T: Any>(&self) -> Option<RefMut<T>> {
        let brw = self.processing.borrow_mut();
        let rf = &*brw.handler;
        let ty = rf.type_id();
        if ty == TypeId::of::<T>() {
            // SAFTEY:
            // The pointer 'handler_ptr' will allways point to the object
            // refered to by the 'handler': Since 'handler' is owned through
            // an 'Arc' its memory position will NOT changed. Thus 'handler_ptr'
            // allways points to valid memory. Pointer aligment is guranteed.
            //
            // Since the created &T is encapluslated in a Ref<&T> this functions acts as
            // a call of 'RefCell::borrow' thus upholding the borrowing invariants.
            //
            // Should the type check fail, the Ref is dropped so the borrow is freed.
            Some(RefMut::map(brw, |brw| unsafe {
                let hpt: *mut dyn Module = &mut *brw.handler;
                &mut *(hpt.cast::<T>())
            }))
        } else {
            None
        }
    }
}

impl ModuleRef {
    pub(crate) fn is_active(&self) -> bool {
        self.ctx.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) fn as_str(&self) -> &str {
        self.ctx.path.as_str()
    }

    pub(crate) fn scope_token(&self) -> crate::tracing::ScopeToken {
        self.ctx.scope_token
    }

    /// INTERNAL
    #[doc(hidden)]
    #[allow(unused)]
    pub fn activate(&self) {
        enter_scope(self.scope_token());
        let prev = ModuleContext::place(Arc::clone(&self.ctx));

        #[cfg(feature = "async")]
        {
            use crate::time::{Driver, SimTime, TimerSlot};

            if let Some(prev) = prev {
                prev.async_ext.write().driver = Driver::unset();
            }

            let driver = self.ctx.async_ext.write().driver.take();
            if let Some(mut d) = driver {
                let bumpable = d.bump();
                if d.next_wakeup <= SimTime::now() {
                    d.next_wakeup = SimTime::MAX;
                }
                bumpable.into_iter().for_each(TimerSlot::wake_all);
                d.set();
            }
        }
    }

    /// INTERNAL
    #[doc(hidden)]
    #[allow(unused)]
    pub(crate) fn deactivate(&self, rt: &mut impl EventSink<NetEvents>) {
        #[cfg(feature = "async")]
        {
            use crate::net::AsyncWakeupEvent;
            use crate::time::Driver;

            let mut ext = self.ctx.async_ext.write();
            let Some(mut driver) = Driver::unset() else {
                // Somebody stole our driver
                #[cfg(feature = "tracing")]
                tracing::error!("IO time driver missing after event execution");

                ext.driver = Some(Driver::new());
                return;
            };
            if let Some(next_wakeup) = driver.next() {
                if next_wakeup < driver.next_wakeup {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(
                        "scheduling new wakeup at {} (prev {})",
                        next_wakeup,
                        driver.next_wakeup
                    );

                    driver.next_wakeup = next_wakeup;
                    rt.add(
                        NetEvents::AsyncWakeupEvent(AsyncWakeupEvent {
                            module: self.clone(),
                        }),
                        next_wakeup,
                    );
                }
            }
            ext.driver = Some(driver);
        }

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

impl Debug for ModuleRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!(
            "ModuleRef {{ name: {}, handler: {}, ctx: {} }}",
            self.ctx.path,
            Arc::strong_count(&self.processing),
            Arc::strong_count(&self.ctx),
        ))
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
        let module = ModuleContext::standalone("root.a.b".into());
        let m2 = module.clone();
        let weak = ModuleRefWeak::new(&module);

        assert_eq!(module.as_str(), "root.a.b");
        assert_eq!(
            format!("{module:?}"),
            "ModuleRef { name: root.a.b, handler: 2, ctx: 2 }"
        );
        assert_eq!(format!("{weak:?}"), "ModuleRefWeak");

        assert_eq!(module, m2);
    }

    #[test]
    fn as_typed_ref() {
        #[derive(Debug, PartialEq)]
        struct A {
            inner: i32,
        }
        impl Module for A {}

        let module = ModuleContext::standalone("root".into());
        module.upgrade_dummy(ProcessingElements::new(Vec::new(), A { inner: 42 }));

        assert!(module.try_as_ref::<i32>().is_none());
        assert!(module.try_as_mut::<i32>().is_none());

        module.as_mut::<A>().inner += 1;
        assert_eq!(*module.as_ref::<A>(), A { inner: 43 });
    }
}
