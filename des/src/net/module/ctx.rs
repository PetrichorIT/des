use fxhash::{FxBuildHasher, FxHashMap};
use spin::RwLock;

use super::{
    meta::Metadata, DummyModule, ModuleId, ModuleRef, ModuleRefWeak, ModuleReferencingError,
};
use crate::{
    prelude::{GateRef, ObjectPath},
    sync::SwapLock,
    tracing::{new_scope, ScopeToken},
};
use std::{
    any::Any,
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

#[cfg(feature = "async")]
use crate::net::module::rt::AsyncCoreExt;

pub(crate) static MOD_CTX: SwapLock<Option<Arc<ModuleContext>>> = SwapLock::new(None);

pub(crate) fn module_ctx_drop() {
    MOD_CTX.swap(&mut None);
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
    pub(crate) active: AtomicBool,
    pub(crate) id: ModuleId,

    pub(crate) path: ObjectPath,
    pub(crate) gates: RwLock<Vec<GateRef>>,

    pub(super) meta: RwLock<Metadata>,
    pub(crate) scope_token: ScopeToken,

    #[cfg(feature = "async")]
    pub(crate) async_ext: RwLock<AsyncCoreExt>,
    pub(crate) parent: Option<ModuleRefWeak>,
    pub(crate) children: RwLock<FxHashMap<String, ModuleRef>>,
}

impl ModuleContext {
    /// Creates a new standalone instance of a new node.
    ///
    /// Note that this function returns a `ModuleRef`.
    /// A `ModuleRef` contains both the topological properties of a node
    /// if form of a `ModuleContext` as well as some attached software.
    /// The sofware attched to the returned reference is a dummy module
    /// that should be replaced before the simulation is started.
    pub fn standalone(path: ObjectPath) -> ModuleRef {
        let this = ModuleRef::dummy(Arc::new(Self {
            #[cfg(feature = "async")]
            async_ext: RwLock::new(AsyncCoreExt::new()),

            meta: RwLock::new(Metadata::new()),
            scope_token: new_scope(path.clone()),

            active: AtomicBool::new(true),
            id: ModuleId::gen(),
            path,

            gates: RwLock::new(Vec::new()),

            parent: None,
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),
        }));

        this
    }

    /// Creates a instance within a module tree.
    ///  
    /// Note that this function returns a `ModuleRef`.
    /// A `ModuleRef` contains both the topological properties of a node
    /// if form of a `ModuleContext` as well as some attached software.
    /// The sofware attched to the returned reference is a dummy module
    /// that should be replaced before the simulation is started.
    #[allow(clippy::needless_pass_by_value)]
    pub fn child_of(name: &str, parent: ModuleRef) -> ModuleRef {
        let path = ObjectPath::appended(&parent.ctx.path, name);
        let this = ModuleRef::dummy(Arc::new(Self {
            #[cfg(feature = "async")]
            async_ext: RwLock::new(AsyncCoreExt::new()),

            meta: RwLock::new(Metadata::new()),
            scope_token: new_scope(path.clone()),

            active: AtomicBool::new(true),

            id: ModuleId::gen(),
            path,

            gates: RwLock::new(Vec::new()),

            parent: Some(ModuleRefWeak::new(&parent)),
            children: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),
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
    ///         println!("[{path}] recv message: {}", msg.str())  
    ///     }
    /// }
    /// ```
    ///
    /// [`Module`]: crate::net::module::Module
    pub fn path(&self) -> ObjectPath {
        self.path.clone()
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

    /// Retrieves metadata about a module, based on a type.
    ///
    /// # Examples
    ///
    /// # Panics
    ///
    /// Panics when concurrently accessed from multiple threads.
    pub fn meta<T: Any + Clone>(&self) -> Option<T> {
        Some(
            self.meta
                .try_read()
                .expect("Failed lock")
                .get::<T>()?
                .clone(),
        )
    }

    /// Sets a metadata object.
    ///
    /// # Panics
    ///
    /// Panics when concurrently accessed from multiple threads.
    pub fn set_meta<T: Any + Clone>(&self, value: T) {
        self.meta.try_write().expect("Failed lock").set(value);
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
