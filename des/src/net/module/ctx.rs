use super::{DummyModule, ModuleId, ModuleRef, ModuleRefWeak, Prop, PropType, Props, RawProp};
use crate::{
    net::{
        Error, ErrorKind,
        gate::IntoModuleGate,
        module::SignalCode,
        processing::ProcessingStack,
        runtime::{ModuleShutdownEvent, NetEvents, Spawner},
        schedule_event,
    },
    prelude::{GateRef, ObjectPath, current},
    sync::SwapLock,
    time::SimTime,
    tracing::{ScopeToken, new_scope},
};
use fxhash::{FxBuildHasher, FxHashMap};

use spin::RwLock;
use std::{cell::Cell, fmt::Debug, hash::Hash, sync::Arc, time::Duration};

pub(crate) static MOD_CTX: SwapLock<Option<Arc<ModuleContext>>> = SwapLock::new(None);

pub(crate) fn module_ctx_drop() {
    MOD_CTX.swap(&mut None);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum State {
    /// The context has been created, but no impl is yet attached.
    Created,
    /// The module impl has been attached, but `at_sim_start` has not yet been called.
    Initialized,
    /// The module impl & ctx are fully ready.
    Running,
    /// The module has been shutdown but not dropped.
    Shutdown,
}

/// The topological components of a module, not including the attached
/// software.
///
/// The term `within node-context` refers to the presence of a `ModuleContext`
/// in the global scope, that indicates that a module is currently active.
///
/// This type is internally used to create the simulations layout, but
/// creating module contexts on your own is highly discouraged, since
/// managing these structures is rather complicated. However the nessecary
/// constructors are still available, so use them with care.
pub struct ModuleContext {
    pub(crate) state: Cell<State>,
    pub(crate) id: ModuleId,

    pub(crate) me: RwLock<ModuleRefWeak>,

    pub(crate) path: ObjectPath,
    pub(crate) gates: RwLock<Vec<GateRef>>,

    pub(crate) props: RwLock<Props>,

    pub(crate) unwind_behaviour: Cell<UnwindBehaviour>,
    pub(crate) scope_token: ScopeToken,

    pub(crate) parent: Option<ModuleRefWeak>,
    pub(crate) children: RwLock<FxHashMap<String, ModuleRef>>,

    pub(crate) signal_subscribers: RwLock<FxHashMap<SignalCode, Vec<ModuleRefWeak>>>,
}

impl ModuleContext {
    /// Creates a new standalone instance of a new node.
    ///
    /// Note that this function returns a `ModuleRef`.
    /// A `ModuleRef` contains both the topological properties of a node
    /// if form of a `ModuleContext` as well as some attached software.
    /// The sofware attched to the returned reference is a dummy module
    /// that should be replaced before the simulation is started.
    #[must_use]
    pub fn standalone(path: ObjectPath) -> ModuleRef {
        ModuleRef::dummy(Arc::new(Self {
            me: RwLock::new(ModuleRefWeak::empty()),
            scope_token: new_scope(path.clone()),

            props: RwLock::new(Props::default()),

            state: Cell::new(State::Created),
            id: ModuleId::generate(),
            path,
            unwind_behaviour: Cell::default(),

            gates: RwLock::new(Vec::new()),

            parent: None,
            children: RwLock::default(),

            signal_subscribers: RwLock::default(),
        }))
    }

    /// Creates a instance within a module tree.
    ///
    /// Note that this function returns a `ModuleRef`.
    /// A `ModuleRef` contains both the topological properties of a node
    /// if form of a `ModuleContext` as well as some attached software.
    /// The sofware attched to the returned reference is a dummy module
    /// that should be replaced before the simulation is started.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn child_of(name: &str, parent: ModuleRef) -> ModuleRef {
        let path = ObjectPath::appended(&parent.ctx.path, name);
        let this = ModuleRef::dummy(Arc::new(Self {
            me: RwLock::new(ModuleRefWeak::empty()),
            scope_token: new_scope(path.clone()),

            props: RwLock::new(Props::default()),

            state: Cell::new(State::Created),
            id: ModuleId::generate(),
            path,
            unwind_behaviour: Cell::default(),

            gates: RwLock::new(Vec::new()),

            parent: Some(ModuleRefWeak::new(&parent)),
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),

            signal_subscribers: RwLock::default(),
        }));

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

    /// Indicates whether the module belonging to this context is currently active.
    pub fn is_currently_active(&self) -> bool {
        with_mod_ctx(|ctx| ctx.id == self.id)
    }

    /// Indicates whether the module belonging to this context is already initialized.
    pub fn is_initialized(&self) -> bool {
        self.me.read().upgrade().is_some()
    }

    /// Registers the currently active module as a subscriber to the given signal.
    pub fn subscribe_to(&self, signal: SignalCode) {
        self.signal_subscribers
            .write()
            .entry(signal)
            .or_default()
            .push(ModuleRefWeak::new(&current().me()));
    }

    /// Unregisters the currently active module as a subscriber to the given signal.
    pub fn unsubscribe_from(&self, signal: SignalCode) {
        let id = current().id();
        if let Some(v) = self.signal_subscribers.write().get_mut(&signal) {
            v.retain(|v| v.upgrade().is_some_and(|v| v.id != id));
        }
    }

    /// Shuts down all activity for the module.
    ///
    /// > *This function requires a node-context within the simulation*
    ///
    /// A module that is shut down, will not longer be able to
    /// handle incoming messages, or run any user-defined code.
    /// All plugin activity will be suspendend. However the
    /// custom state will be kept for debug purposes.
    ///
    /// This function must be used within a module context
    /// otherwise its effects should be consider UB.
    pub fn shutdown(&self) {
        schedule_event(
            NetEvents::ModuleShutdownEvent(ModuleShutdownEvent {
                module: self.me(),
                restart_at: None,
            }),
            SimTime::now(),
        );
    }

    /// Shuts down all activity for the module.
    /// Restarts after the given duration.
    ///
    /// > *This function requires a node-context within the simulation*
    ///
    /// On restart the module will be reinitalized
    /// using `Module::reset`  and then `Module::at_sim_start`.
    /// Use the reset function to get the custom state to a resonable default
    /// state, which may or may not be defined by `Module::new`.
    /// However you can simulate persistent-beyond-shutdown data
    /// by not reseting this data in `Module::reset`.
    ///
    /// ```
    /// # use des::prelude::*;
    /// # type Data = usize;
    /// struct MyModule {
    ///     volatile: Data,
    ///     persistent: Data,
    /// }
    ///
    /// impl Module for MyModule {
    ///     fn reset(&mut self) {
    ///         self.volatile = 0;
    ///     }
    ///
    ///     fn at_sim_start(&mut self, _: usize) {
    ///         println!(
    ///             "Start at {} with volatile := {} and persistent := {}",
    ///             SimTime::now(),
    ///             self.volatile,
    ///             self.persistent
    ///         );
    ///
    ///         self.volatile = 42;
    ///         self.persistent = 1024;
    ///
    ///         if SimTime::now() == SimTime::ZERO {
    ///             current().shutdow_and_restart_in(Duration::from_secs(10));
    ///         }
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let app = /* ... */
    /// #    Sim::new(());
    ///     let rt = Builder::new().build(app.freeze()).run();
    ///     // outputs 'Start at 0s with volatile := 0 and persistent := 0'
    ///     // outputs 'Start at 10s with volatile := 0 and persistent := 1024'
    /// }
    /// ```
    ///
    /// [`Module::reset`]: crate::net::module::Module::reset
    /// [`Module::at_sim_start`]: crate::net::module::Module::at_sim_start
    pub fn shutdow_and_restart_in(&self, dur: Duration) {
        schedule_event(
            NetEvents::ModuleShutdownEvent(ModuleShutdownEvent {
                module: self.me(),
                restart_at: Some(SimTime::now() + dur),
            }),
            SimTime::now(),
        );
    }

    /// Shuts down all activity for the module.
    /// Restarts at the given time.
    ///
    /// > *This function requires a node-context within the simulation*
    ///
    /// The user must ensure that the restart time
    /// point is greater or equal to the current simtime.
    ///
    /// See [`shutdow_and_restart_in`](ModuleContext::shutdow_and_restart_in) for more information.
    pub fn shutdow_and_restart_at(&self, restart_at: SimTime) {
        schedule_event(
            NetEvents::ModuleShutdownEvent(ModuleShutdownEvent {
                module: self.me(),
                restart_at: Some(restart_at),
            }),
            SimTime::now(),
        );
    }

    /// Creates a new [`Spawner`] for the module.
    ///
    /// > *This function requires a node-context within the simulation*
    ///
    /// Must provide a stack, just like in `SimBuilder`
    ///
    /// # Panics
    ///
    /// Panics if the module is not yet initialized.
    pub fn spawner<F: Fn() -> ProcessingStack + 'static>(&self, stack: F) -> Spawner<'_, ()> {
        assert!(
            self.is_initialized(),
            "cannot use spawner on a not yet initialized module"
        );
        Spawner::new_at_runtime(self, stack)
    }

    /// Returns a runtime-unqiue identifier for the currently active module.
    ///
    /// # Example
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// struct MyModule;
    /// impl Module for MyModule {
    ///     fn handle_message(&mut self, msg: Message) {
    ///         let id = current().id();
    ///         assert_eq!(id, msg.header.receiver_module_id);
    ///     }
    /// }
    /// ```
    ///
    /// [`Module`]: crate::net::module::Module
    pub fn id(&self) -> ModuleId {
        self.id
    }

    /// Returns a runtime-unqiue identifier for the currently active module,
    /// based on its place in the module graph.
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// struct MyModule;
    /// impl Module for MyModule {
    ///     fn handle_message(&mut self, msg: Message) {
    ///         let path = current().path();
    ///         println!("[{path}] recv message: {}", msg)
    ///     }
    /// }
    /// ```
    ///
    /// [`Module`]: crate::net::module::Module
    pub fn path(&self) -> ObjectPath {
        self.path.clone()
    }

    /// Returns the `ModuleRef` associated with this context.
    ///
    /// # Panics
    ///
    /// Cannot be called during teardown.
    pub fn me(&self) -> ModuleRef {
        self.me.read().upgrade().expect("cannot upgrade")
    }

    /// Returns a handle to a typed property on this module.
    ///
    /// See [`Prop`] for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// struct ModuleWithProps;
    /// impl Module for ModuleWithProps {
    ///     fn at_sim_start(&mut self, _: usize) {
    ///         let sid = current().prop::<u32>("sid").expect("cannot retrive prop");
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function is a shorthand for `prop_raw(key).typed::<T>()`.
    /// See [`RawProp::typed`] for information on errors.
    pub fn prop<T: PropType>(&self, key: &str) -> Result<Prop<T>, Error> {
        self.props.write().get(key)
    }

    /// Returns a untyped property handle for the property under the given key.
    ///
    /// See [`RawProp`] for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// struct ModuleWithProps;
    /// impl Module for ModuleWithProps {
    ///     fn at_sim_start(&mut self, _: usize) {
    ///         let sid = current().prop_raw("cfg").as_value();
    ///         //...
    ///     }
    /// }
    /// ```
    pub fn prop_raw(&self, key: &str) -> RawProp {
        self.props.write().get_raw(key)
    }

    /// Returns the keys to all available props.
    pub fn props_keys(&self) -> Vec<String> {
        self.props.read().keys()
    }

    /// Returns the name for the currently active module.
    ///
    /// Note that the module name is just the last component of the module
    /// path.
    pub fn name(&self) -> String {
        self.path.name().to_string()
    }

    /// Returns a unstructured list of all gates from the current module.
    pub fn gates(&self) -> Vec<GateRef> {
        self.gates.read().clone()
    }

    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    pub fn gate(&self, desc: impl IntoModuleGate) -> Option<GateRef> {
        desc.as_gate(self)
    }

    /// Returns the unwind behaviour of this module.
    ///
    /// # Panics
    ///
    /// Panics when concurrently accesed from multiple threads.
    pub fn unwind_behaviour(&self) -> UnwindBehaviour {
        self.unwind_behaviour.get()
    }

    /// Sets the unwind behaviour of this module.
    ///
    /// # Panics
    ///
    /// Panics when concurrently accesed from multiple threads.
    pub fn set_unwind_behaviour(&self, new: UnwindBehaviour) {
        self.unwind_behaviour.set(new);
    }

    /// Returns a reference to a parent module
    ///
    /// Use this handle to either access the parent modules topological
    /// state, or cast it to access the custom state of the parent.
    ///
    /// # Errors
    ///
    /// Returns an error if no parent exists, or
    /// the parent is currently shut down.
    ///
    /// # Panics
    ///
    /// May panic when the simulation is currently being dropped.
    pub fn parent(&self) -> Result<ModuleRef, Error> {
        if let Some(ref parent) = self.parent {
            let strong = parent
                .upgrade()
                .expect("Failed to fetch parent, ptr missing in drop");

            if !strong.is_active() {
                return Err(Error::new(
                    self.path.clone(),
                    ErrorKind::ModuleNotFound(
                        "the parent module is currently inactive, thus cannot be accessed".into(),
                    ),
                ));
            }

            if strong.try_as_ref::<DummyModule>().is_some() {
                Err(Error::new(
                    self.path.clone(),
                    ErrorKind::ModuleNotFound(
                        "the parent module is not yet initalized, thus cannot be accessed".into(),
                    ),
                ))
            } else {
                Ok(strong)
            }
        } else {
            Err(Error::new(
                self.path.clone(),
                ErrorKind::ModuleNotFound("no parent module exists".into()),
            ))
        }
    }

    /// Returns a handle to the child element, with the provided module name.
    ///
    /// Use this handle to either access and modify the childs modules topological
    /// state, or cast it to access its custom state .
    ///
    /// # Errors
    ///
    /// Returns an error if no child was found under the given name,
    /// or the child is currently shut down.
    pub fn child(&self, name: &str) -> Result<ModuleRef, Error> {
        if let Some(child) = self.children.read().get(name) {
            if !child.is_active() {
                return Err(Error::new(
                    self.path.clone(),
                    ErrorKind::ModuleNotFound(format!(
                        "the child module '{name}' is currently inactive, thus cannot be accessed"
                    )),
                ));
            }

            Ok(child.clone())
        } else {
            Err(Error::new(
                self.path.clone(),
                ErrorKind::ModuleNotFound(format!("the child module '{name}' does not exist")),
            ))
        }
    }
}

// FIXME:
// Since the module ctx is available from all other modules, none of the APIs
// should assume that self is the currently active module context.
//
// Some however do:
// - spawner

cfg_async! {
    use tokio::task::JoinHandle;
    use crate::net::processing::TokioRuntime;

    impl ModuleContext {
        /// Schedules a task to be joined when the simulatio ends
        ///
        /// This function will **not** block, but rather defer the joining
        /// to the simulation shutdown phase.
        ///
        /// # Panics
        ///
        /// Panics if the module context is not the currently active module context.
        pub fn join(&self, handle: JoinHandle<()>) {
            assert!(self.is_currently_active(), "Cannot add join handle to the join group of another module");
            TokioRuntime::join(handle);
        }

        /// Will try to join a task when the simulation ends.
        ///
        /// This will catch panics that occured within the task, but
        /// if the task is still running, no error will be returned.
        ///
        /// # Panics
        ///
        /// Panics if the module context is not the currently active module context.
        pub fn try_join(&self, handle: JoinHandle<()>) {
            assert!(self.is_currently_active(), "Cannot add join handle to the join group of another module");
            TokioRuntime::try_join(handle);
        }
    }
}

impl Debug for ModuleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleContext").finish()
    }
}

impl Hash for ModuleContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.path.hash(state);
    }
}

unsafe impl Send for ModuleContext {}
unsafe impl Sync for ModuleContext {}

impl Drop for ModuleContext {
    fn drop(&mut self) {
        for gate in self.gates() {
            gate.dissolve_paths();
        }
    }
}

/// A config that defines a nodes behaviour on startup, shutdown or panic.
///
/// The lifecycle of a node is defined as follows:
/// 1. A node is created (potentially from non-sim context)
/// 2. A node is started (`at_sim_start`)
/// 3. A node runs.
/// 4. The software panics (shutdown/restart is also treated as a panic???)
///  1. Node is either restarted or dropped
///  2. Children are droped if flag is set
///  3. Parent is informed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct UnwindBehaviour {
    /// Indicates whether to catch an panic and allow the simulation to continue without error
    /// or to record the module panic as an error in the runtime result.
    pub on_panic_catch: bool,
    /// Indicates whether a node should be restared if it panicked.
    pub on_panic_restart: bool,
    /// Indicates whether a panic in this module should shut down all submodules.
    pub on_panic_drop_submodules: bool,
}

impl UnwindBehaviour {
    /// The default behaviour of an independent node.
    ///
    /// Failures will be recorded & restarts attempted.
    /// Submodules will be dropped in upon panic.
    pub const HOST: UnwindBehaviour = UnwindBehaviour {
        on_panic_catch: false,
        on_panic_restart: true,
        on_panic_drop_submodules: true,
    };

    /// The default behaviour of a subprocess node.
    ///
    /// Failures will not be recorded & restarts not attempted.
    /// Submodules will be dropped in upon panic.
    ///
    /// Handeling the failure is the resposiblity of the managing node
    /// (aka the 'parent' process).
    pub const SUBPROCESS: UnwindBehaviour = UnwindBehaviour {
        on_panic_catch: true,
        on_panic_restart: false,
        on_panic_drop_submodules: true,
    };
}

impl Default for UnwindBehaviour {
    fn default() -> Self {
        Self::HOST
    }
}

// Panic behaviour
// on_panic -> catch-stop / catch-restart / unwind
// on_panic submodules -> drop / keep
// on_panic parent -> inform / NOP

pub(crate) fn with_mod_ctx<R>(f: impl FnOnce(&Arc<ModuleContext>) -> R) -> R {
    let lock = MOD_CTX.read();
    let ctx = lock
        .as_ref()
        .expect("failed operation: no module currently in scope");
    let r = f(ctx);
    drop(lock);
    r
}

pub(crate) fn try_with_mod_ctx<R>(f: impl FnOnce(&Arc<ModuleContext>) -> R) -> Option<R> {
    let lock = MOD_CTX.read();
    if let Some(ctx) = lock.as_real_inner() {
        let r = f(ctx);
        drop(lock);
        Some(r)
    } else {
        None
    }
}
