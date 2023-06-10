use crate::net::NetEvents;
use crate::prelude::{ChannelRef, Gate, GateRef, GateServiceType};
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
    handler: Weak<RefCell<Box<dyn Module>>>,
    // handler_ptr: *mut u8,
}

impl ModuleRefWeak {
    pub(crate) fn new(strong: &ModuleRef) -> Self {
        Self {
            ctx: Arc::downgrade(&strong.ctx),
            handler: Arc::downgrade(&strong.handler),
            // handler_ptr: strong.handler_ptr,
        }
    }

    pub(crate) fn upgrade(&self) -> Option<ModuleRef> {
        Some(ModuleRef {
            ctx: self.ctx.upgrade()?,
            handler: self.handler.upgrade()?,
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
    pub(crate) handler: Arc<RefCell<Box<dyn Module>>>,
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
        let boxed = Box::new(module);
        let dynboxed: Box<dyn Module> = boxed;

        let handler = Arc::new(RefCell::new(dynboxed));

        Self { ctx, handler }
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
    pub fn upgrade_dummy(&self, module: Box<dyn Module>) {
        let celled = RefCell::new(module);
        let celled: RefCell<Box<dyn Module>> = celled;
        self.handler.swap(&celled);
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
        let brw = self.handler.borrow();
        let rf = &**brw;
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
                let hpt: *const dyn Module = &**brw;
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
        let brw = self.handler.borrow_mut();
        let rf = &**brw;
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
                let hpt: *mut dyn Module = &mut **brw;
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
            use crate::time::{Driver, SimTime};

            if let Some(prev) = prev {
                prev.async_ext.write().driver = Driver::unset();
            }

            let driver = self.ctx.async_ext.write().driver.take();
            driver.map(|mut d| {
                let bumpable = d.bump();
                if d.next_wakeup <= SimTime::now() {
                    d.next_wakeup = SimTime::MAX;
                }
                bumpable.into_iter().for_each(|s| s.wake_all());
                d.set();
            });
        }
    }

    // INTERNAL
    #[doc(hidden)]
    #[allow(unused)]
    pub(crate) fn deactivate(&self, rt: &mut impl EventSink<NetEvents>) {
        #[cfg(feature = "async")]
        {
            use crate::net::AsyncWakeupEvent;
            use crate::time::Driver;

            let mut ext = self.ctx.async_ext.write();
            let mut driver = Driver::unset().unwrap();
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
                    )
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
    pub fn create_gate(&self, name: &str, typ: GateServiceType) -> GateRef {
        self.create_gate_cluster(name, 1, typ).remove(0)
    }

    ///
    /// Creates a gate on the current module that points to another gate as its
    /// next hop, returning the ID of the created gate.
    ///
    #[must_use]
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
    #[must_use]
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
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
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

        let mut ids = Vec::new();

        for (i, item) in next_hops.into_iter().enumerate() {
            ids.push(self.create_raw_gate(name, typ, size, i, channel.clone(), item));
        }

        ids
    }

    /// Creates a gate on the current module, returning its ID.
    ///
    #[must_use]
    pub fn create_raw_gate(
        &self,
        name: &str,
        typ: GateServiceType,
        size: usize,
        pos: usize,
        channel: Option<ChannelRef>,
        next: Option<GateRef>,
    ) -> GateRef {
        let gate = Gate::new(self, name, typ, size, pos, channel, next);
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
            "ModuleRef {{ name: {}, handler: {}, ctx {} }}",
            self.ctx.path.name(),
            Arc::strong_count(&self.handler),
            Arc::strong_count(&self.ctx),
        ))
        .finish()
    }
}

unsafe impl Send for ModuleRef {}
unsafe impl Send for ModuleRefWeak {}

unsafe impl Sync for ModuleRef {}
unsafe impl Sync for ModuleRefWeak {}
