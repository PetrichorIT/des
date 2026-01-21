use crate::{
    Error, Failure,
    gate::Connection,
    module::{Cfg, DummyModule, MOD_CTX, Props, UnwindBehaviour, try_current},
    prelude::{GateRef, Message, Module, ModuleRef, ObjectPath},
    processing::{ProcessingStack, TokioRuntime},
    runtime::{
        bench::Profiler, future_event_set::FutureEventSet, limit::RuntimeLimit,
        result::RuntimeResult, rng::set_rng,
    },
    statistics::Statistics,
    time::SimTime,
};
use rand::{RngCore, SeedableRng};
use serde_norway::{Value, from_str};
#[cfg(feature = "cqueue")]
use std::time::Duration;
use std::{
    fmt::Debug,
    fs, io, mem,
    ops::Deref,
    panic::{PanicHookInfo, set_hook, take_hook},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

mod api;
mod bench;
mod cfg;
mod events;
mod exec;
mod future_event_set;
mod guard;
mod limit;
mod result;
mod rng;
mod spawner;

pub mod handlers;

use guard::SimStaticsGuard;

pub(crate) use cfg::SimConfiguration;
pub(crate) use exec::*;

pub use self::api::{fail, globals, report, schedule_event};
pub use self::cfg::SimLifecycle;
pub use self::events::*;
pub use self::future_event_set::EventSink;
pub use self::rng::{random, rng, sample};
pub use self::spawner::{Spawner, SpawnerKind};

pub(crate) const FT_CQUEUE: bool = cfg!(feature = "cqueue");
pub(crate) const FT_ASYNC: bool = cfg!(feature = "async");

pub(crate) const SYM_CHECKMARK: char = '\u{2713}';
pub(crate) const SYM_CROSSMARK: char = '\u{02df}';

/// A networking simulation.
///
/// This type acts as both a builder for simulations, as well as the application object
/// used in the [`Runtime`].
///
/// A networking simulation can internally contain an application `A`,
/// that implements [`SimLifecycle`]. This type can be used attach
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
/// # use des::net::handlers::HandlerFn;
/// # use des::net::SimLifecycle;
/// # use des::net::Error;
/// struct Inner;
/// impl SimLifecycle for Inner {
///     fn at_sim_start(rt: &mut Runtime<Sim<Inner>>)  -> Result<(), Error> {
///         println!("Hello simulation");
///         /* Do something */
///         Ok(())
///     }
/// }
///
/// let mut sim = Sim::new(Inner);
/// sim.node("alice", HandlerFn::new(|msg| {
///     /* Message processing */
/// }));
///
/// let _ = Builder::new().build(sim.freeze()).run(); // prints 'Hello simulation'
/// ```
pub struct Sim<A> {
    pub(crate) error: Vec<Error>,
    globals: Arc<Globals>,
    /// A inner field of a network simulation that can be used to attach
    /// custom lifetime handlers to a simulation
    pub inner: A,
    /// Statistics collected from the simulation.
    /// This value is only set after the simulation has finished.
    pub statistics: Statistics,

    state: State,

    limit: RuntimeLimit,

    event_id: EventId,
    itr: usize,

    quiet: bool,
    /// The profiler for the simulation.
    pub profiler: Profiler<NetEvents>,
    future_event_set: FutureEventSet<NetEvents>,

    #[allow(unused)]
    guard: SimStaticsGuard,
}

type EventId = usize;

#[derive(Debug, PartialEq, Eq)]
enum State {
    Ready,
    Running,
}

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
    cqueue_num_buckets: usize,
    #[cfg(feature = "cqueue")]
    cqueue_bucket_timespan: Duration,

    app: A,
    cfg: SimConfiguration,

    globals: Arc<Globals>,
}

impl<A: SimLifecycle> Sim<A> {
    ///
    /// Returns the number of events that were dispatched on this [`Runtime`] instance.
    ///
    #[inline]
    pub fn num_events_scheduled(&self) -> usize {
        self.event_id
    }

    ///
    /// Returns the number of events that were recieved & handled on this [`Runtime`] instance.
    ///
    pub fn num_events_dispatched(&self) -> usize {
        self.itr
    }

    ///
    /// Returns the number of events that are remaining to be dispatched on this [`Runtime`] instance.
    ///
    pub fn num_events_remaining(&self) -> usize {
        self.future_event_set.len()
    }

    ///
    /// Returns the current simulation time.
    ///
    #[allow(clippy::unused_self)]
    pub fn sim_time(&self) -> SimTime {
        SimTime::now()
    }

    /// Indicates whether the simulation was already started.
    pub fn was_started(&self) -> bool {
        matches!(self.state, State::Running)
    }

    /// Indicates whether the simulation has reached its limit.
    pub fn has_reached_limit(&self) -> bool {
        self.limit.applies(self.itr + 1, self.sim_time())
    }

    /// Runs the application until it terminates or a breaking condition
    /// is reached.
    ///
    /// ### Examples
    ///
    /// ```
    /// use des::prelude::*;
    /// use std::convert::Infallible;
    ///
    /// struct MyApp();
    /// impl Application for MyApp {
    ///     type Error = Infallible;
    ///     type EventSet = MyEventSet;
    ///     fn at_sim_start(rt: &mut Runtime<Self>) -> Result<(), Infallible> {
    ///         rt.add_event(MyEventSet::EventA, SimTime::from(1.0));
    ///         rt.add_event(MyEventSet::EventB, SimTime::from(2.0));
    ///         rt.add_event(MyEventSet::EventA, SimTime::from(3.0));
    ///         Ok(())
    ///     }
    /// }
    ///
    /// #[derive(Debug)]
    /// enum MyEventSet {
    ///     EventA,
    ///     EventB
    /// }
    /// impl Event<MyApp> for MyEventSet {
    ///     fn handle(self, rt: &mut Runtime<MyApp>) -> Result<(), Infallible> {
    ///         dbg!(self, SimTime::now());
    ///         Ok(())
    ///     }
    /// }
    ///
    ///
    /// let runtime = Builder::new().build(MyApp());
    /// let result = runtime.run().assert_no_err();
    /// assert_eq!(result.time, SimTime::from(3.0));
    /// assert_eq!(result.profiler.event_count, 3);
    ///
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the application has determined that a simulation critical
    /// failure has occurred.
    ///
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn run(mut self) -> RuntimeResult<A> {
        assert_eq!(
            self.state,
            State::Ready,
            "Sim::run can only be used for simulations in the ready state"
        );
        // (0) Start sim-start
        if let Err(e) = self.start() {
            return RuntimeResult::new(self, Some(e));
        }

        // (1) Event main loop
        if let Err(e) = self.dispatch_all() {
            return RuntimeResult::new(self, Some(e));
        }

        // (2) Finish sim-end
        self.finish()
    }

    /// Starts the simulation manually. If `Sim::run` is not used, use the combination
    /// of start, tick and finish to complete a full execution cycle.
    ///
    /// `start` must be called before any calls to the main loop.
    ///
    /// # Errors
    ///
    /// Returns an error if any errors occurred during simulation startup.
    pub fn start(&mut self) -> Result<(), Failure> {
        macro_rules! symbol {
            ($i:ident) => {
                if $i { SYM_CHECKMARK } else { SYM_CROSSMARK }
            };
        }

        // (0) Publish sim-start message
        if !self.quiet {
            println!("\u{23A1}");
            println!("\u{23A2} Simulation starting");
            println!(
                "\u{23A2}  cqueue [{}] async[{}]",
                symbol!(FT_CQUEUE),
                symbol!(FT_ASYNC),
            );
            println!(
                "\u{23A2}  Executor := {}",
                self.future_event_set.descriptor()
            );
            println!("\u{23A2}  Event limit := {}", self.limit);
            println!("\u{23A3}");
        }

        // (1) Start profiler
        self.profiler.start();

        // (2) sim-starting on application object
        self.at_sim_start()?;

        self.state = State::Running;
        Ok(())
    }

    /// Executes the next n events in the runtime queue.
    ///
    /// # Errors
    ///
    /// Returns an error if any errors occurred during event execution.
    ///
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn dispatch_n_events(&mut self, n: usize) -> Result<(), Failure> {
        assert_eq!(
            self.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );

        let mut limit = RuntimeLimit::EventCount(self.num_events_dispatched() + n);
        mem::swap(&mut self.limit, &mut limit);
        self.dispatch_all()?;
        self.limit = limit;

        Ok(())
    }

    /// Executes runtime events until the runtime reaches the designated time
    ///
    /// # Errors
    ///
    /// Returns an error if any errors occurred during event
    ///
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn dispatch_events_until(&mut self, t: SimTime) -> Result<(), Failure> {
        assert_eq!(
            self.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );

        let mut limit = RuntimeLimit::SimTime(t);
        mem::swap(&mut self.limit, &mut limit);
        self.dispatch_all()?;
        self.limit = limit;

        Ok(())
    }

    /// Executes runtime events until the runtime reaches the designated time
    ///
    /// # Errors
    ///
    /// Returns an error if any errors occurred during event
    ///
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn dispatch_all(&mut self) -> Result<(), Failure> {
        assert_eq!(
            self.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );
        while !(self.dispatch_event()?) {}
        Ok(())
    }

    /// Decontructs the runtime and returns the application and the final `sim_time`.
    ///
    /// This funtions should only be used when running the simulation with manual calls
    /// to `dispatch_*`.
    ///
    /// # Errors
    ///
    /// Returns an error if the application has determined that a simulation critical
    /// failure has occurred.
    ///
    /// # Panics
    ///
    /// This function panics if the runtime is has not yet been started.
    #[allow(unused_mut)]
    pub fn finish(mut self) -> RuntimeResult<A> {
        assert_eq!(
            self.state,
            State::Running,
            "only a running simulation can be finished"
        );

        // Call the fin-handler on the allocated application
        let error = self.at_sim_end().err();

        let mut result = RuntimeResult {
            time: self.sim_time(),
            app: self,
            error,
        };
        result.app.profiler.finish(result.app.itr);

        if result.app.future_event_set.is_empty() && result.app.itr == 0 {
            if !result.app.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Empty simulation");
                println!("\u{23A2}  Ended at event #0 after 0s");
                println!("\u{23A3}");
            }

            return result;
        }

        if result.app.future_event_set.is_empty() {
            if !result.app.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation ended");
                println!(
                    "\u{23A2}  Ended at event #{} after {}",
                    result.app.itr, result.time
                );
                println!("\u{23A3}");
            }

            result
        } else {
            if !result.app.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation stopped");
                println!(
                    "\u{23A2}  Ended at event #{} with {} active events after {}",
                    result.app.itr,
                    result.app.future_event_set.len(),
                    result.time
                );
                println!("\u{23A3}");
            }

            result
                .app
                .profiler
                .remaining
                .reserve(result.app.future_event_set.len());
            while !result.app.future_event_set.is_empty() {
                let event_frame = result.app.future_event_set.fetch_next();
                result.app.profiler.remaining.push(event_frame);
            }

            result
        }
    }

    /// Processes the next event in the future event list by calling its handler.
    /// Returns `true` if the simulation should stop.
    ///
    /// This function requires the caller to guarantee that at least one
    /// event exists in the future event set.
    #[allow(clippy::should_implement_trait)]
    fn dispatch_event(&mut self) -> Result<bool, Failure> {
        if self.future_event_set.is_empty() {
            return Ok(true);
        }

        let (event, time) = self.future_event_set.fetch_next();

        if self.limit.applies(self.itr + 1, time) {
            self.future_event_set.add(time, event);
            return Ok(true);
        }

        self.itr += 1;

        // Let this be the only position where SimTime is changed
        SimTime::set_now(time);
        event.handle(self)?;

        Ok(false)
    }

    ///
    /// Adds and event to the future event heap, that will be handled in 'duration'
    /// time units.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    /// use std::convert::Infallible;
    /// # struct MyApp();
    /// # impl Application for MyApp {
    /// #     type Error = Infallible;
    /// #     type EventSet = MyEventSet;
    /// # }
    /// #
    /// # enum MyEventSet {
    /// #     EventA,
    /// #     EventB
    /// # }
    /// # impl Event<MyApp> for MyEventSet {
    /// #     fn handle(self, rt: &mut Runtime<MyApp>) -> Result<(), Infallible> { Ok(()) }
    /// # }
    /// #
    /// fn main() {
    ///     let mut runtime = Builder::seeded(1)
    ///         .start_time(10.0.into())
    ///         .build(MyApp());
    ///     runtime.add_event_in(MyEventSet::EventA, Duration::new(12, 0));
    ///
    ///     let result = runtime.run().assert_no_err();
    ///     assert_eq!(result.time, SimTime::from(22.0));
    ///     assert_eq!(result.profiler.event_count, 1);
    /// }
    /// ```
    ///
    pub fn add_event_in(&mut self, event: impl Into<NetEvents>, duration: impl Into<Duration>) {
        self.add_event(event, self.sim_time() + duration.into());
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that `sim_time`, or this
    /// function will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    /// use std::convert::Infallible;
    /// # struct MyApp();
    /// # impl Application for MyApp {
    /// #     type EventSet = MyEventSet;
    /// #     type Error = Infallible;
    /// # }
    /// #
    /// # enum MyEventSet {
    /// #     EventA,
    /// #     EventB
    /// # }
    /// # impl Event<MyApp> for MyEventSet {
    /// #     fn handle(self, rt: &mut Runtime<MyApp>) -> Result<(), Infallible> { Ok(()) }
    /// # }
    /// #
    /// fn main() {
    ///     let mut runtime = Builder::seeded(1)
    ///         .start_time(10.0.into())
    ///         .build(MyApp());
    ///     runtime.add_event(MyEventSet::EventA, SimTime::from(12.0));
    ///
    ///     let result = runtime.run().assert_no_err();
    ///     assert_eq!(result.time, SimTime::from(12.0)); // 12 not 10+12 = 22
    ///     assert_eq!(result.profiler.event_count, 1);
    ///
    /// }
    /// ```
    ///
    pub fn add_event(&mut self, event: impl Into<NetEvents>, time: SimTime) {
        self.future_event_set.add(time, event);
        self.event_id += 1;
    }
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
        let guard = SimStaticsGuard::new();
        let future_event_set = FutureEventSet::new_with(&self);

        SimTime::set_now(self.start_time);
        set_rng(self.rng);

        Sim {
            error: Vec::new(),
            globals: self.globals,
            inner: self.app,
            statistics: Statistics::default(),
            state: State::Ready,
            limit: self.limit,
            event_id: 0,
            itr: 0,
            quiet: self.quiet,
            profiler: Profiler::default(),
            future_event_set,
            guard,
        }
    }
}

impl<A> Sim<A> {
    pub(crate) fn with_roots<R>(&self, f: impl FnOnce(&ModuleRoots) -> R) -> R {
        f(&self.roots.lock().expect("failed to lock"))
    }

    /// Creates a new network simulation, with an inner application `A`.
    ///
    /// This allready binds the simulation globals to this instance.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(inner: A) -> SimBuilder<A> {
        SimBuilder::new(inner)
    }

    /// Returns an iterator over all nodes in the simulation.
    pub fn nodes(&self) -> impl Iterator<Item = ObjectPath> + '_ {
        self.with_roots(|mods| {
            mods.nodes()
                .map(|v| v.path())
                .collect::<Vec<_>>()
                .into_iter()
        })
    }

    /// Sets the output directory for this simulation.
    #[allow(clippy::missing_panics_doc)]
    pub fn set_output_dir(&self, dir: PathBuf) {
        *self.dir.lock().expect("failed") = dir;
    }

    /// Returns a handle to the simulation globals.
    pub fn globals(&self) -> Arc<Globals> {
        self.globals.clone()
    }
}

impl<A> Deref for Sim<A> {
    type Target = Globals;
    fn deref(&self) -> &Self::Target {
        &self.globals
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl<A: Debug> Debug for Sim<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sim")
            .field("inner", &self.inner)
            .field("modules", &self.roots)
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

impl<A> SimBuilder<A> {
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
    /// # use des::net::handlers::ModuleFn;
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
    /// let _ = Builder::new().build(sim.freeze()).run();
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
    /// let _ = Builder::new().build(sim.freeze()).run();
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
        let Some(module) = self.get(path.as_ref()) else {
            panic!("cannot create gate '{path}.{gate}', because node '{path}' does not exist")
        };
        let mut gates = Vec::new();
        for k in 0..size {
            if let Some(gate) = module.gate((gate, k)) {
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
    /// let _ = Builder::new().build(sim.freeze()).run();
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

impl<A> Debug for SimBuilder<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimBuilder").finish()
    }
}

/// A trait that descibes that an object can be build into a tree of modules
/// at a given scope within the simulation.
///
/// Types that implement `ModuleBlock` should be treated as builders for the actual
/// block of modules. They can contain abitrary information that may be relevent to the
/// build process of the actual modules within the block.
///
/// A module block can consist of either:
/// - no module at all
/// - on module specifically at the position defined by the scope
/// - on module at the scope position, an more as direct or indirect children of the first module.
///
/// See [`Spawner`] for more information.
pub trait IntoModuleTree {
    /// The returns type of the build method. This will be returned by `Sim::node`
    type Ret;

    /// Build the described module block within the context of scoped part of
    /// a simulation.
    fn build<A>(self, spawner: Spawner<'_, A>) -> Self::Ret;
}

impl<M: Module> IntoModuleTree for M {
    type Ret = ();
    fn build<A>(self, mut spawner: Spawner<'_, A>) -> Self::Ret {
        spawner.root(self);
    }
}

impl IntoModuleTree for () {
    type Ret = ();
    fn build<A>(self, mut spawner: Spawner<'_, A>) -> Self::Ret {
        spawner.root(DummyModule);
    }
}

impl<A: SimLifecycle> Sim<A> {
    fn at_sim_start(&mut self) -> Result<(), Failure> {
        set_hook(Box::new(panic_hook));

        let mods = self.roots.lock().expect("failed");
        // (2) Run network-node sim_starting stages
        // - inline this to ensure this is run before any possible events

        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        let max_stage = mods
            .nodes()
            .fold(1, |acc, module| acc.max(module.num_sim_start_stages()));

        drop(mods);

        // (2.1) Call the stages in order, parallel over all modules
        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            let mods = self
                .roots
                .lock()
                .expect("failed")
                .nodes()
                .collect::<Vec<_>>();
            for module in mods {
                // Use cloned handles to appease the brwchk
                if stage < module.num_sim_start_stages() {
                    let ctx = EventExecutionContext::default();

                    module.activate_with(Some(ctx.clone()));

                    #[cfg(feature = "tracing")]
                    tracing::info!("Calling at_sim_start({}).", stage);
                    self.error.extend(module.at_sim_start(stage).err());
                    module.deactivate();

                    ctx.finish(self)?;
                }
            }
        }

        A::at_sim_start(self)?;

        Ok(())
    }

    fn at_sim_end(&mut self) -> Result<(), Failure> {
        A::at_sim_end(self)?;

        let mut error = Vec::new();
        mem::swap(&mut error, &mut self.error);

        if !self.error.is_empty() {
            return Err(error.into());
        }

        let mods = self
            .roots
            .lock()
            .expect("failed")
            .nodes()
            .collect::<Vec<_>>();
        for module in mods {
            let ctx = EventExecutionContext::default();

            module.activate_with(Some(ctx.clone()));

            #[cfg(feature = "tracing")]
            tracing::info!("Calling 'at_sim_end'");
            error.extend(
                module
                    .at_sim_end()
                    .map_err(Failure::into_inner)
                    .err()
                    .unwrap_or(Vec::new()),
            );
            module.deactivate();

            ctx.finish(self)?;
        }

        let mut stats = self.globals.statistics.lock().expect("failed lock");
        mem::swap(&mut *stats, &mut self.statistics);

        let _ = take_hook();
        if error.is_empty() {
            Ok(())
        } else {
            Err(error.into())
        }
    }
}

impl<A: SimLifecycle> Sim<A> {
    ///
    /// Adds a message event into a [`Runtime<NetworkApplication<A>>`] onto a gate.
    ///
    pub fn add_message_onto(&mut self, gate: GateRef, message: impl Into<Message>, time: SimTime) {
        let event = MessageExitingConnection {
            con: Connection::new(gate),
            msg: message.into(),
        };

        self.add_event(NetEvents::MessageExitingConnection(event), time);
    }

    ///
    /// Adds a message event into a [`Runtime<NetworkApplication<A>>`] onto a module.
    ///
    pub fn handle_message_on(
        &mut self,
        module: impl Into<ModuleRef>,
        message: impl Into<Message>,
        time: SimTime,
    ) {
        let event = HandleMessageEvent {
            module: module.into(),
            message: message.into(),
        };

        self.add_event(NetEvents::HandleMessageEvent(event), time);
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

        #[cfg(feature = "async")]
        TokioRuntime::report_panic();
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
#[derive(Debug, Default)]
pub struct Globals {
    pub(crate) roots: Arc<Mutex<ModuleRoots>>,
    pub(crate) cfgs: Arc<Mutex<Vec<Cfg>>>,
    pub(crate) dir: Arc<Mutex<PathBuf>>,
    pub(crate) statistics: Mutex<Statistics>,
}

impl Globals {
    pub(crate) fn with<R>(&self, f: impl FnOnce(&ModuleRoots) -> R) -> R {
        f(&self.roots.lock().expect("failed"))
    }

    /// Returns a handle to a module from the global scope.
    /// This can be used to access arbitrary modules, independent of the current execution context.
    #[must_use]
    pub fn get(&self, path: impl AsRef<str>) -> Option<ModuleRef> {
        self.with(|mods| mods.get(path.as_ref()))
    }

    /// Returns the directory path of the
    /// out directory for this simulation.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn dir(&self) -> PathBuf {
        self.dir.lock().expect("failed").clone()
    }

    pub(crate) fn add_module(&self, module: ModuleRef) {
        self.roots.lock().expect("failed").add(module);
    }

    pub(crate) fn add_cfg(&self, cfg: Cfg) {
        self.cfgs.lock().expect("failed").push(cfg);
    }

    pub(crate) fn capture_for(&self, path_parts: &[&str], props: &mut Props) {
        let lock = self.cfgs.lock().expect("failed");
        for cfg in &*lock {
            cfg.capture_for(path_parts, props);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ModuleRoots {
    modules: Vec<ModuleRef>,
}

/// The all nodes iterator.
struct AllNodesIter<'a> {
    stack: Vec<(ModuleRef, Vec<String>)>,
    remaining: &'a [ModuleRef],
}

impl Iterator for AllNodesIter<'_> {
    type Item = ModuleRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_along_stack().or_else(|| {
            assert!(self.stack.is_empty());
            let next_root = self.remaining.first()?.clone();
            self.remaining = &self.remaining[1..];
            self.stack.push((
                next_root.clone(),
                next_root.children.read().keys().cloned().collect(),
            ));
            Some(next_root)
        })
    }
}

impl AllNodesIter<'_> {
    fn next_along_stack(&mut self) -> Option<ModuleRef> {
        let (node, keys) = self.stack.last_mut()?;
        let Some(key) = keys.pop() else {
            self.stack.pop();
            return self.next_along_stack();
        };

        let child = node.children.read()[&key].clone();
        self.stack.push((
            child.clone(),
            child.children.read().keys().cloned().collect(),
        ));
        Some(child)
    }
}

impl ModuleRoots {
    pub(crate) fn nodes(&self) -> impl Iterator<Item = ModuleRef> + '_ {
        AllNodesIter {
            stack: Vec::new(),
            remaining: &self.modules,
        }
    }

    pub(crate) fn get(&self, path: &str) -> Option<ModuleRef> {
        let (first, mut rem) = if self.modules.first()?.path.is_root() {
            ("", path)
        } else {
            path.split_once('.').unwrap_or((path, ""))
        };
        let mut current = self.modules.iter().find(|m| m.path == first)?.clone();

        while !rem.is_empty() {
            let (next, rest) = rem.split_once('.').unwrap_or((rem, ""));
            rem = rest;
            current = current.child(next).ok()?;
        }

        Some(current)
    }

    pub(crate) fn add(&mut self, module: ModuleRef) {
        assert!(
            module.parent.is_none(), // && dbg!(module.path.parent()).is_none(),
            "cannot register non-root module as root"
        );
        match self
            .modules
            .binary_search_by_key(&&module.path, |m| &m.path)
        {
            Ok(i) | Err(i) => self.modules.insert(i, module),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Weak;

    use super::*;
    use crate::module::ModuleContext;

    #[test]
    fn module_tree() {
        let mut tree = ModuleRoots::default();
        fn module(path: &str) -> ModuleRef {
            ModuleContext::new_root(path.into(), Weak::new())
        }

        tree.add(module("alice"));
        tree.add(module("bob"));
        tree.add(module("eve"));

        assert_eq!(
            tree.nodes().map(|v| v.path.to_string()).collect::<Vec<_>>(),
            ["alice", "bob", "eve",]
        );
    }
}
