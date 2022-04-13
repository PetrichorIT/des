use crate::{
    core::{event::Application, SimTime},
    util::*,
};
use log::warn;
use rand::{
    distributions::Standard,
    prelude::{Distribution, StdRng},
    Rng,
};
use std::{
    any::type_name,
    fmt::{Debug, Display},
};

mod future_event_set;
use self::future_event_set::*;

mod options;
pub use self::options::*;

mod core;
pub use self::core::*;

pub(crate) const FT_NET: bool = cfg!(feature = "net");
pub(crate) const FT_SIMTIME_U128: bool = cfg!(feature = "simtime-u128");
pub(crate) const FT_CQUEUE: bool = cfg!(feature = "cqueue");
pub(crate) const FT_INTERNAL_METRICS: bool = cfg!(feature = "internal-metrics");

///
/// The central managment point for a generic
/// instance of a discrete event based simulation.
///
/// # Generic usage
///
/// If you want to create a generic simulation you are requied to provide a 'app'
/// parameter with an associated event set yourself. To do this follow this steps:
///
/// - Create an 'App' struct that implements the trait [Application].
/// This struct will hold the systems state and define the event set used in the simulation.
/// - Create your events that handle the logic of you simulation. They must implement [Event](crate::core::Event) with the generic
/// parameter A, where A is your 'App' struct.
/// - To bind those two together create a enum that implements [EventSet](crate::core::EventSet) that holds all your events.
/// This can be done via a macro. The use this event set as the associated event set in 'App'.
///
/// # Usage with module system
///
/// If you want to use the module system for network-like simulations
/// than you must create a NetworkRuntime<A> as app parameter for the core [Runtime].
/// This network runtime comes preconfigured with an event set and all managment
/// event nessecary for the simulation. All you have to do is to pass the app into [Runtime::new]
/// to create a runnable instance and the run it.
///
pub struct Runtime<A>
where
    A: Application,
{
    /// The contained runtime application, defining globals and the used event set.
    pub app: A,

    core: Mrc<Option<RuntimeCore>>,

    future_event_set: FutureEventSet<A>,

    #[cfg(feature = "internal-metrics")]
    metrics: Mrc<crate::metrics::RuntimeMetrics>,
}

impl<A> Runtime<A>
where
    A: Application,
{
    fn core(&self) -> &RuntimeCore {
        (*self.core).as_ref().unwrap()
    }

    fn core_mut(&mut self) -> &mut RuntimeCore {
        self.core.as_mut().unwrap()
    }

    ///
    /// Returns the current number of events on enqueud.
    ///
    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn num_non_zero_events_queued(&self) -> usize {
        self.future_event_set.len_nonzero()
    }

    ///
    /// Returns the current number of events on enqueud.
    ///
    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn num_zero_events_queued(&self) -> usize {
        self.future_event_set.len_zero()
    }

    ///
    /// Returns the number of events that were dispatched on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_dispatched(&self) -> u64 {
        self.core().event_id
    }

    ///
    /// Returns the number of events that were recieved & handled on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_received(&self) -> usize {
        self.core().itr
    }

    ///
    /// Returns the maximum number of events will be received on this [Runtime] instance before
    /// the instance shuts down.
    ///
    #[inline(always)]
    pub fn max_itr(&self) -> usize {
        self.core().max_itr
    }

    ///
    /// Sets the maximum number of iterations for this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn set_max_itr(&mut self, value: usize) {
        self.core_mut().max_itr = value;
    }

    ///
    /// Returns the current simulation time.
    ///
    pub fn sim_time(&self) -> SimTime {
        self.core().sim_time
    }

    ///
    /// Returns the random number generator by mutable refernce
    ///
    #[allow(unused)]
    pub(crate) fn rng(&mut self) -> *mut StdRng {
        &mut self.core_mut().rng
    }

    ///
    /// Returns the rng.
    ///
    pub fn random<T>(&mut self) -> T
    where
        Standard: Distribution<T>,
    {
        self.core_mut().rng.gen()
    }

    ///
    /// Returns the rng.
    ///
    pub fn rng_sample<T, D>(&mut self, distribution: D) -> T
    where
        D: Distribution<T>,
    {
        self.core_mut().rng.sample(distribution)
    }
}

impl<A> Runtime<A>
where
    A: Application,
{
    ///
    /// Creates a new [Runtime] Instance using an application as core,
    /// and accepting events of type [Event<A>](crate::core::Event).
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
    /// # }
    /// # enum Events {}
    /// # impl EventSet<App> for Events {
    /// #   fn handle(self, rt: &mut Runtime<App>) {}
    /// # }
    ///
    /// let app = App(42, String::from("Hello there!"));
    /// let rt = Runtime::new(app);
    /// ```
    ///
    pub fn new(app: A) -> Self {
        Self::new_with(app, RuntimeOptions::default())
    }

    ///
    /// Creates a new [Runtime] Instance using an application as core,
    /// and accepting events of type [Event<A>](crate::core::Event), using a custom set of
    /// [RuntimeOptions].
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
    /// # }
    /// # enum Events {}
    /// # impl EventSet<App> for Events {
    /// #   fn handle(self, rt: &mut Runtime<App>) {}
    /// # }
    ///
    /// let app = App(42, String::from("Hello there!"));
    /// let rt = Runtime::new_with(app, RuntimeOptions::seeded(42).max_itr(69));
    /// ```
    ///
    pub fn new_with(app: A, options: RuntimeOptions) -> Self {
        let mut this = Self {
            future_event_set: FutureEventSet::new_with(&options),

            core: RuntimeCore::new(
                options.min_sim_time,
                0,
                0,
                options.max_itr,
                options.max_sim_time,
                options.rng,
            ),
            app,

            #[cfg(feature = "internal-metrics")]
            metrics: MrcS::new(crate::metrics::RuntimeMetrics::new()),
        };

        macro_rules! symbol {
            ($i:ident) => {
                if $i {
                    '\u{1F5F8}'
                } else {
                    '\u{26CC}'
                }
            };
        }

        // Startup message
        println!("\u{23A1}");
        println!("\u{23A2} Simulation starting");
        println!(
            "\u{23A2}  net [{}] metrics [{}] precision time [{}] cqueue [{}]",
            symbol!(FT_NET),
            symbol!(FT_INTERNAL_METRICS),
            symbol!(FT_SIMTIME_U128),
            symbol!(FT_CQUEUE)
        );
        println!(
            "\u{23A2}  Event limit := {}",
            if this.max_itr() == !0 {
                "∞".to_string()
            } else {
                format!("{}", this.max_itr())
            }
        );
        println!("\u{23A3}");

        A::at_sim_start(&mut this);
        this
    }

    ///
    /// Processes the next event in the future event list by calling its handler.
    /// Returns `true` if there is another event in queue, false if not.
    ///
    /// This function requires the caller to guarantee that at least one
    /// event exists in the future event set.
    ///
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        debug_assert!(!self.future_event_set.is_empty());

        let node = self.future_event_set.fetch_next(
            #[cfg(feature = "internal-metrics")]
            MrcS::clone(&self.metrics),
        );

        self.core_mut().itr += 1;

        if self.check_break_condition(&node) {
            let EventNode { time, event, .. } = node;
            self.future_event_set.add(
                time,
                event,
                #[cfg(feature = "internal-metrics")]
                MrcS::clone(&self.metrics),
            );
            return false;
        }

        // Let this be the only position where SimTime is changed
        self.core_mut().sim_time = node.time;

        node.handle(self);
        !self.future_event_set.is_empty()
    }

    ///
    /// Returns true if the one of the break conditions is met.
    ///
    fn check_break_condition(&self, node: &EventNode<A>) -> bool {
        self.core().itr > self.core().max_itr || node.time > self.core().max_sim_time
    }

    ///
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
    ///
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
    /// let runtime = Runtime::new(MyApp());
    /// let result = runtime.run();
    ///
    /// match result {
    ///     RuntimeResult::Finished { time, event_count, .. } => {
    ///         assert_eq!(time, SimTime::from(3.0));
    ///         assert_eq!(event_count, 3);
    ///     },
    ///     _ => panic!("They can't do that! Shoot them or something!")
    /// }
    ///
    /// ```
    ///
    pub fn run(mut self) -> RuntimeResult<A> {
        if self.future_event_set.is_empty() {
            warn!(target: "des::core", "Running simulation without any events. Think about adding some inital events.");
            return RuntimeResult::EmptySimulation { app: self.app };
        }
        while self.next() {}

        self.finish()
    }

    ///
    /// Decontructs the runtime and returns the application and the final sim_time.
    ///
    /// This funtions should only be used when running the simulation with manual calls
    /// to [next](Runtime::next).
    ///
    #[allow(unused_mut)]
    pub fn finish(mut self) -> RuntimeResult<A> {
        // Call the fin-handler on the allocated application
        A::at_sim_end(&mut self);

        if self.future_event_set.is_empty() {
            let time = self.sim_time();
            self.core().interner.fincheck();

            println!("\u{23A1}");
            println!("\u{23A2} Simulation ended");
            println!(
                "\u{23A2}  Ended at event #{} after {}",
                self.core().itr,
                time
            );

            #[cfg(feature = "internal-metrics")]
            {
                println!("\u{23A2}");
                self.metrics.finish()
            }

            println!("\u{23A3}");

            RuntimeResult::Finished {
                event_count: self.core().itr,
                app: self.app,
                time,
            }
        } else {
            let time = self.sim_time();
            self.core().interner.fincheck();

            println!("\u{23A1}");
            println!("\u{23A2} Simulation ended prematurly");
            println!(
                "\u{23A2}  Ended at event #{} with {} active events after {}",
                self.core().itr,
                self.future_event_set.len(),
                time
            );

            #[cfg(feature = "internal-metrics")]
            {
                println!("\u{23A2}");
                self.metrics.finish()
            }

            println!("\u{23A3}");

            RuntimeResult::PrematureAbort {
                event_count: self.core().itr,
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
    ///     let mut runtime = Runtime::new_with(
    ///         MyApp(),
    ///         RuntimeOptions::seeded(1).min_time(SimTime::from(10.0))
    ///     );
    ///     runtime.add_event_in(MyEventSet::EventA, SimTime::from(12.0));
    ///
    ///     match runtime.run() {
    ///         RuntimeResult::Finished { time, event_count, .. } => {
    ///             assert_eq!(time, SimTime::from(22.0));
    ///             assert_eq!(event_count, 1);
    ///         },
    ///         _ => panic!("They can't do that! Shoot them or something!")
    ///     }
    /// }
    /// ```
    ///
    pub fn add_event_in(&mut self, event: impl Into<A::EventSet>, duration: SimTime) {
        self.add_event(event, self.sim_time() + duration)
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that sim_time, or this
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
    ///     let mut runtime = Runtime::new_with(
    ///         MyApp(),
    ///         RuntimeOptions::seeded(1).min_time(SimTime::from(10.0))
    ///     );
    ///     runtime.add_event(MyEventSet::EventA, SimTime::from(12.0));
    ///
    ///     match runtime.run() {
    ///         RuntimeResult::Finished { time, event_count, .. } => {
    ///             assert_eq!(time, SimTime::from(12.0)); // 12 not 10+12 = 22
    ///             assert_eq!(event_count, 1);
    ///         },
    ///         _ => panic!("They can't do that! Shoot them or something!")
    ///     }
    /// }
    /// ```
    ///
    pub fn add_event(&mut self, event: impl Into<A::EventSet>, time: SimTime) {
        self.future_event_set.add(
            time,
            event,
            #[cfg(feature = "internal-metrics")]
            MrcS::clone(&self.metrics),
        )
    }
}

///
/// The result of an full execution of a runtime object.
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RuntimeResult<A> {
    EmptySimulation {
        app: A,
    },
    Finished {
        app: A,
        time: SimTime,
        event_count: usize,
    },
    PrematureAbort {
        app: A,
        time: SimTime,
        event_count: usize,
        active_events: usize,
    },
}

impl<A> RuntimeResult<A> {
    ///
    /// Returns the contained [`PrematureAbort`](Self::PrematureAbort) variant
    /// consuming the `self`value.
    ///
    /// # Panics
    ///
    /// This function panics if self contains another variant that [`PrematureAbort`](Self::PrematureAbort).
    ///
    pub fn unwrap_premature_abort(self) -> (A, SimTime, usize, usize) {
        match self {
            Self::PrematureAbort { app, time, event_count, active_events} => (app, time, event_count, active_events),
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
    /// # #[derive(Debug, PartialEq, Eq)]
    /// # struct MyApp;
    /// # fn main() {
    /// let result = RuntimeResult::Finished { app: MyApp, time: 1.0.into(), event_count: 1 };
    /// assert_eq!(result.unwrap(), (MyApp, SimTime::from(1.0), 1));
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
    pub fn unwrap(self) -> (A, SimTime, usize) {
        match self {
            Self::Finished {
                app,
                time,
                event_count,
            } => (app, time, event_count),
            _ => panic!("called `RuntimeResult::unwrap` on value that is not 'Finished'"),
        }
    }

    ///
    /// Returns the contained [`Finished`](Self::Finished) variant or
    /// the provided default.
    ///
    /// The argument `default` is eagerly evaulated, for lazy evaluation use
    /// [unwrap_or_else](Self::unwrap_or_else).
    ///
    pub fn unwrap_or(self, default: (A, SimTime, usize)) -> (A, SimTime, usize) {
        match self {
            Self::Finished {
                app,
                time,
                event_count,
            } => (app, time, event_count),
            _ => default,
        }
    }

    ///
    /// Returns the contained [`Finished`](Self::Finished) variant or lazily
    /// computes a fallback value from the given closure.
    ///
    pub fn unwrap_or_else<F>(self, f: F) -> (A, SimTime, usize)
    where
        F: FnOnce() -> (A, SimTime, usize),
    {
        match self {
            Self::Finished {
                app,
                time,
                event_count,
            } => (app, time, event_count),
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
                event_count,
            } => RuntimeResult::Finished {
                app: f(app),
                time,
                event_count,
            },
            Self::PrematureAbort {
                app,
                time,
                event_count,
                active_events,
            } => RuntimeResult::PrematureAbort {
                app: f(app),
                time,
                event_count,
                active_events,
            },
        }
    }
}

#[cfg(feature = "net")]
use crate::net::*;

use super::event::EventNode;

#[cfg(feature = "net")]
impl<A> Runtime<NetworkRuntime<A>> {
    ///
    /// Adds a message event into a [Runtime<NetworkRuntime<A>>] onto a gate.
    ///
    pub fn add_message_onto(
        &mut self,
        gate: GateRefMut,
        message: impl Into<Message>,
        time: SimTime,
    ) {
        let event = MessageAtGateEvent {
            gate: gate.make_readonly(),
            message: message.into(),
        };

        self.add_event(NetEvents::MessageAtGateEvent(event), time)
    }

    ///
    /// Adds a message event into a [Runtime<NetworkRuntime<A>>] onto a module.
    ///
    pub fn handle_message_on(
        &mut self,
        module: ModuleRefMut,
        message: impl Into<Message>,
        time: SimTime,
    ) {
        let event = HandleMessageEvent {
            module,
            message: message.into(),
        };

        self.add_event(NetEvents::HandleMessageEvent(event), time)
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
            if self.max_itr() == !0 {
                String::from("inf")
            } else {
                format!("{}", self.max_itr())
            },
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
            if self.max_itr() == !0 {
                String::from("inf")
            } else {
                format!("{}", self.max_itr())
            },
            self.num_events_dispatched(),
            self.future_event_set.len()
        )
    }
}
