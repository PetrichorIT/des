//!
//! Central primitives for running a discrete event simulation.
//!

use crate::{
    macros::support::SyncWrap,
    time::{Duration, SimTime},
};
use rand::{distributions::Standard, prelude::Distribution, Rng, RngCore};
use std::{
    any::type_name,
    cell::UnsafeCell,
    fmt::{Debug, Display},
    sync::MutexGuard,
};

mod event;
pub use self::event::*;

mod limit;
pub use self::limit::*;

mod bench;
pub use bench::*;

mod builder;
pub use builder::*;

mod metrics;

pub(crate) const FT_NET: bool = cfg!(feature = "net");
pub(crate) const FT_NDL: bool = cfg!(feature = "ndl");
pub(crate) const FT_CQUEUE: bool = cfg!(feature = "cqueue");
pub(crate) const FT_INTERNAL_METRICS: bool = cfg!(feature = "metrics");
pub(crate) const FT_ASYNC: bool = cfg!(feature = "async");

pub(crate) const SYM_CHECKMARK: char = '\u{2713}';
pub(crate) const SYM_CROSSMARK: char = '\u{02df}';

pub(crate) static RNG: SyncWrap<UnsafeCell<Option<Box<dyn RngCore>>>> =
    SyncWrap::new(UnsafeCell::new(None));

///
/// Returns a reference to a given rng.
///
/// # Panics
///
/// This function will panic if the RNG has not been initalized.
/// This will be done once the `Runtime` was created.
///
#[must_use]
pub fn rng() -> &'static mut dyn RngCore {
    unsafe { &mut *RNG.get() }
        .as_mut()
        .expect("RNG not yet initalized")
}

///
/// Generates a random instance of type T with a Standard distribution.
///
#[must_use]
pub fn random<T>() -> T
where
    Standard: Distribution<T>,
{
    rng().gen::<T>()
}

///
/// Generates a random instance of type T with a distribution
/// of type D.
///
pub fn sample<T, D>(distr: D) -> T
where
    D: Distribution<T>,
{
    rng().sample::<T, D>(distr)
}

///
/// The central managment point for a generic
/// instance of a discrete event based simulation.
///
/// # Generic usage
///
/// If you want to create a generic simulation you are requied to provide a 'app'
/// parameter with an associated event set yourself. To do this follow this steps:
///
/// - Create an 'App' struct that implements the trait [`Application`].
/// This struct will hold the systems state and define the event set used in the simulation.
/// - Create your events that handle the logic of you simulation. They must implement [`Event`](crate::runtime::Event) with the generic
/// parameter A, where A is your 'App' struct.
/// - To bind those two together create a enum that implements [`EventSet`](crate::runtime::EventSet) that holds all your events.
/// This can be done via a macro. The use this event set as the associated event set in 'App'.
///
/// # Usage with module system
///
/// If you want to use the module system for network-like simulations
/// than you must create a [`NetworkApplication<A>`] as app parameter for the core [`Runtime`].
/// This network runtime comes preconfigured with an event set and all managment
/// event nessecary for the simulation. All you have to do is to pass the app into [`Builder::build`]
/// to create a runnable instance and the run it.
///
pub struct Runtime<App>
where
    App: Application,
{
    /// The contained runtime application, defining globals and the used event set.
    pub app: App,

    // Rt limits
    limit: RuntimeLimit,

    event_id: EventId,
    itr: usize,

    // Misc
    quiet: bool,
    profiler: Profiler,

    #[allow(dead_code)]
    permit: MutexGuard<'static, ()>,

    future_event_set: FutureEventSet<App>,
}

impl<A> Runtime<A>
where
    A: Application,
{
    // ///
    // /// Returns the current number of events on enqueud.
    // ///
    // pub(crate) fn num_non_zero_events_queued(&self) -> usize {
    //     self.future_event_set.len_nonzero()
    // }

    // ///
    // /// Returns the current number of events on enqueud.
    // ///
    // pub(crate) fn num_zero_events_queued(&self) -> usize {
    //     self.future_event_set.len_zero()
    // }

    ///
    /// Returns the number of events that were dispatched on this [`Runtime`] instance.
    ///
    #[inline]
    pub fn num_events_dispatched(&self) -> usize {
        self.event_id
    }

    ///
    /// Returns the number of events that were recieved & handled on this [`Runtime`] instance.
    ///
    pub fn num_events_received(&self) -> usize {
        self.itr
    }

    ///
    /// Returns the current simulation time.
    ///
    #[allow(clippy::unused_self)]
    pub fn sim_time(&self) -> SimTime {
        SimTime::now()
    }

    // ///
    // /// Returns the random number generator by mutable refernce
    // ///
    // pub(crate) fn rng(&mut self) -> *mut StdRng {
    //     self::rng()
    // }

    ///
    /// Returns the rng.
    ///
    #[allow(clippy::unused_self)]
    pub fn random<T>(&mut self) -> T
    where
        Standard: Distribution<T>,
    {
        self::random()
    }

    ///
    /// Returns the rng.
    ///
    #[allow(clippy::unused_self)]
    pub fn rng_sample<T, D>(&mut self, distr: D) -> T
    where
        D: Distribution<T>,
    {
        self::sample(distr)
    }
}

impl<A> Runtime<A>
where
    A: Application,
{
    ///
    /// Creates a new [`Runtime`] Instance using an application as core,
    /// and accepting events of type [`Event<A>`](crate::runtime::Event).
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// // Assumme Application is implemented for App.
    /// #[derive(Debug)]
    /// struct App(usize,  String);
    /// # impl Application for App {
    /// #   type EventSet = Events;
    /// #   type Lifecycle = ();
    /// # }
    /// # enum Events {}
    /// # impl EventSet<App> for Events {
    /// #   fn handle(self, rt: &mut Runtime<App>) {}
    /// # }
    ///
    /// let app = App(42, String::from("Hello there!"));
    /// let rt = Builder::new().build(app);
    /// ```
    ///
    // #[must_use]
    // pub fn new(app: A) -> Self {
    //     Self::new_with(app, RuntimeOptions::default())
    // }

    ///
    /// Creates a new [`Runtime`] Instance using an application as core,
    /// and accepting events of type [`Event<A>`](crate::runtime::Event), using a custom set of
    /// [`RuntimeOptions`].
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// // Assumme Application is implemented for App.
    /// #[derive(Debug)]
    /// struct App(usize,  String);
    /// # impl Application for App {
    /// #   type EventSet = Events;
    /// #   type Lifecycle = ();
    /// # }
    /// # enum Events {}
    /// # impl EventSet<App> for Events {
    /// #   fn handle(self, rt: &mut Runtime<App>) {}
    /// # }
    ///
    /// let app = App(42, String::from("Hello there!"));
    /// let rt = Builder::seeded(42).max_itr(69).build(app);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if no RNG can be created from the OS-RNG.
    ///
    // #[must_use]
    // pub fn new_with(app: A, mut options: RuntimeOptions) -> Self {
    //     todo!()
    // }

    fn poison_cleanup() {
        // NOP
    }

    /// Runs the application until it terminates or a breaking condition
    /// is reached.
    ///
    /// ### Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// struct MyApp();
    /// impl Application for MyApp {
    ///     type EventSet = MyEventSet;
    ///     type Lifecycle = Self;
    /// }
    /// impl EventLifecycle for MyApp {
    ///     fn at_sim_start(rt: &mut Runtime<Self>) {
    ///         rt.add_event(MyEventSet::EventA, SimTime::from(1.0));
    ///         rt.add_event(MyEventSet::EventB, SimTime::from(2.0));
    ///         rt.add_event(MyEventSet::EventA, SimTime::from(3.0));
    ///     }
    /// }
    ///
    /// #[derive(Debug)]
    /// enum MyEventSet {
    ///     EventA,
    ///     EventB
    /// }
    /// impl EventSet<MyApp> for MyEventSet {
    ///     fn handle(self, rt: &mut Runtime<MyApp>) {
    ///         dbg!(self, SimTime::now());
    ///     }
    /// }
    ///
    ///
    /// let runtime = Builder::new().build(MyApp());
    /// let result = runtime.run();
    ///
    /// match result {
    ///     RuntimeResult::Finished { time, profiler, .. } => {
    ///         assert_eq!(time, SimTime::from(3.0));
    ///         assert_eq!(profiler.event_count, 3);
    ///     },
    ///     _ => panic!("They can't do that! Shoot them or something!")
    /// }
    ///
    /// ```
    #[must_use]
    pub fn run(mut self) -> RuntimeResult<A> {
        // (0) Start sim-start
        self.start();

        // (1) Event main loop
        if !self.future_event_set.is_empty() {
            while self.next() {}
        }

        // (2) Finish sim-end
        self.finish()
    }

    fn start(&mut self) {
        macro_rules! symbol {
            ($i:ident) => {
                if $i {
                    SYM_CHECKMARK
                } else {
                    SYM_CROSSMARK
                }
            };
        }

        // (0) Publish sim-start message
        println!("\u{23A1}");
        println!("\u{23A2} Simulation starting");
        println!(
            "\u{23A2}  net [{}] metrics [{}] cqueue [{}] ndl[{}] async[{}]",
            symbol!(FT_NET),
            symbol!(FT_INTERNAL_METRICS),
            symbol!(FT_CQUEUE),
            symbol!(FT_NDL),
            symbol!(FT_ASYNC),
        );
        println!(
            "\u{23A2}  Executor := {}",
            self.future_event_set.descriptor()
        );
        println!("\u{23A2}  Event limit := {}", self.limit);
        println!("\u{23A3}");

        // (1) Start profiler
        self.profiler.start();

        // (2) sim-starting on application object
        A::Lifecycle::at_sim_start(self);
    }

    /// Processes the next event in the future event list by calling its handler.
    /// Returns `true` if there is another event in queue, false if not.
    ///
    /// This function requires the caller to guarantee that at least one
    /// event exists in the future event set.
    #[allow(clippy::should_implement_trait)]
    fn next(&mut self) -> bool {
        debug_assert!(!self.future_event_set.is_empty());

        let (event, time) = self.future_event_set.fetch_next();

        self.itr += 1;

        if self.limit.applies(self.itr, time) {
            self.future_event_set.add(time, event);
            return false;
        }

        // Let this be the only position where SimTime is changed
        SimTime::set_now(time);

        // {
        //     #[cfg(feature = "tracing")]
        //     let span = tracing::span!(tracing::Level::TRACE, "event", id = self.itr);
        //     #[cfg(feature = "tracing")]
        //     let _g = span.enter();

        event.handle(self);
        // }

        !self.future_event_set.is_empty()
    }

    /// Decontructs the runtime and returns the application and the final `sim_time`.
    ///
    /// This funtions should only be used when running the simulation with manual calls
    /// to [`next`](Runtime::next).
    #[allow(unused_mut)]
    #[must_use]
    fn finish(mut self) -> RuntimeResult<A> {
        // Call the fin-handler on the allocated application
        A::Lifecycle::at_sim_end(&mut self);
        self.profiler.finish(self.itr);

        if self.future_event_set.is_empty() && self.itr == 0 {
            if !self.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Empty simulation");
                println!("\u{23A2}  Ended at event #0 after 0s");
                println!("\u{23A3}");
            }

            return RuntimeResult::EmptySimulation { app: self.app };
        }

        if self.future_event_set.is_empty() {
            let time = self.sim_time();

            if !self.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation ended");
                println!("\u{23A2}  Ended at event #{} after {}", self.itr, time);
                println!("\u{23A3}");
            }

            RuntimeResult::Finished {
                app: self.app,
                profiler: self.profiler,
                time,
            }
        } else {
            let time = self.sim_time();

            if !self.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation ended prematurly");
                println!(
                    "\u{23A2}  Ended at event #{} with {} active events after {}",
                    self.itr,
                    self.future_event_set.len(),
                    time
                );
                println!("\u{23A3}");
            }

            RuntimeResult::PrematureAbort {
                profiler: self.profiler,
                active_events: self.future_event_set.len(),
                app: self.app,
                time,
            }
        }
    }

    ///
    /// Adds and event to the future event heap, that will be handled in 'duration'
    /// time units.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// # struct MyApp();
    /// # impl Application for MyApp {
    /// #     type EventSet = MyEventSet;
    /// #     type Lifecycle = ();
    /// # }
    /// #
    /// # enum MyEventSet {
    /// #     EventA,
    /// #     EventB
    /// # }
    /// # impl EventSet<MyApp> for MyEventSet {
    /// #     fn handle(self, rt: &mut Runtime<MyApp>) {}
    /// # }
    /// #
    /// fn main() {
    ///     let mut runtime = Builder::seeded(1)
    ///         .start_time(10.0.into())
    ///         .build(MyApp());
    ///     runtime.add_event_in(MyEventSet::EventA, Duration::new(12, 0));
    ///
    ///     match runtime.run() {
    ///         RuntimeResult::Finished { time, profiler, .. } => {
    ///             assert_eq!(time, SimTime::from(22.0));
    ///             assert_eq!(profiler.event_count, 1);
    ///         },
    ///         _ => panic!("They can't do that! Shoot them or something!")
    ///     }
    /// }
    /// ```
    ///
    pub fn add_event_in(&mut self, event: impl Into<A::EventSet>, duration: impl Into<Duration>) {
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
    ///
    /// # struct MyApp();
    /// # impl Application for MyApp {
    /// #     type EventSet = MyEventSet;
    /// #     type Lifecycle = ();
    /// # }
    /// #
    /// # enum MyEventSet {
    /// #     EventA,
    /// #     EventB
    /// # }
    /// # impl EventSet<MyApp> for MyEventSet {
    /// #     fn handle(self, rt: &mut Runtime<MyApp>) {}
    /// # }
    /// #
    /// fn main() {
    ///     let mut runtime = Builder::seeded(1)
    ///         .start_time(10.0.into())
    ///         .build(MyApp());
    ///     runtime.add_event(MyEventSet::EventA, SimTime::from(12.0));
    ///
    ///     match runtime.run() {
    ///         RuntimeResult::Finished { time, profiler, .. } => {
    ///             assert_eq!(time, SimTime::from(12.0)); // 12 not 10+12 = 22
    ///             assert_eq!(profiler.event_count, 1);
    ///         },
    ///         _ => panic!("They can't do that! Shoot them or something!")
    ///     }
    /// }
    /// ```
    ///
    pub fn add_event(&mut self, event: impl Into<A::EventSet>, time: SimTime) {
        self.future_event_set.add(time, event);
    }
}

///
/// The result of an full execution of a runtime object.
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RuntimeResult<A> {
    /// The simulation has finished with an event count of `1`.
    /// This ususally inidcates that some parameter was invalid,
    /// or the user forgot to insert a startup event. However a
    /// at_sim_start event has been called.
    EmptySimulation {
        /// The application provided upon runtime creation, only changed through
        /// the `at_sim_start` method of modules.
        app: A,
    },
    /// The simulation has fully depleted its event pool with an event count
    /// greater than `1`. The function `at_sim_end` has been called.
    Finished {
        /// The application after the simulation was executed.
        app: A,
        /// The time of the final event in the simulation.
        time: SimTime,
        /// The runtime profile of the simulation
        profiler: Profiler,
    },
    /// The simulation has not fully deleted its event pool. but a `RuntimeLimit`
    /// has been reached.
    PrematureAbort {
        /// The application in the intermediary state of premature abort,
        /// but `at_sim_end` has been called.
        app: A,
        /// The time of the last event valid withing the limits of the runtime.
        time: SimTime,
        /// The size of the current event pool.
        active_events: usize,
        /// The runtime profile of the simulation
        profiler: Profiler,
    },
}

impl<A> RuntimeResult<A> {
    /// Gets the contained application from the runtime result.
    ///
    /// An application is contained in each variant of a runtime result.
    pub fn into_app(self) -> A {
        match self {
            Self::EmptySimulation { app }
            | Self::Finished { app, .. }
            | Self::PrematureAbort { app, .. } => app,
        }
    }

    ///
    /// Returns the contained [`PrematureAbort`](Self::PrematureAbort) variant
    /// consuming the `self`value.
    ///
    /// # Panics
    ///
    /// This function panics if self contains another variant that [`PrematureAbort`](Self::PrematureAbort).
    ///
    pub fn unwrap_premature_abort(self) -> (A, SimTime, Profiler, usize) {
        match self {
            Self::PrematureAbort { app, time,profiler, active_events} => (app, time, profiler, active_events),
            _ => panic!("called `RuntimeResult::unwrap_premature_abort` on a value that is not `PrematureAbort`")
        }
    }

    ///
    /// Returns the contained [`Finished`](Self::Finished) variant consuming the `self` value.
    ///
    /// # Panics
    ///
    /// This function panics should the `self` value contain another variant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::runtime::Profiler;
    /// # #[derive(Debug, PartialEq, Eq)]
    /// # struct MyApp;
    /// # fn main() {
    /// let p = Profiler::default();
    /// let result = RuntimeResult::Finished { app: MyApp, time: 1.0.into(), profiler: p.clone() };
    /// assert_eq!(result.unwrap(), (MyApp, SimTime::from(1.0), p));
    /// # }
    /// ```
    ///
    /// ```should_panic
    /// # use des::prelude::*;
    /// # #[derive(Debug, PartialEq, Eq)]
    /// # struct MyApp;
    /// # fn main() {
    /// let result = RuntimeResult::EmptySimulation { app: MyApp };
    /// result.unwrap();
    /// # }
    /// ```
    pub fn unwrap(self) -> (A, SimTime, Profiler) {
        match self {
            Self::Finished {
                app,
                time,
                profiler,
            } => (app, time, profiler),
            _ => panic!("called `RuntimeResult::unwrap` on value that is not 'Finished'"),
        }
    }

    ///
    /// Returns the contained [`Finished`](Self::Finished) variant or
    /// the provided default.
    ///
    /// The argument `default` is eagerly evaulated, for lazy evaluation use
    /// [`unwrap_or_else`](Self::unwrap_or_else).
    ///
    pub fn unwrap_or(self, default: (A, SimTime, Profiler)) -> (A, SimTime, Profiler) {
        match self {
            Self::Finished {
                app,
                time,
                profiler,
            } => (app, time, profiler),
            _ => default,
        }
    }

    ///
    /// Returns the contained [`Finished`](Self::Finished) variant or lazily
    /// computes a fallback value from the given closure.
    ///
    pub fn unwrap_or_else<F>(self, f: F) -> (A, SimTime, Profiler)
    where
        F: FnOnce() -> (A, SimTime, Profiler),
    {
        match self {
            Self::Finished {
                app,
                time,
                profiler,
            } => (app, time, profiler),
            _ => f(),
        }
    }

    ///
    /// Maps the `app` property that is contained in all variants to a new
    /// value of type T, using the given closure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # #[derive(Debug, PartialEq, Eq)]
    /// struct InnerResult { value: usize }
    /// # #[derive(Debug, PartialEq, Eq)]
    /// struct OuterResult { inner: InnerResult }
    ///
    /// # fn main() {
    /// let result = RuntimeResult::EmptySimulation {
    ///     app: OuterResult { inner: InnerResult { value: 42 } }
    /// };
    /// let result = result.map_app(|outer| outer.inner);
    /// assert_eq!(result, RuntimeResult::EmptySimulation { app: InnerResult { value: 42 } });
    /// # }
    /// ```
    ///
    pub fn map_app<F, T>(self, f: F) -> RuntimeResult<T>
    where
        F: FnOnce(A) -> T,
    {
        match self {
            Self::EmptySimulation { app } => RuntimeResult::EmptySimulation { app: f(app) },
            Self::Finished {
                app,
                time,
                profiler,
            } => RuntimeResult::Finished {
                app: f(app),
                time,
                profiler,
            },
            Self::PrematureAbort {
                app,
                time,
                profiler,
                active_events,
            } => RuntimeResult::PrematureAbort {
                app: f(app),
                time,
                profiler,
                active_events,
            },
        }
    }
}

cfg_net! {
    use crate::net::{gate::{GateRef, Connection},  HandleMessageEvent, message::Message, MessageExitingConnection, module::ModuleRef, NetEvents, NetworkApplication};

    impl<A> Runtime<NetworkApplication<A>> where
        A: EventLifecycle<NetworkApplication<A>>,{
        ///
        /// Adds a message event into a [`Runtime<NetworkApplication<A>>`] onto a gate.
        ///
        pub fn add_message_onto(
            &mut self,
            gate: GateRef,
            message: impl Into<Message>,
            time: SimTime,
        ) {
            let event = MessageExitingConnection {
                con: Connection::new(gate),
                msg: message.into(),
            };

            self.add_event(NetEvents::MessageAtGateEvent(event), time);
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
}

impl<A> Debug for Runtime<A>
where
    A: Application,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Runtime<{}> {{ sim_time: {} (itr {} / {}) dispached: {} enqueued: {} }}",
            type_name::<A>(),
            self.sim_time(),
            self.num_events_received(),
            self.limit,
            self.num_events_dispatched(),
            self.future_event_set.len()
        )
    }
}

impl<A> Display for Runtime<A>
where
    A: Application,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Runtime<{}> {{ sim_time: {} (itr {} / {}) dispached: {} enqueued: {} }}",
            type_name::<A>(),
            self.sim_time(),
            self.num_events_received(),
            self.limit,
            self.num_events_dispatched(),
            self.future_event_set.len()
        )
    }
}
