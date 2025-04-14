use des_net_utils::props::Cfg;
use serde_yml::{from_str, Value};

use crate::{
    net::{
        module::{module_ctx_drop, try_current, ModuleContext, ModuleExt, MOD_CTX},
        processing::ProcessingStack,
        topology::Topology,
    },
    prelude::{Application, EventLifecycle, GateRef, Module, ModuleRef, ObjectPath, Runtime},
    runtime::RuntimeError,
    time::SimTime,
    tracing::{enter_scope, leave_scope},
};
use std::{
    fmt::Debug,
    fs, io, mem, ops,
    panic::{set_hook, take_hook, PanicHookInfo},
    path::Path,
    sync::{Arc, Mutex, MutexGuard, TryLockError, Weak},
};

mod api;
pub use self::api::*;

mod events;
pub(crate) use self::events::*;

#[cfg(feature = "async")]
pub use self::events::JoinError;

mod ctx;
pub(crate) use self::ctx::*;

mod blocks;
pub use self::blocks::*;

mod unwind;
use self::unwind::Harness;
pub use self::unwind::PanicError;

use super::module::ModuleReferencingError;

static GUARD: Mutex<()> = Mutex::new(());

/// A networking simulation.
///
/// This type acts as both a builder for simulations, as well as the application object
/// used in the [`Runtime`].
///
/// A networking simulation can internally contain an application `A`,
/// that implements [`EventLifecycle`]. This type can be used attach
/// custom global behaviour at the simulation launch and shutdown. The
/// lifetime events will be applied after the simulation has started itself
/// and before the simulation itself will shut down.
///
/// However networking simulations allways use events of type `NetEvents`,
/// internally. These events do not interact with the inner application `A`.
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// # use des::net::HandlerFn;
/// struct Inner;
/// impl EventLifecycle<Sim<Inner>> for Inner {
///     fn at_sim_start(rt: &mut Runtime<Sim<Inner>>) {
///         println!("Hello simulation");
///         /* Do something */
///     }
/// }
///
/// let mut sim = Sim::new(Inner);
/// sim.node("alice", HandlerFn::new(|msg| {
///     /* Message processing */
/// }));
///
/// let _ = Builder::new().build(sim).run(); // prints 'Hello simulation'
/// ```
pub struct Sim<A> {
    pub(crate) stack: Box<dyn FnMut() -> ProcessingStack>,
    pub(crate) error: RuntimeError,

    modules: ModuleTree,
    pub(crate) cfgs: Vec<Cfg>,
    globals: Arc<Globals>,
    /// A inner field of a network simulation that can be used to attach
    /// custom lifetime handlers to a simulation
    pub inner: A,

    #[allow(unused)]
    guard: SimStaticsGuard,
}

#[derive(Debug)]
struct SimStaticsGuard {
    #[allow(unused)]
    guard: MutexGuard<'static, ()>,
}

impl SimStaticsGuard {
    fn new(globals: Weak<Globals>) -> Self {
        let guard = GUARD.try_lock();
        let guard = match guard {
            Ok(guard) => guard,
            Err(e) => match e {
                TryLockError::WouldBlock => GUARD.lock().unwrap_or_else(|e| {
                    eprintln!("net-sim lock poisnoed: rebuilding lock");
                    e.into_inner()
                }),
                TryLockError::Poisoned(poisoned) => {
                    eprintln!("net-sim lock poisoned: rebuilding lock");
                    poisoned.into_inner()
                }
            },
        };

        buf_init(globals);
        Self { guard }
    }
}

impl Drop for SimStaticsGuard {
    fn drop(&mut self) {
        buf_drop();
        module_ctx_drop();
    }
}

/// A helper to manage a scoped part of a networking simulation,
/// exclusivly used when building the simulation.
///
/// This type is helpful in combination with the trait [`ModuleBlock`]
/// to create reproducable blocks of modules at different
/// locations within the simulation.
///
/// This builder acts comparable to [`Sim`], but with an automatically
/// applied path prefix, the `scope`.
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// # use des::net::{ModuleBlock, ModuleFn, HandlerFn};
/// struct LAN {}
/// impl ModuleBlock for LAN {
///     type Ret = ();
///     fn build<A>(self, mut sim: ScopedSim<'_, A>) {
///         sim.root(HandlerFn::new(|_| {}));
///         let gates = sim.gates("", "port", 5);
///         for i in 0..5 {
///             let host = format!("host-{i}");
///             sim.node(&host, ModuleFn::new(
///                 /* ... */
///                 # || 123, |_, _| {}
///             ));
///             let gate = sim.gate(&host, "port");
///             gate.connect(gates[i].clone(), None);
///         }
///     }
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("google", LAN {});
/// sim.node("microsoft", LAN {});
/// sim.node("aws", HandlerFn::new(|_| {}));
/// sim.node("aws.us-east", LAN {});
///
/// let _ = Builder::new().build(sim).run();
/// ```
#[derive(Debug)]
pub struct ScopedSim<'a, A> {
    pub(crate) base: &'a mut Sim<A>,
    pub(crate) scope: ObjectPath,
}

impl<A> Sim<A> {
    /// Returns an iterator over all nodes in the simulation.
    pub fn nodes(&self) -> impl Iterator<Item = ObjectPath> + '_ {
        self.modules.iter().map(|ctx| &ctx.path).cloned()
    }

    #[inline]
    pub(crate) fn modules(&self) -> &ModuleTree {
        &self.modules
    }

    #[inline]
    pub(crate) fn modules_mut(&mut self) -> &mut ModuleTree {
        &mut self.modules
    }

    /// Creates a new network simulation, with an inner application `A`.
    ///
    /// This allready binds the simulation globals to this instance.
    pub fn new(inner: A) -> Self {
        let globals = Arc::new(Globals::default());
        let guard = SimStaticsGuard::new(Arc::downgrade(&globals));
        let stack: Box<dyn FnMut() -> ProcessingStack> = Box::new(ProcessingStack::default);
        Self {
            stack,
            error: RuntimeError::empty(),
            guard,
            modules: ModuleTree::default(),
            cfgs: Vec::new(),
            globals,
            inner,
        }
    }

    /// Sets the default processing stack for the simulation.
    ///
    /// Note that this will only affect calls of `node` after
    /// this function was called.
    pub fn set_stack<T: Into<ProcessingStack>>(&mut self, mut stack: impl FnMut() -> T + 'static) {
        let boxed: Box<dyn FnMut() -> ProcessingStack> = Box::new(move || stack().into());
        self.stack = boxed;
    }

    /// Sets the default processing stack for the simulation.
    ///
    /// Note that this will only affect calls of `node` after
    /// this function was called.
    #[must_use]
    pub fn with_stack<T: Into<ProcessingStack>>(
        mut self,
        stack: impl FnMut() -> T + 'static,
    ) -> Self {
        self.set_stack(stack);
        self
    }

    /// Includes raw parameter defintions in the simulation.
    ///
    /// If a parsing error is encountered, it will be silently
    /// ignored. Only successful parses will be applied to the
    /// module parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::ModuleFn;
    /// use std::net::IpAddr;
    ///
    /// let mut sim = Sim::new(());
    /// sim.include_cfg("alice.addr: 198.2.1.45\nalice.role: host");
    /// sim.node("alice", ModuleFn::new(
    ///     || {
    ///         let addr = current().prop::<Option<Ipv4Addr>>("addr").unwrap().get().unwrap();
    ///         let role = current().prop::<String>("role").unwrap().get();
    ///     },
    ///     |_, _| {}
    /// ));
    /// /*
    ///     Note that the order of the previous operations does not matter,
    ///     since the setup code will only be executed when the simulation
    ///     is startin, so on `Runtime::run`.
    /// */
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    pub fn include_cfg(&mut self, raw: &str) {
        if let Ok(value) = from_str::<Value>(raw) {
            let cfg = Cfg::new(value);

            // update config of already existing modules
            for module in self.modules.iter() {
                cfg.capture_for(
                    &module.path.as_str().split('.').collect::<Vec<_>>(),
                    &mut module.props.write(),
                );
            }
            self.cfgs.push(cfg);
        }
    }

    /// See [`Sim::include_par`]
    #[must_use]
    pub fn with_cfg(mut self, raw: &str) -> Self {
        self.include_cfg(raw);
        self
    }

    /// Tries to read and include parameters from a file into the simulation.
    ///
    /// See [`Sim::include_par`] for more infomation.
    ///
    /// # Errors
    ///
    /// This function may fail if the reading from a file fails.
    pub fn include_cfg_file(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.include_cfg(&fs::read_to_string(path)?);
        Ok(())
    }

    /// Returns a handle to the simulation globals.
    pub fn globals(&self) -> Arc<Globals> {
        self.globals.clone()
    }

    /// Returns the topology of the simulation.
    pub fn topology(&self) -> Topology<(), ()> {
        Topology::from_modules(&self.modules)
    }

    /// Creates a new module block within the simulation.
    ///
    /// A "node" is a block of modules at a given `path`. This may include:
    /// - no modules at all
    /// - just one module exactly at the given `path`
    /// - multiple modules, one at `path`, the others as direct or indirect children of this root module.
    ///
    /// The provided parameter `module_block` must be some type that implements the trait `ModuleBlock`.
    /// This trait can be used to create all components of the required block, within the local scope
    /// defined by `path`. Modules themself also implement `ModuleBlock` so modules themselfs can be
    /// build into a block of size 1.
    ///
    /// Custom implementations of `ModuleBlock` can not only create modules based
    /// on config data, but also gates and connections between these modules. Note
    /// that `ModuleBlock::build` is confined to the scope defined by `path`, since
    /// it uses a [`ScopedSim`] builder.
    ///
    /// See [`ScopedSim`] for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// struct MyModule {
    ///     state: i32,
    /// }
    /// impl Module for MyModule {
    ///     fn handle_message(&mut self, msg: Message) {
    ///         /* Do something */
    ///     }
    /// }
    ///
    /// let mut sim = Sim::new(());
    /// sim.node("alice", MyModule { state: 42 });
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    pub fn node<M: ModuleBlock>(&mut self, path: impl Into<ObjectPath>, module_block: M) -> M::Ret {
        let scoped = ScopedSim::new(self, path.into());
        module_block.build(scoped)
    }

    /// Retrieves a module by reference from the simulation.
    pub fn get(&self, path: &ObjectPath) -> Option<ModuleRef> {
        self.modules.get(path)
    }

    /// Creates a gate on a allready created module.
    ///
    /// The module will be defined `path` and the gate will be named `gate`.
    /// Should such a gate allready exist, the allready existing gate will be
    /// returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # struct SomeModule;
    /// # impl Module for SomeModule {}
    /// let mut sim = Sim::new(());
    /// sim.node("alice", SomeModule);
    /// sim.node("bob", SomeModule);
    ///
    /// let a = sim.gate("alice", "in");
    /// let b = sim.gate("bob", "out");
    ///
    /// b.connect(a, None);
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if node modules exists at `path`.
    pub fn gate(&mut self, path: impl Into<ObjectPath>, gate: &str) -> GateRef {
        let path = path.into();
        let Some(module) = self.get(&path) else {
            panic!("cannot create gate '{path}.{gate}', because node '{path}' does not exist")
        };
        if let Some(gate) = module.gate(gate, 0) {
            gate
        } else {
            module.create_gate(gate)
        }
    }

    /// Creates a clust of gate gate on a allready created module.
    ///
    /// The module will be defined `path` and the gate cluster will be named `gate`.
    /// Should such a gate cluster allready exist, the allready existing gate will be
    /// returned.
    ///
    /// # Panics
    ///
    /// This function panics if either, not module exists at `path`, or
    /// some parts of the gate cluster allready exist, but others do not.
    pub fn gates(&mut self, path: impl Into<ObjectPath>, gate: &str, size: usize) -> Vec<GateRef> {
        let path = path.into();
        let Some(module) = self.get(&path) else {
            panic!("cannot create gate '{path}.{gate}', because node '{path}' does not exist")
        };
        let mut gates = Vec::new();
        for k in 0..size {
            if let Some(gate) = module.gate(gate, k) {
                gates.push(gate);
            } else {
                break;
            }
        }
        if gates.len() == size {
            gates
        } else {
            assert!(
                gates.is_empty(),
                "cannot create gate cluster from partial gate cluster"
            );
            module.create_gate_cluster(gate, size)
        }
    }

    pub(super) fn raw(&mut self, path: ObjectPath, module: impl Module) -> ModuleRef {
        // Check dup
        assert!(
            self.modules.get(&path).is_none(),
            "cannot create node '{path}', node allready exists"
        );

        // Check node path location
        let ctx = if let Some(parent) = path.nonzero_parent() {
            // (a) Check that the parent exists
            let Some(parent) = self.get(&parent) else {
                panic!("cannot create node '{path}', since parent node '{parent}' is required, but does not exist");
            };

            ModuleContext::child_of(path.name(), parent)
        } else {
            ModuleContext::standalone(path)
        };

        // read in Props
        let path_parts = ctx.path.as_str().split('.').collect::<Vec<_>>();
        for cfg in &self.cfgs {
            cfg.capture_for(&path_parts, &mut ctx.props.write());
        }

        ctx.activate();
        let pe = module.to_processing_chain((self.stack)());
        ctx.upgrade_dummy(pe);

        self.globals
            .modules
            .lock()
            .expect("failed to lock globals")
            .push(ctx.clone());

        let mut sink = Vec::new();
        ctx.deactivate(&mut sink);
        assert!(
            sink.is_empty(),
            "events cannot be dispatched in constructors"
        );

        self.modules.add(ctx.clone());
        ctx
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl<A: Debug> Debug for Sim<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sim")
            .field("inner", &self.inner)
            .field("modules", &self.modules)
            .field("cfgs", &self.cfgs)
            .finish()
    }
}

impl<A> Drop for Sim<A> {
    fn drop(&mut self) {
        // SAFETY: Remove ctxs, since the next use of a `Sim` may occur on
        // a different thread
        unsafe {
            MOD_CTX.reset(None);
        }
    }
}

impl<'a, A> ScopedSim<'a, A> {
    pub(crate) fn new(base: &'a mut Sim<A>, scope: ObjectPath) -> Self {
        Self { base, scope }
    }

    #[allow(unused)]
    pub(crate) fn subscope(&mut self, path: impl AsRef<str>) -> ScopedSim<'_, A> {
        ScopedSim {
            base: &mut *self.base,
            scope: self.scope.appended(path),
        }
    }
}

impl<A> ScopedSim<'_, A> {
    /// The current scope from an absoute prespective.
    #[must_use]
    pub fn scope(&self) -> &ObjectPath {
        &self.scope
    }

    /// The inner application of the simulation `Sim<A>`.
    #[must_use]
    pub fn inner(&self) -> &A {
        &self.base.inner
    }

    /// Sets the current scope module.
    ///
    /// This call is equivalent to `sim.node(scope, <module_block>)` on [`Sim`].
    pub fn root(&mut self, module_block: impl Module) {
        self.base.raw(self.scope.clone(), module_block);
    }

    /// Creates a module block within the current scope.
    ///
    /// See [`Sim::node`] for more information.
    pub fn node(&mut self, path: impl Into<ObjectPath>, module_block: impl ModuleBlock) {
        self.base
            .node(self.scope.appended(path.into().as_str()), module_block);
    }

    /// Creates a gate on an existing node within the current scope.
    ///
    /// See [`Sim::gate`] for more information.
    pub fn gate(&mut self, path: impl Into<ObjectPath>, gate: &str) -> GateRef {
        self.base.gate(self.scope.appended(path.into()), gate)
    }

    /// Creates a cluster gate on an existing node within the current scope.
    ///
    /// See [`Sim::gates`] for more information.
    pub fn gates(&mut self, path: impl Into<ObjectPath>, gate: &str, size: usize) -> Vec<GateRef> {
        self.base
            .gates(self.scope.appended(path.into()), gate, size)
    }
}

impl<A> Application for Sim<A>
where
    A: EventLifecycle<Sim<A>>,
{
    type EventSet = NetEvents;
    type Lifecycle = SimLifecycle;
}

#[doc(hidden)]
#[derive(Debug)]
pub struct SimLifecycle;
impl<A> EventLifecycle<Sim<A>> for SimLifecycle
where
    A: EventLifecycle<Sim<A>>,
{
    fn at_sim_start(rt: &mut Runtime<Sim<A>>) {
        set_hook(Box::new(panic_hook));

        // (1) Get Topology
        let mut top = rt
            .app
            .globals
            .topology
            .lock()
            .expect("could not get topology lock");
        *top = Topology::from_modules(&rt.app.modules);
        drop(top);

        // (2) Run network-node sim_starting stages
        // - inline this to ensure this is run before any possible events

        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        let max_stage = rt
            .app
            .modules
            .iter()
            .fold(1, |acc, module| acc.max(module.num_sim_start_stages()));

        // (2.1) Call the stages in order, parallel over all modules
        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            for module in rt.app.modules.iter().cloned().collect::<Vec<_>>() {
                // Use cloned handles to appease the brwchk
                if stage < module.num_sim_start_stages() {
                    module.activate();

                    #[cfg(feature = "tracing")]
                    tracing::info!("Calling at_sim_start({}).", stage);

                    rt.app.error.extend(module.at_sim_start(stage).err());
                    module.deactivate(rt);

                    super::buf_process(&module, rt);
                }
            }
        }

        leave_scope();

        A::at_sim_start(rt);
    }

    fn at_sim_end(rt: &mut Runtime<Sim<A>>) -> Result<(), RuntimeError> {
        A::at_sim_end(rt)?;

        let mut error = RuntimeError::empty();
        mem::swap(&mut error, &mut rt.app.error);

        if !rt.app.error.is_empty() {
            return Err(error);
        }

        for module in rt.app.modules.iter().cloned().collect::<Vec<_>>() {
            enter_scope(module.scope_token());

            #[cfg(feature = "tracing")]
            tracing::info!("Calling 'at_sim_end'");
            module.activate();
            let _ = module.at_sim_end().map_err(|e| error.merge(e));
            module.deactivate(rt);

            // NOTE: no buf_process since no furthe events will be processed.
        }

        let _ = take_hook();
        leave_scope();
        if error.is_empty() {
            Ok(())
        } else {
            Err(error)
        }
    }
}

fn panic_hook(info: &PanicHookInfo) {
    if let Some(current) = try_current() {
        if let Some(location) = info.location() {
            eprintln!(
                "module '{}' panicked at {}:{}:{} after {}",
                current.path(),
                location.file(),
                location.line(),
                location.column(),
                SimTime::now()
            );
        }
    } else {
        eprintln!("thread 'main' panicked:");
    }

    if let Some(str) = info.payload().downcast_ref::<&str>() {
        eprintln!("{str}");
        return;
    }

    if let Some(str) = info.payload().downcast_ref::<String>() {
        eprintln!("{str}");
        return;
    }

    eprintln!("Box<dyn Any>");
}

///
/// The global parameters about a [`Sim`] that are publicly
/// exposed.
///
#[derive(Debug)]
pub struct Globals {
    /// The topology of the network from a module viewpoint.
    pub topology: Mutex<Topology<(), ()>>,

    /// Root modules
    pub(crate) modules: Mutex<Vec<ModuleRef>>,
}

impl Globals {
    /// Returns a handle to a module from the global scope.
    /// This can be used to access arbitrary modules, independent of the current execution context.
    ///
    /// # Errors
    ///
    /// Returns an module referencing error, if not module with the given path exists.
    #[allow(clippy::missing_panics_doc)]
    pub fn node(&self, path: impl Into<ObjectPath>) -> Result<ModuleRef, ModuleReferencingError> {
        let path = path.into();

        let modules = self.modules.lock().expect("failed to get lock");
        modules
            .iter()
            .find(|m| m.path == path)
            .ok_or(ModuleReferencingError::NoEntry(path.to_string()))
            .cloned()
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self {
            topology: Mutex::new(Topology::default()),
            modules: Mutex::new(Vec::default()),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ModuleTree {
    modules: Vec<ModuleRef>,
}

impl ModuleTree {
    pub(crate) fn get(&self, path: &ObjectPath) -> Option<ModuleRef> {
        self.modules
            .iter()
            .find(|module| module.path == *path)
            .cloned()
    }

    pub(crate) fn add(&mut self, module: ModuleRef) {
        if let Some(parent) = module.path.parent() {
            if parent.is_root() {
                // root either non existen or at index 0
                self.modules.push(module);
            } else {
                let parent_depth = parent.len();

                // search for parent insert at last possible position
                let Some(mut pos) = self.modules.iter().rposition(|m| m.path == parent) else {
                    panic!("cannot create node '{}', since parent node '{parent}' is required, but does not exist", module.path)
                };
                pos += 1;

                // (iter as long as we stay at path lengths > parent)
                while pos < self.modules.len() && self.modules[pos].path.len() > parent_depth {
                    pos += 1;
                }
                self.modules.insert(pos, module);
            }
        } else {
            // No parent
            self.modules.push(module);
        }
    }
}

impl ops::Deref for ModuleTree {
    type Target = [ModuleRef];
    fn deref(&self) -> &Self::Target {
        &self.modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_tree() {
        let mut tree = ModuleTree::default();
        fn module(path: &str) -> ModuleRef {
            ModuleContext::standalone(path.into())
        }

        tree.add(module("alice"));
        tree.add(module("alice.alicent"));
        tree.add(module("alice.john"));
        tree.add(module("alice.john.previous"));
        tree.add(module("bob"));
        tree.add(module("eve"));
        tree.add(module("eve.trevor"));
        tree.add(module("eve.trevor.list"));
        tree.add(module("eve.mark"));

        assert_eq!(
            tree.iter().map(|v| v.path.as_str()).collect::<Vec<_>>(),
            [
                "alice",
                "alice.alicent",
                "alice.john",
                "alice.john.previous",
                "bob",
                "eve",
                "eve.trevor",
                "eve.trevor.list",
                "eve.mark"
            ]
        );
    }
}
