use super::{DummyModule, ModuleId, ModuleRef, ModuleRefWeak, ModuleReferencingError};
use crate::prelude::{GateRef, ObjectPath};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

use crate::net::plugin::*;

#[cfg(feature = "async")]
use crate::net::module::core::AsyncCoreExt;

thread_local! {
    static MOD_CTX: RefCell<Option<Arc<ModuleContext>>> = const { RefCell::new(None) }
}

thread_local! {
    pub (crate) static SETUP_FN: RefCell<fn(&ModuleContext)> =  const { RefCell::new(_default_setup) };
}

#[cfg(not(feature = "async"))]
fn _default_setup(_: &ModuleContext) {}

#[cfg(feature = "async")]
fn _default_setup(this: &ModuleContext) {
    this.add_plugin(TokioTimePlugin::new(this.path.path().to_string()), 0, false);
    this.add_plugin(TokioNetPlugin::new(), 1, false);
}

/// INTERNAL
pub struct ModuleContext {
    pub(crate) active: AtomicBool,
    pub(crate) id: ModuleId,

    pub(crate) path: ObjectPath,
    pub(crate) gates: RefCell<Vec<GateRef>>,
    pub(crate) plugins: RefCell<Vec<PluginEntry>>,

    #[cfg(feature = "async")]
    pub(crate) async_ext: RefCell<AsyncCoreExt>,
    pub(crate) parent: Option<ModuleRefWeak>,
    pub(crate) children: RefCell<HashMap<String, ModuleRef>>,
}

impl ModuleContext {
    /// Creates a new standalone instance
    pub fn standalone(path: ObjectPath) -> ModuleRef {
        let this = ModuleRef::dummy(Arc::new(Self {
            active: AtomicBool::new(true),

            id: ModuleId::gen(),
            path,
            gates: RefCell::new(Vec::new()),
            plugins: RefCell::new(Vec::new()),

            parent: None,
            children: RefCell::new(HashMap::new()),

            #[cfg(feature = "async")]
            async_ext: RefCell::new(AsyncCoreExt::new()),
        }));

        SETUP_FN.with(|f| f.borrow()(&this));

        this
    }

    /// Creates a child
    #[allow(clippy::needless_pass_by_value)]
    pub fn child_of(name: &str, parent: ModuleRef) -> ModuleRef {
        let path = ObjectPath::module_with_parent(name, &parent.ctx.path);
        let this = ModuleRef::dummy(Arc::new(Self {
            active: AtomicBool::new(true),

            id: ModuleId::gen(),
            path,
            gates: RefCell::new(Vec::new()),
            plugins: RefCell::new(Vec::new()),

            parent: Some(ModuleRefWeak::new(&parent)),
            children: RefCell::new(HashMap::new()),

            #[cfg(feature = "async")]
            async_ext: RefCell::new(AsyncCoreExt::new()),
        }));

        SETUP_FN.with(|f| f.borrow()(&this));

        parent
            .ctx
            .children
            .borrow_mut()
            .insert(name.to_string(), this.clone());

        this
    }

    pub(crate) fn place(self: Arc<Self>) -> Option<Arc<ModuleContext>> {
        // println!("Now active module: {}", self.path.path());
        MOD_CTX.with(|ctx| ctx.borrow_mut().replace(self))
    }

    pub(crate) fn take() -> Option<Arc<ModuleContext>> {
        MOD_CTX.with(|ctx| ctx.take())
    }

    /// INTERNAL
    pub fn id(&self) -> ModuleId {
        self.id
    }

    /// INTERNAL
    pub fn path(&self) -> ObjectPath {
        self.path.clone()
    }
    /// INTERNAL
    pub fn name(&self) -> String {
        self.path.name().to_string()
    }
    /// INTERNAL
    pub fn gates(&self) -> Vec<GateRef> {
        self.gates.borrow().clone()
    }
    /// INTERNAL
    pub fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        self.gates
            .borrow()
            .iter()
            .find(|&g| g.name() == name && g.pos() == pos)
            .cloned()
    }
    /// INTERNAL
    pub fn parent(&self) -> Result<ModuleRef, ModuleReferencingError> {
        if let Some(ref parent) = self.parent {
            let strong = parent
                .upgrade()
                .expect("Failed to fetch parent, ptr missing in drop");

            if strong.try_as_ref::<DummyModule>().is_some() {
                Err(ModuleReferencingError::NotYetInitalized(format!("The parent ptr of module '{}' is existent but not yet initalized, according to the load order.", self.path)))
            } else {
                Ok(strong)
            }
        } else {
            Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a parent ptr",
                self.path
            )))
        }
    }
    /// INTERNAL
    pub fn child(&self, name: &str) -> Result<ModuleRef, ModuleReferencingError> {
        dbg!(self.path.path(), self.children.borrow());
        if let Some(child) = self.children.borrow().get(name) {
            Ok(child.clone())
        } else {
            Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a child ptr with the name '{}'",
                self.path, name
            )))
        }
    }
}

impl Debug for ModuleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleContext").finish()
    }
}

// impl Drop for ModuleContext {
//     fn drop(&mut self) {
//         println!("<DROP> dropping module ctx '{}'", self.path)
//     }
// }

pub(crate) fn with_mod_ctx<R>(f: impl FnOnce(Ref<Arc<ModuleContext>>) -> R) -> R {
    MOD_CTX.with(|ctx| {
        let brw = ctx.borrow();
        let brw = Ref::map(brw, |v| v.as_ref().unwrap());
        f(brw)
    })
}

cfg_async! {
    use tokio::runtime::Runtime;
    use tokio::task::JoinHandle;
    use tokio::sync::mpsc::{UnboundedReceiver, error::SendError};
    use super::ext::WaitingMessage;

    pub(super) fn async_get_rt() -> Option<Arc<Runtime>> {
        with_mod_ctx(|ctx| Some(Arc::clone(ctx.async_ext.borrow().rt.as_ref()?)))
    }

    pub(super) fn async_ctx_reset() {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().reset());
    }

    // Wait queue

    pub(super) fn async_wait_queue_tx_send(msg: WaitingMessage) -> Result<(), SendError<WaitingMessage>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().wait_queue_tx.send(msg))
    }

    pub(super) fn async_wait_queue_rx_take() -> Option<UnboundedReceiver<WaitingMessage>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().wait_queue_rx.take())
    }

    pub(super) fn async_set_wait_queue_join(join: JoinHandle<()>) {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().wait_queue_join = Some(join));
    }

    // Sim Staart

    pub(super) fn async_sim_start_rx_take() -> Option<UnboundedReceiver<usize>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_rx.take())
    }

    pub(super) fn async_set_sim_start_join(join: JoinHandle<()>) {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_join = Some(join));
    }

    pub(super) fn async_sim_start_tx_send(stage: usize) -> Result<(), SendError<usize>>  {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_tx.send(stage))
    }

    pub(super) fn async_sim_start_join_take() -> Option<JoinHandle<()>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_join.take())
    }

    // SIM END

    pub(super) fn async_sim_end_join_set(join: JoinHandle<()>)  {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_end_join = Some(join));
    }

    pub(super) fn async_sim_end_join_take() -> Option<JoinHandle<()>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_end_join.take())
    }
}
