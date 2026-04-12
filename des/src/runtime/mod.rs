use crate::{
    Error, ErrorKind, Failure,
    gate::Connection,
    module::{DummyModule, MOD_CTX, ModuleContext, try_current},
    prelude::{GateRef, Message, Module, ModuleRef, ObjectPath},
    runtime::{
        bench::Profiler, event::FutureEventSet, limit::RuntimeLimit, result::RuntimeResult,
        rng::set_rng,
    },
    time::SimTime,
};
#[cfg(feature = "cqueue")]
use std::time::Duration;
use std::{
    any::Any,
    fmt::Debug,
    mem,
    ops::Deref,
    panic::{PanicHookInfo, set_hook, take_hook},
    path::PathBuf,
    sync::Arc,
};

mod api;
mod bench;
mod builder;
mod event;
mod events;
mod exec;
mod limit;
mod result;
mod rng;

pub mod handlers;

pub(crate) use exec::*;

pub use self::api::{fail, globals, report, schedule_event};
pub use self::builder::*;
pub use self::event::{EventSink, SimLifecycle};
pub use self::events::*;
pub use self::rng::{random, rng, sample};

pub(crate) const FT_CQUEUE: bool = cfg!(feature = "cqueue");
pub(crate) const FT_ASYNC: bool = cfg!(feature = "async");

pub(crate) const SYM_CHECKMARK: char = '\u{2713}';
pub(crate) const SYM_CROSSMARK: char = '\u{02df}';

/// A networking simulation.
///
/// This type acts as both a builder for simulations, as well as the application object
/// used in the [`Sim`].
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
/// # use des::runtime::handlers::HandlerFn;
/// # use des::runtime::SimLifecycle;
/// # use des::Failure;
/// struct Inner;
/// impl SimLifecycle for Inner {
///     fn at_sim_start(rt: &mut Sim<Inner>)  -> Result<(), Failure> {
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
/// let _ = sim.build().run(); // prints 'Hello simulation'
/// ```
pub struct Sim<A> {
    /// A inner field of a network simulation that can be used to attach
    /// custom lifetime handlers to a simulation
    pub inner: A,

    /// The profiler for the simulation.
    pub profiler: Profiler<NetEvents>,

    pub(crate) error: Vec<Error>,

    globals: Arc<Globals>,
    future_event_set: FutureEventSet<NetEvents>,
    param: RunParameters,
}

struct RunParameters {
    state: State,
    limit: RuntimeLimit,
    event_id: usize,
    itr: usize,
    quiet: bool,
    _guard: SimGuard,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Ready,
    Running,
}

impl<A: SimLifecycle> Sim<A> {
    ///
    /// Returns the number of events that were dispatched on this [`Sim`] instance.
    ///
    #[inline]
    pub fn num_events_scheduled(&self) -> usize {
        self.param.event_id
    }

    ///
    /// Returns the number of events that were recieved & handled on this [`Sim`] instance.
    ///
    pub fn num_events_dispatched(&self) -> usize {
        self.param.itr
    }

    ///
    /// Returns the number of events that are remaining to be dispatched on this [`Sim`] instance.
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
        matches!(self.param.state, State::Running)
    }

    /// Indicates whether the simulation has reached its limit.
    pub fn has_reached_limit(&self) -> bool {
        self.param
            .limit
            .applies(self.param.itr + 1, self.sim_time())
    }

    /// Runs the application until it terminates or a breaking condition
    /// is reached.
    ///
    /// ### Examples
    ///
    /// ```
    /// use des::prelude::*;
    /// let sim = Sim::new(());
    /// /* ... */
    ///
    /// let result = sim.build().run().assert_no_err();
    /// # return;
    /// assert_eq!(result.time, SimTime::from(3.0));
    /// assert_eq!(result.app.profiler.event_count, 3);
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
            self.param.state,
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
        if !self.param.quiet {
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
            println!("\u{23A2}  Event limit := {}", self.param.limit);
            println!("\u{23A3}");
        }

        // (1) Start profiler
        self.profiler.start();

        // (2) sim-starting on application object
        self.at_sim_start()?;

        self.param.state = State::Running;
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
            self.param.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );

        let mut limit = RuntimeLimit::EventCount(self.num_events_dispatched() + n);
        mem::swap(&mut self.param.limit, &mut limit);
        self.dispatch_all()?;
        self.param.limit = limit;

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
            self.param.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );

        let mut limit = RuntimeLimit::SimTime(t);
        mem::swap(&mut self.param.limit, &mut limit);
        self.dispatch_all()?;
        self.param.limit = limit;

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
            self.param.state,
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
            self.param.state,
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
        result.app.profiler.finish(result.app.param.itr);

        if result.app.future_event_set.is_empty() && result.app.param.itr == 0 {
            if !result.app.param.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Empty simulation");
                println!("\u{23A2}  Ended at event #0 after 0s");
                println!("\u{23A3}");
            }

            return result;
        }

        if result.app.future_event_set.is_empty() {
            if !result.app.param.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation ended");
                println!(
                    "\u{23A2}  Ended at event #{} after {}",
                    result.app.param.itr, result.time
                );
                println!("\u{23A3}");
            }

            result
        } else {
            if !result.app.param.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation stopped");
                println!(
                    "\u{23A2}  Ended at event #{} with {} active events after {}",
                    result.app.param.itr,
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

        if self.param.limit.applies(self.param.itr + 1, time) {
            self.future_event_set.add(time, event);
            return Ok(true);
        }

        self.param.itr += 1;

        // Let this be the only position where SimTime is changed
        SimTime::set_now(time);
        event.handle(self)?;

        Ok(false)
    }

    ///
    /// Adds and event to the future event heap, that will be handled in 'duration'
    /// time units.
    ///
    pub fn add_event_in(&mut self, event: impl Into<NetEvents>, duration: impl Into<Duration>) {
        self.add_event(event, self.sim_time() + duration.into());
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that `sim_time`, or this
    /// function will panic.
    ///
    pub fn add_event(&mut self, event: impl Into<NetEvents>, time: SimTime) {
        self.future_event_set.add(time, event);
        self.param.event_id += 1;
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
        *self.dir.lock().expect("failed") = Some(dir);
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
                    module.at_sim_start(stage);
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
    /// Adds a message event into a [`Sim<A>`] onto a gate.
    ///
    pub fn add_message_onto(&mut self, gate: GateRef, message: impl Into<Message>, time: SimTime) {
        let event = MessageExitingConnection {
            con: Connection::new(gate),
            msg: message.into(),
        };

        self.add_event(NetEvents::MessageExitingConnection(event), time);
    }

    ///
    /// Adds a message event into a [`Sim<A>`] onto a module.
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

        report_panic(&current, info);
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

fn report_panic(handle: &ModuleContext, info: &PanicHookInfo) {
    let behaviour = handle.unwind_behaviour();
    if behaviour.ignore_panics {
        return;
    }

    let payload = into_payload_box(info);
    let error = Error::new_current(ErrorKind::ModulePanic(payload));

    if behaviour.on_panic_abort {
        handle.exec().report_failure(error);
    } else {
        handle.exec().report_error(error);
    }
}

fn into_payload_box(info: &PanicHookInfo) -> Box<dyn Any + Send + 'static> {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        return Box::new((*s).to_string());
    }
    if let Some(s) = info.payload().downcast_ref::<String>() {
        return Box::new(s.clone());
    }
    Box::new("dyn Any")
}
