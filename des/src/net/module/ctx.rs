use fxhash::{FxBuildHasher, FxHashMap};

use super::{DummyModule, ModuleId, ModuleRef, ModuleRefWeak, ModuleReferencingError};
use crate::{
    net::plugin::PluginRegistry,
    prelude::{GateRef, ObjectPath},
    sync::{RwLock, SwapLock, SwapLockReadGuard},
};
use std::{
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

#[cfg(feature = "async")]
use crate::net::module::core::AsyncCoreExt;

pub(crate) static MOD_CTX: SwapLock<Option<Arc<ModuleContext>>> = SwapLock::new(None);
pub(crate) static SETUP_FN: RwLock<fn(&ModuleContext)> = RwLock::new(_default_setup);

pub(crate) fn _default_setup(_: &ModuleContext) {}

///
pub struct ModuleContext {
    pub(crate) active: AtomicBool,
    pub(crate) id: ModuleId,

    pub(crate) path: ObjectPath,
    pub(crate) gates: RwLock<Vec<GateRef>>,
    pub(crate) plugins: RwLock<PluginRegistry>,

    #[cfg(feature = "tracing")]
    pub(crate) scope_token: crate::tracing::ScopeToken,

    #[cfg(feature = "async")]
    pub(crate) async_ext: RwLock<AsyncCoreExt>,
    pub(crate) parent: Option<ModuleRefWeak>,
    pub(crate) children: RwLock<FxHashMap<String, ModuleRef>>,
}

impl ModuleContext {
    /// Creates a new standalone instance
    pub fn standalone(path: ObjectPath) -> ModuleRef {
        let this = ModuleRef::dummy(Arc::new(Self {
            #[cfg(feature = "tracing")]
            scope_token: crate::tracing::new_scope(path.as_str()),

            active: AtomicBool::new(true),
            id: ModuleId::gen(),
            path,

            gates: RwLock::new(Vec::new()),
            plugins: RwLock::new(PluginRegistry::new()),

            parent: None,
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),

            #[cfg(feature = "async")]
            async_ext: RwLock::new(AsyncCoreExt::new()),
        }));

        SETUP_FN.read()(&this);

        this
    }

    /// Creates a child
    #[allow(clippy::needless_pass_by_value)]
    pub fn child_of(name: &str, parent: ModuleRef) -> ModuleRef {
        let path = ObjectPath::appended(&parent.ctx.path, name);
        let this = ModuleRef::dummy(Arc::new(Self {
            #[cfg(feature = "tracing")]
            scope_token: crate::tracing::new_scope(path.as_str()),

            active: AtomicBool::new(true),

            id: ModuleId::gen(),
            path,

            gates: RwLock::new(Vec::new()),
            plugins: RwLock::new(PluginRegistry::new()),

            parent: Some(ModuleRefWeak::new(&parent)),
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),

            #[cfg(feature = "async")]
            async_ext: RwLock::new(AsyncCoreExt::new()),
        }));

        SETUP_FN.read()(&this);

        parent
            .ctx
            .children
            .write()
            .insert(name.to_string(), this.clone());

        this
    }

    pub(crate) fn place(self: Arc<Self>) -> Option<Arc<ModuleContext>> {
        let mut this = Some(self);
        MOD_CTX.swap(&mut this);
        this
    }

    pub(crate) fn take() -> Option<Arc<ModuleContext>> {
        let mut this = None;
        MOD_CTX.swap(&mut this);
        this
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
    /// Returns a reference to a parent module
    ///
    /// # Errors
    ///
    /// Returns an error if no parent exists, or is not yet initalized
    /// (see load order).
    pub fn parent(&self) -> Result<ModuleRef, ModuleReferencingError> {
        if let Some(ref parent) = self.parent {
            let strong = parent
                .upgrade()
                .expect("Failed to fetch parent, ptr missing in drop");

            if !strong.is_active() {
                return Err(ModuleReferencingError::CurrentlyInactive(format!(
                    "The parent module of '{}' is currently shut down, thus cannot be accessed",
                    self.path,
                )));
            }

            if strong.try_as_ref::<DummyModule>().is_some() {
                Err(ModuleReferencingError::NotYetInitalized(
                    format!("The parent ptr of module '{}' is existent but not yet initalized, according to the load order.", self.path)
                ))
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
    /// Returns a reference to a child module.
    ///
    /// # Errors
    ///
    /// Returns an error if no child with the provided name exists.
    pub fn child(&self, name: &str) -> Result<ModuleRef, ModuleReferencingError> {
        if let Some(child) = self.children.read().get(name) {
            if !child.is_active() {
                return Err(ModuleReferencingError::CurrentlyInactive(format!(
                    "The child module '{}' of '{}' is currently shut down, thus cannot be accessed",
                    name, self.path,
                )));
            }

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

unsafe impl Send for ModuleContext {}
unsafe impl Sync for ModuleContext {}

pub(crate) fn with_mod_ctx<R>(f: impl FnOnce(&Arc<ModuleContext>) -> R) -> R {
    let lock = MOD_CTX.read();
    let r = f(&lock);
    drop(lock);
    r
}

pub(crate) fn with_mod_ctx_lock() -> SwapLockReadGuard<'static, Option<Arc<ModuleContext>>> {
    MOD_CTX.read()
}

cfg_async! {
    use tokio::runtime::Runtime;
    use tokio::task::JoinHandle;
    use tokio::task::LocalSet;
    use tokio::sync::mpsc::{UnboundedReceiver, error::SendError};
    use super::ext::WaitingMessage;

    pub(crate) fn async_get_rt() -> Option<(Arc<Runtime>, Arc<LocalSet>)> {
        with_mod_ctx(|ctx| Some(ctx.async_ext.read().rt.clone()?))
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
