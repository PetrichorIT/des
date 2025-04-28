use super::{DummyModule, ModuleId, ModuleRef, ModuleRefWeak, ModuleReferencingError};
use crate::{
    prelude::{GateRef, ObjectPath},
    sync::SwapLock,
    time::SimTime,
    tracing::{new_scope, ScopeToken},
};
use des_net_utils::props::{Prop, PropType, Props, RawProp};
use fxhash::{FxBuildHasher, FxHashMap};

use spawner::Spawner;
use spin::RwLock;
use std::{
    cell::Cell,
    fmt::Debug,
    hash::Hash,
    io::Error,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub(crate) static MOD_CTX: SwapLock<Option<Arc<ModuleContext>>> = SwapLock::new(None);

pub(crate) fn module_ctx_drop() {
    MOD_CTX.swap(&mut None);
}

cfg_async! {
    pub(super) mod rt;
    use self::rt::AsyncCoreExt;
}

mod spawner;
mod stereotyp;

pub use stereotyp::Stereotyp;

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
    pub(crate) active: AtomicBool,
    pub(crate) id: ModuleId,

    pub(crate) me: RwLock<Option<ModuleRefWeak>>,

    pub(crate) path: ObjectPath,
    pub(crate) gates: RwLock<Vec<GateRef>>,

    pub(crate) props: RwLock<Props>,

    pub(crate) stereotyp: Cell<Stereotyp>,
    pub(crate) scope_token: ScopeToken,

    #[cfg(feature = "async")]
    pub(crate) async_ext: RwLock<AsyncCoreExt>,
    pub(crate) parent: Option<ModuleRefWeak>,
    pub(crate) children: RwLock<FxHashMap<String, ModuleRef>>,

    // RUNTIME VALUES
    #[allow(clippy::option_option)]
    pub(crate) shutdown_task: RwLock<Option<Option<SimTime>>>,
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
            #[cfg(feature = "async")]
            async_ext: RwLock::new(AsyncCoreExt::new()),

            me: RwLock::new(None),
            scope_token: new_scope(path.clone()),

            props: RwLock::new(Props::default()),

            active: AtomicBool::new(true),
            id: ModuleId::gen(),
            path,
            stereotyp: Cell::default(),

            gates: RwLock::new(Vec::new()),

            parent: None,
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),

            shutdown_task: RwLock::default(),
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
            #[cfg(feature = "async")]
            async_ext: RwLock::new(AsyncCoreExt::new()),

            me: RwLock::new(None),
            scope_token: new_scope(path.clone()),

            props: RwLock::new(Props::default()),

            active: AtomicBool::new(true),
            id: ModuleId::gen(),
            path,
            stereotyp: Cell::default(),

            gates: RwLock::new(Vec::new()),

            parent: Some(ModuleRefWeak::new(&parent)),
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),

            shutdown_task: RwLock::default(),
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
        *self.shutdown_task.write() = Some(None);
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
    ///     let rt = Builder::new().build(app).run();
    ///     // outputs 'Start at 0s with volatile := 0 and persistent := 0'
    ///     // outputs 'Start at 10s with volatile := 0 and persistent := 1024'
    /// }
    /// ```
    ///
    /// [`Module::new`]: crate::net::module::Module::new
    /// [`Module::reset`]: crate::net::module::Module::reset
    /// [`Module::at_sim_start`]: crate::net::module::Module::at_sim_start
    pub fn shutdow_and_restart_in(&self, dur: Duration) {
        *self.shutdown_task.write() = Some(Some(SimTime::now() + dur));
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
        *self.shutdown_task.write() = Some(Some(restart_at));
    }

    /// TODO
    pub fn spawner(&self) -> Spawner<'_> {
        Spawner { ctx: self }
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
    ///         assert_eq!(id, msg.header().receiver_module_id);
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
        self.me
            .read()
            .as_ref()
            .expect("failed")
            .upgrade()
            .expect("cannot upgrade")
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
    pub fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        self.gates
            .read()
            .iter()
            .find(|&g| g.name() == name && g.pos() == pos)
            .cloned()
    }

    /// Returns the unwind behaviour of this module.
    ///
    /// # Panics
    ///
    /// Panics when concurrently accesed from multiple threads.
    pub fn stereotyp(&self) -> Stereotyp {
        self.stereotyp.get()
    }

    /// Sets the unwind behaviour of this module.
    ///
    /// # Panics
    ///
    /// Panics when concurrently accesed from multiple threads.
    pub fn set_stereotyp(&self, new: Stereotyp) {
        self.stereotyp.set(new);
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

    /// Returns a handle to the child element, with the provided module name.
    ///
    /// Use this handle to either access and modify the childs modules topological
    /// state, or cast it to access its custom state .
    ///
    /// # Errors
    ///
    /// Returns an error if no child was found under the given name,
    /// or the child is currently shut down.
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
