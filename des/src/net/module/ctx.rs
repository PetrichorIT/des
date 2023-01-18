use super::{DummyModule, ModuleId, ModuleRef, ModuleRefWeak, ModuleReferencingError};
use crate::prelude::{GateRef, ObjectPath};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

use crate::net::plugin::*;

#[cfg(feature = "async")]
use crate::net::module::core::AsyncCoreExt;

pub(crate) static MOD_CTX: spin::RwLock<Option<Arc<ModuleContext>>> = spin::RwLock::new(None);
pub(crate) static SETUP_FN: spin::Mutex<fn(&ModuleContext)> = spin::Mutex::new(_default_setup);

#[cfg(not(feature = "async"))]
fn _default_setup(_: &ModuleContext) {}

#[cfg(feature = "async")]
fn _default_setup(this: &ModuleContext) {
    this.add_plugin(TokioTimePlugin::new(this.path.path().to_string()), 0, false);
}

/// INTERNAL
pub struct ModuleContext {
    pub(crate) active: AtomicBool,
    pub(crate) id: ModuleId,

    pub(crate) path: ObjectPath,
    pub(crate) gates: spin::RwLock<Vec<GateRef>>,
    pub(crate) plugins: spin::RwLock<Vec<PluginEntry>>,

    #[cfg(feature = "async")]
    pub(crate) async_ext: spin::RwLock<AsyncCoreExt>,
    pub(crate) parent: Option<ModuleRefWeak>,
    pub(crate) children: spin::RwLock<HashMap<String, ModuleRef>>,
}

impl ModuleContext {
    /// Creates a new standalone instance
    pub fn standalone(path: ObjectPath) -> ModuleRef {
        let this = ModuleRef::dummy(Arc::new(Self {
            active: AtomicBool::new(true),

            id: ModuleId::gen(),
            path,
            gates: spin::RwLock::new(Vec::new()),
            plugins: spin::RwLock::new(Vec::new()),

            parent: None,
            children: spin::RwLock::new(HashMap::new()),

            #[cfg(feature = "async")]
            async_ext: spin::RwLock::new(AsyncCoreExt::new()),
        }));

        SETUP_FN.lock()(&this);

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
            gates: spin::RwLock::new(Vec::new()),
            plugins: spin::RwLock::new(Vec::new()),

            parent: Some(ModuleRefWeak::new(&parent)),
            children: spin::RwLock::new(HashMap::new()),

            #[cfg(feature = "async")]
            async_ext: spin::RwLock::new(AsyncCoreExt::new()),
        }));

        SETUP_FN.lock()(&this);

        parent
            .ctx
            .children
            .write()
            .insert(name.to_string(), this.clone());

        this
    }

    pub(crate) fn place(self: Arc<Self>) -> Option<Arc<ModuleContext>> {
        MOD_CTX.write().replace(self)
    }

    pub(crate) fn take() -> Option<Arc<ModuleContext>> {
        MOD_CTX.write().take()
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
        self.gates.read().clone()
    }
    /// INTERNAL
    pub fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        self.gates
            .read()
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
        if let Some(child) = self.children.read().get(name) {
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

pub(crate) fn with_mod_ctx<R>(f: impl FnOnce(&Arc<ModuleContext>) -> R) -> R {
    let lock = MOD_CTX.read();
    let r = f(&*lock.as_ref().unwrap());
    drop(lock);
    r
}

cfg_async! {
    use tokio::runtime::Runtime;
    use tokio::task::JoinHandle;
    use tokio::sync::mpsc::{UnboundedReceiver, error::SendError};
    use super::ext::WaitingMessage;

    pub(super) fn async_get_rt() -> Option<Arc<Runtime>> {
        with_mod_ctx(|ctx| Some(Arc::clone(ctx.async_ext.read().rt.as_ref()?)))
    }

    pub(super) fn async_ctx_reset() {
        with_mod_ctx(|ctx| ctx.async_ext.write().reset());
    }

    // Wait queue

    pub(super) fn async_wait_queue_tx_send(msg: WaitingMessage) -> Result<(), SendError<WaitingMessage>> {
        with_mod_ctx(|ctx| ctx.async_ext.write().wait_queue_tx.send(msg))
    }

    pub(super) fn async_wait_queue_rx_take() -> Option<UnboundedReceiver<WaitingMessage>> {
        with_mod_ctx(|ctx| ctx.async_ext.write().wait_queue_rx.take())
    }

    pub(super) fn async_set_wait_queue_join(join: JoinHandle<()>) {
        with_mod_ctx(|ctx| ctx.async_ext.write().wait_queue_join = Some(join));
    }

    // Sim Staart

    pub(super) fn async_sim_start_rx_take() -> Option<UnboundedReceiver<usize>> {
        with_mod_ctx(|ctx| ctx.async_ext.write().sim_start_rx.take())
    }

    pub(super) fn async_set_sim_start_join(join: JoinHandle<()>) {
        with_mod_ctx(|ctx| ctx.async_ext.write().sim_start_join = Some(join));
    }

    pub(super) fn async_sim_start_tx_send(stage: usize) -> Result<(), SendError<usize>>  {
        with_mod_ctx(|ctx| ctx.async_ext.write().sim_start_tx.send(stage))
    }

    pub(super) fn async_sim_start_join_take() -> Option<JoinHandle<()>> {
        with_mod_ctx(|ctx| ctx.async_ext.write().sim_start_join.take())
    }

    // SIM END

    pub(super) fn async_sim_end_join_set(join: JoinHandle<()>)  {
        with_mod_ctx(|ctx| ctx.async_ext.write().sim_end_join = Some(join));
    }

    pub(super) fn async_sim_end_join_take() -> Option<JoinHandle<()>> {
        with_mod_ctx(|ctx| ctx.async_ext.write().sim_end_join.take())
    }
}
