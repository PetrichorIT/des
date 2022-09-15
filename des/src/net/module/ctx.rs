use super::{ModuleId, ModuleRef, ModuleRefWeak, ModuleReferencingError};
use crate::{
    net::{
        common::Optional,
        gate::IntoModuleGate,
        globals,
        runtime::{buf_schedule_at, buf_schedule_shutdown, buf_send_at},
        ParHandle,
    },
    prelude::{Duration, GateRef, Message, ObjectPath, SimTime},
};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

#[cfg(feature = "async")]
use super::ext::AsyncCoreExt;
#[cfg(feature = "async")]
use tokio::sim::SimContext;

thread_local! {
    static MOD_CTX: RefCell<Option<Arc<ModuleContext>>> = const { RefCell::new(None) }
}

#[allow(missing_docs)]
pub struct ModuleContext {
    pub(crate) id: ModuleId,

    /// A human readable identifier for the module.
    pub(crate) path: ObjectPath,

    /// A collection of all gates register to the current module
    pub(crate) gates: RefCell<Vec<GateRef>>,

    /// Expensions for async
    #[cfg(feature = "async")]
    pub(crate) async_ext: RefCell<AsyncCoreExt>,

    /// The reference for the parent module.
    pub(crate) parent: Option<ModuleRefWeak>,

    /// The collection of child nodes for the current module.
    pub(crate) children: HashMap<String, ModuleRef>,
}

impl ModuleContext {
    /// Creates a new standalone instance
    pub fn standalone(path: ObjectPath) -> Self {
        #[cfg(feature = "async")]
        let ident = path.path().to_string();

        Self {
            id: ModuleId::gen(),
            path,
            gates: RefCell::new(Vec::new()),
            parent: None,
            children: HashMap::new(),

            #[cfg(feature = "async")]
            async_ext: RefCell::new(AsyncCoreExt::new(ident)),
        }
    }

    /// Creates a child
    pub fn child_of(name: &str, parent: ModuleRef) -> Self {
        let path = ObjectPath::module_with_parent(name, &parent.ctx.path);
        #[cfg(feature = "async")]
        let ident = path.path().to_string();

        // TODO set parents child

        Self {
            id: ModuleId::gen(),
            path,
            gates: RefCell::new(Vec::new()),
            parent: Some(ModuleRefWeak::new(&parent)),
            children: HashMap::new(),

            #[cfg(feature = "async")]
            async_ext: RefCell::new(AsyncCoreExt::new(ident)),
        }
    }

    pub(crate) fn place(self: Arc<Self>) -> Option<Arc<ModuleContext>> {
        log::trace!("Now active module: {}", self.path.path());
        MOD_CTX.with(|ctx| ctx.borrow_mut().replace(self))
    }

    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn path(&self) -> ObjectPath {
        self.path.clone()
    }

    pub fn name(&self) -> String {
        self.path.name().to_string()
    }

    pub fn gates(&self) -> Vec<GateRef> {
        self.gates.borrow().clone()
    }

    pub fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        self.gates
            .borrow()
            .iter()
            .find(|&g| g.name() == name && g.pos() == pos)
            .cloned()
    }

    pub fn parent(&self) -> Result<ModuleRef, ModuleReferencingError> {
        if let Some(ref parent) = self.parent {
            Ok(parent
                .upgrade()
                .expect("Failed to fetch parent, ptr missing in drop"))
        } else {
            Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a parent ptr",
                self.path
            )))
        }
    }

    pub fn child(&self, name: &str) -> Result<ModuleRef, ModuleReferencingError> {
        if let Some(child) = self.children.get(name) {
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

fn with_mod_ctx<R>(f: impl FnOnce(Ref<Arc<ModuleContext>>) -> R) -> R {
    MOD_CTX.with(|ctx| {
        let brw = ctx.borrow();
        let brw = Ref::map(brw, |v| v.as_ref().unwrap());
        f(brw)
    })
}

/// A runtime-unqiue identifier for this module-core and by extension this module.
pub fn module_id() -> ModuleId {
    with_mod_ctx(|ctx| ctx.id())
}

/// A runtime-unqiue (not enforced) identifier for this module, based on its place in the module tree.
pub fn module_path() -> ObjectPath {
    with_mod_ctx(|ctx| ctx.path())
}

/// Returns the name of the module instance.
pub fn module_name() -> String {
    with_mod_ctx(|ctx| ctx.name())
}

// PARENT CHILD

/// Returns the parent element
pub fn parent() -> Result<ModuleRef, ModuleReferencingError> {
    with_mod_ctx(|ctx| ctx.parent())
}

/// Returns the child element.
pub fn child(name: &str) -> Result<ModuleRef, ModuleReferencingError> {
    with_mod_ctx(|ctx| ctx.child(name))
}

// GATE RELATED

///
/// Returns a ref unstructured list of all gates from the current module.
///
pub fn gates() -> Vec<GateRef> {
    with_mod_ctx(|ctx| ctx.gates())
}

///
/// Returns a ref to a gate of the current module dependent on its name and cluster position
/// if possible.
///
pub fn gate(name: &str, pos: usize) -> Option<GateRef> {
    with_mod_ctx(|ctx| ctx.gate(name, pos))
}

// BUF CTX based

///
/// Sends a message onto a given gate. This operation will be performed after
/// `handle_message` finished.
///
pub fn send(msg: impl Into<Message>, gate: impl IntoModuleGate) {
    self::send_at(msg, gate, SimTime::now())
}

///
/// Sends a message onto a given gate with a delay. This operation will be performed after
/// `handle_message` finished.
///
pub fn send_in(msg: impl Into<Message>, gate: impl IntoModuleGate, dur: Duration) {
    self::send_at(msg, gate, SimTime::now() + dur)
}
///
/// Sends a message onto a given gate at the sepcified time. This operation will be performed after
/// `handle_message` finished.
///
pub fn send_at(msg: impl Into<Message>, gate: impl IntoModuleGate, send_time: SimTime) {
    assert!(send_time >= SimTime::now());
    // (0) Cast the message.
    let msg: Message = msg.into();

    let gate = with_mod_ctx(|ctx| {
        // (1) Cast the gate
        let gate = gate.as_gate(&*ctx);
        // (3) Return the results (DO NOT USE BOTH CONTEXTES AT THE SAME TIME)
        gate
    });

    if let Some(gate) = gate {
        buf_send_at(msg, gate, send_time)
    } else {
        log::error!("Error: Could not find gate in current module")
    }
}

///
/// Enqueues a event that will trigger the [`Module::handle_message`] function
/// in duration seconds, shifted by the processing time delay.
///
pub fn schedule_in(msg: impl Into<Message>, dur: Duration) {
    self::schedule_at(msg, SimTime::now() + dur)
}

///
/// Enqueues a event that will trigger the [`Module::handle_message`] function
/// at the given `SimTime`
///
pub fn schedule_at(msg: impl Into<Message>, arrival_time: SimTime) {
    assert!(arrival_time >= SimTime::now());
    let msg: Message = msg.into();
    buf_schedule_at(msg, arrival_time)
}

///
/// Shuts down all activity for the module.
///
pub fn shutdown() {
    buf_schedule_shutdown(None)
}

///
/// Shuts down all activity for the module.
/// Restarts after the given duration.
///
pub fn shutdow_and_restart_in(dur: Duration) {
    self::shutdow_and_restart_at(SimTime::now() + dur)
}

///
/// Shuts down all activity for the module.
/// Restarts at the given time.
///
pub fn shutdow_and_restart_at(restart_at: SimTime) {
    buf_schedule_shutdown(Some(restart_at));
}

///
/// Returns the parameters for the current module.
///
pub fn pars() -> HashMap<String, String> {
    let path = self::module_path();
    globals().parameters.get_def_table(path.path())
}

///
/// Returns a parameter by reference (not parsed).
///
pub fn par(key: &str) -> ParHandle<Optional> {
    globals()
        .parameters
        .get_handle(self::module_path().path(), key)
}

cfg_async! {
    use tokio::runtime::Runtime;
    use tokio::task::JoinHandle;
    use tokio::sync::mpsc::{UnboundedReceiver, error::SendError};
    use super::ext::WaitingMessage;

    #[cfg(not(feature = "async-sharedrt"))]
    pub(super) fn async_get_rt() -> Option<Arc<Runtime>> {
        with_mod_ctx(|ctx| Some(Arc::clone(ctx.async_ext.borrow().rt.as_ref()?)))
    }

    #[cfg(feature = "async-sharedrt")]
    pub(super) fn async_get_rt() -> Option<Arc<Runtime>> {
         Some(Arc::clone(&globals().runtime))
    }

    pub(super) fn async_take_sim_ctx() -> SimContext {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().ctx.take().expect("Sombody stole our sim context"))
    }

    pub(super) fn async_leave_sim_ctx(sim_ctx: SimContext) {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().ctx = Some(sim_ctx))
    }

    #[cfg(not(feature = "async-sharedrt"))]
    pub(super) fn async_ctx_reset() {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().reset())
    }

    // Wait queue

    pub(super) fn async_wait_queue_tx_send(msg: WaitingMessage) -> Result<(), SendError<WaitingMessage>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().wait_queue_tx.send(msg))
    }

    pub(super) fn async_wait_queue_rx_take() -> Option<UnboundedReceiver<WaitingMessage>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().wait_queue_rx.take())
    }

    pub(super) fn async_set_wait_queue_join(join: JoinHandle<()>) {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().wait_queue_join = Some(join))
    }

    // Sim Staart

    pub(super) fn async_sim_start_rx_take() -> Option<UnboundedReceiver<usize>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_rx.take())
    }

    pub(super) fn async_set_sim_start_join(join: JoinHandle<()>) {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_join = Some(join))
    }

    pub(super) fn async_sim_start_tx_send(stage: usize) -> Result<(), SendError<usize>>  {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_tx.send(stage))
    }

    pub(super) fn async_sim_start_join_take() -> Option<JoinHandle<()>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_start_join.take())
    }

    // SIM END

    pub(super) fn async_sim_end_join_set(join: JoinHandle<()>)  {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_end_join = Some(join))
    }

    pub(super) fn async_sim_end_join_take() -> Option<JoinHandle<()>> {
        with_mod_ctx(|ctx| ctx.async_ext.borrow_mut().sim_end_join.take())
    }
}
