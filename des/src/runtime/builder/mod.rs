#[cfg(feature = "cqueue")]
use std::time::Duration;
use std::{fmt, fs, io, iter::from_fn, ops::Deref, path::Path, sync::Arc};

use rand::{RngCore, SeedableRng};
use serde_norway::{Value, from_str};

use crate::{
    ObjectPath, Sim,
    gate::{GateClusterRef, GateRef},
    module::{Cfg, ModuleRef, UnwindBehaviour},
    processing::ProcessingStack,
    runtime::{
        IntoModuleTree, RunParameters, State, bench::Profiler, event::FutureEventSet,
        limit::RuntimeLimit, set_rng,
    },
    time::SimTime,
};

mod cfg;
mod globals;
mod guard;
mod spawner;

pub(crate) use self::cfg::SimConfiguration;
pub(crate) use self::globals::ModuleRoots;
pub(crate) use self::guard::SimGuard;

pub use self::globals::Globals;
pub use self::spawner::{Spawner, SpawnerKind};

/// A builder wrapping a `Sim` object.
///
/// This builder essential implements a construction function as follows:
/// ```ignore
/// fn build_node(ctx: ModuleContext, module_impl: impl Module) -> ModuleRef;
/// ```
#[must_use]
pub struct SimBuilder<A> {
    quiet: bool,
    rng: Box<dyn RngCore>,
    limit: RuntimeLimit,
    start_time: SimTime,

    #[cfg(feature = "cqueue")]
    pub(super) cqueue_num_buckets: usize,
    #[cfg(feature = "cqueue")]
    pub(super) cqueue_bucket_timespan: Duration,

    app: A,
    cfg: SimConfiguration,
    globals: Arc<Globals>,
}

impl<A> SimBuilder<A> {
    /// Creates a new simulation builder with the given application.
    pub fn new(app: A) -> Self {
        SimBuilder {
            quiet: false,
            rng: Box::new(rand::rngs::StdRng::from_rng(
                &mut rand::rngs::ThreadRng::default(),
            )),
            limit: RuntimeLimit::None,
            start_time: SimTime::ZERO,

            #[cfg(feature = "cqueue")]
            cqueue_num_buckets: 1028,
            #[cfg(feature = "cqueue")]
            cqueue_bucket_timespan: Duration::from_secs_f64(0.0025),

            app,
            cfg: SimConfiguration {
                stack: Arc::new(ProcessingStack::default),
                default_unwind_behavior: UnwindBehaviour::default(),
            },

            globals: Arc::default(),
        }
    }

    /// Sets the seed for the random number generator.
    pub fn seeded(mut self, seed: u64) -> Self {
        self.rng = Box::new(rand::rngs::StdRng::seed_from_u64(seed));
        self
    }

    ///
    /// Sets the cqueue options if this runtime uses a cqueue.
    /// NOP otherwise.
    ///
    #[cfg(feature = "cqueue")]
    pub fn cqueue_options(mut self, n: usize, t: Duration) -> Self {
        self.cqueue_num_buckets = n;
        self.cqueue_bucket_timespan = t;

        self
    }

    ///
    /// Suppressed runtime messages from the simulation framework.
    ///
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    ///
    /// Changes the maximum iteration number of a runtime.
    ///
    pub fn start_time(mut self, time: SimTime) -> Self {
        self.start_time = time;
        self
    }

    ///
    /// Changes the maximum iteration number of a runtime.
    ///
    pub fn max_itr(mut self, max_itr: usize) -> Self {
        self.limit.add(RuntimeLimit::EventCount(max_itr));
        self
    }

    ///
    /// Changes the maximum time of the runtime (default: inf).
    ///
    pub fn max_time(mut self, max_time: SimTime) -> Self {
        self.limit.add(RuntimeLimit::SimTime(max_time));
        self
    }

    ///
    /// Sets a custom limit to the end of the runtime, overwriting
    /// all `max_itr` and `max_time` options.
    ///
    pub fn limit(mut self, limit: RuntimeLimit) -> Self {
        self.limit.add(limit);
        self
    }

    ///
    /// Builds the simulation with the given application.
    ///
    pub fn build(self) -> Sim<A> {
        let guard = SimGuard::new();
        let future_event_set = FutureEventSet::new_with(&self);

        SimTime::set_now(self.start_time);
        set_rng(self.rng);

        Sim {
            error: Vec::new(),
            globals: self.globals,
            inner: self.app,
            param: RunParameters {
                state: State::Ready,
                limit: self.limit,
                event_id: 0,
                itr: 0,
                quiet: self.quiet,
                _guard: guard,
            },
            profiler: Profiler::default(),
            future_event_set,
        }
    }
}

impl<A> SimBuilder<A> {
    /// Retrieves the globals for the simulation.
    pub fn globals(&self) -> Arc<Globals> {
        self.globals.clone()
    }

    /// Retrieves a node.
    pub fn get(&self, path: impl AsRef<str>) -> Option<ModuleRef> {
        self.globals.get(path)
    }

    /// Sets the default processing stack for the simulation.
    ///
    /// Note that this will only affect calls of `node` after
    /// this function was called.
    pub fn set_stack<T: Into<ProcessingStack>>(&mut self, stack: impl Fn() -> T + 'static) {
        let boxed: Arc<dyn Fn() -> ProcessingStack + 'static> = Arc::new(move || stack().into());
        self.cfg.stack = boxed;
    }

    /// Sets the default processing stack for the simulation.
    ///
    /// Note that this will only affect calls of `node` after
    /// this function was called.
    pub fn with_stack<T: Into<ProcessingStack>>(mut self, stack: impl Fn() -> T + 'static) -> Self {
        self.set_stack(stack);
        self
    }

    /// Sets the default unwind behavior for the simulation.
    ///
    /// Note that this will only affect calls of `node` after
    /// this function was called.
    pub fn with_default_unwind_behavior(mut self, behavior: UnwindBehaviour) -> Self {
        self.set_default_unwind_behavior(behavior);
        self
    }

    /// Sets the default processing stack for the simulation.
    ///
    /// Note that this will only affect calls of `node` after
    /// this function was called.
    pub fn set_default_unwind_behavior(&mut self, behavior: UnwindBehaviour) {
        self.cfg.default_unwind_behavior = behavior;
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
    /// # use des::runtime::handlers::ModuleFn;
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
    /// let _ = sim.build().run();
    /// ```
    pub fn include_cfg(&mut self, raw: &str) {
        if let Ok(value) = from_str::<Value>(raw) {
            let cfg = Cfg::new(value);

            // update config of already existing modules
            self.globals.with(|mods| {
                for module in mods.nodes() {
                    cfg.capture_for(
                        &module.path.as_str().split('.').collect::<Vec<_>>(),
                        &mut module.props.write(),
                    );
                }
            });

            self.globals.add_cfg(cfg);
        }
    }

    /// See [`SimBuilder::include_cfg`]
    pub fn with_cfg(mut self, raw: &str) -> Self {
        self.include_cfg(raw);
        self
    }

    /// Tries to read and include parameters from a file into the simulation.
    ///
    /// See [`SimBuilder::include_cfg`] for more infomation.
    ///
    /// # Errors
    ///
    /// This function may fail if the reading from a file fails.
    pub fn include_cfg_file(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.include_cfg(&fs::read_to_string(path)?);
        Ok(())
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
    /// b.connect(a);
    ///
    /// let _ = sim.build().run();
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if node modules exists at `path`.
    #[track_caller]
    pub fn gate(&mut self, path: impl Into<ObjectPath>, gate: &str) -> GateRef {
        let path = path.into();
        let Some(module) = self.get(path.as_ref()) else {
            panic!("cannot create gate '{path}.{gate}', because node '{path}' does not exist")
        };
        if let Some(gate) = module.gate((gate, 0)) {
            gate
        } else {
            module.create_singular_gate(gate)
        }
    }

    /// Creates an abstrract gate on a already created module.
    ///
    /// The module will be defined `path` and the abstract gate will be in the namespace `gate`.
    /// Should the namespace already exist (defined as abstract), a reference to the
    /// existing abstract gate will be returned.
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
    /// let a = sim.gate_cluster("alice", "in");
    /// let b = sim.gate_cluster("bob", "out");
    ///
    /// b.connect(a);
    ///
    /// let _ = sim.build().run();
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if node modules exists at `path`.
    #[track_caller]
    pub fn gate_cluster(&mut self, path: impl Into<ObjectPath>, name: &str) -> GateClusterRef {
        let path = path.into();
        let Some(module) = self.get(path.as_ref()) else {
            panic!(
                "cannot create abstract gate '{path}.{name}', because node '{path}' does not exist"
            )
        };
        if let Some(cluster) = module.gate_cluster(name) {
            cluster
        } else {
            module.create_gate_cluster(name)
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
        let Some(module) = self.get(path.as_ref()) else {
            panic!("cannot create gate '{path}.{gate}', because node '{path}' does not exist")
        };

        let mut iter = 0..size;
        let gates = from_fn(|| module.gate((gate, iter.next()?))).collect::<Vec<_>>();

        match gates.len() {
            0 => (0..size).map(|pos| module.create_gate(gate, pos)).collect(),
            s if s == size => gates,
            _ => panic!("cannot create gate cluster from partial gate cluster"),
        }
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
    /// it uses a [`Spawner`] builder.
    ///
    /// See [`Spawner`] for more information.
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
    /// let _ = sim.build().run();
    /// ```
    pub fn node<M: IntoModuleTree>(
        &mut self,
        path: impl Into<ObjectPath>,
        module_block: M,
    ) -> M::Ret {
        let scoped = Spawner::new_at_buildtime(path.into(), self);
        module_block.build(scoped)
    }

    /// Returns the contained `Sim`, ending the building phase.
    pub fn freeze(self) -> Sim<A> {
        self.build()
    }
}

impl<A> Deref for SimBuilder<A> {
    type Target = Globals;

    fn deref(&self) -> &Self::Target {
        &self.globals
    }
}

impl<A> fmt::Debug for SimBuilder<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimBuilder").finish()
    }
}
