//!
//! Central primitives for running a discrete event simulation.
//!

use crate::{
    macros::support::SyncWrap,
    time::{Duration, SimTime},
};
use rand::{distr::StandardUniform, prelude::Distribution, Rng, RngCore};
use std::{
    any::type_name,
    cell::UnsafeCell,
    fmt::{Debug, Display},
    mem,
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

mod error;
pub use error::*;

mod metrics;

pub(crate) const FT_NET: bool = cfg!(feature = "net");
pub(crate) const FT_CQUEUE: bool = cfg!(feature = "cqueue");
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
    StandardUniform: Distribution<T>,
{
    rng().random::<T>()
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
///   This struct will hold the systems state and define the event set used in the simulation.
/// - Create your events that handle the logic of you simulation. They must implement [`Event`] with the generic
///   parameter A, where A is your 'App' struct.
/// - To bind those two together create a enum that implements [`EventSet`] that holds all your events.
///
/// This can be done via a macro. The use this event set as the associated event set in 'App'.
///
/// # Usage with module system
///
/// If you want to use the module system for network-like simulations
/// than you must create a [`Sim<A>`] as app parameter for the core [`Runtime`].
/// This network runtime comes preconfigured with an event set and all managment
/// event nessecary for the simulation. All you have to do is to pass the app into [`Builder::build`]
/// to create a runnable instance and the run it.
///
/// [`Event`]: crate::runtime::Event
/// [`EventSet`]: crate::runtime::EventSet
pub struct Runtime<App>
where
    App: Application,
{
    /// The contained runtime application, defining globals and the used event set.
    pub app: App,

    state: State,

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

#[derive(Debug, PartialEq, Eq)]
enum State {
    Ready,
    Running,
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
    /// Returns the current simulation time.
    ///
    #[allow(clippy::unused_self)]
    pub fn sim_time(&self) -> SimTime {
        SimTime::now()
    }

    ///
    /// Returns the rng.
    ///
    #[allow(clippy::unused_self)]
    pub fn random<T>(&mut self) -> T
    where
        StandardUniform: Distribution<T>,
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
    ///     Ok((_, time, profiler))  => {
    ///         assert_eq!(time, SimTime::from(3.0));
    ///         assert_eq!(profiler.event_count, 3);
    ///     },
    ///     _ => panic!("They can't do that! Shoot them or something!")
    /// }
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
    pub fn run(mut self) -> Result<(A, SimTime, Profiler), RuntimeError> {
        assert_eq!(
            self.state,
            State::Ready,
            "Sim::run can only be used for simulations in the ready state"
        );
        // (0) Start sim-start
        self.start();

        // (1) Event main loop
        self.dispatch_all();

        // (2) Finish sim-end
        self.finish()
    }

    /// Starts the simulation manually. If `Sim::run` is not used, use the combination
    /// of start, tick and finish to complete a full execution cycle.
    ///
    /// `start` must be called before any calls to the main loop.
    pub fn start(&mut self) {
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
        if !self.quiet {
            println!("\u{23A1}");
            println!("\u{23A2} Simulation starting");
            println!(
                "\u{23A2}  net [{}] cqueue [{}] async[{}]",
                symbol!(FT_NET),
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
        A::Lifecycle::at_sim_start(self);

        self.state = State::Running;
    }

    /// Executes the next n events in the runtime queue.
    ///
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn dispatch_n_events(&mut self, n: usize) -> bool {
        assert_eq!(
            self.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );

        let mut limit = RuntimeLimit::EventCount(self.num_events_dispatched() + n);
        mem::swap(&mut self.limit, &mut limit);
        self.dispatch_all();
        self.limit = limit;

        false
    }

    /// Executes runtime events until the runtime reaches the designated time
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn dispatch_events_until(&mut self, t: SimTime) -> bool {
        assert_eq!(
            self.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );

        let mut limit = RuntimeLimit::SimTime(t);
        mem::swap(&mut self.limit, &mut limit);
        self.dispatch_all();
        self.limit = limit;

        false
    }

    /// Executes runtime events until the runtime reaches the designated time
    ///
    /// # Panics
    ///
    /// This function panics if the simulation has not been started.
    pub fn dispatch_all(&mut self) {
        assert_eq!(
            self.state,
            State::Running,
            "dispatching is only allowed for running simulations"
        );
        while !self.dispatch_event() {}
    }

    /// Decontructs the runtime and returns the application and the final `sim_time`.
    ///
    /// This funtions should only be used when running the simulation with manual calls
    /// to [`next`](Runtime::next).
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
    pub fn finish(mut self) -> Result<(A, SimTime, Profiler), RuntimeError> {
        assert_eq!(
            self.state,
            State::Running,
            "only a running simulation can be finished"
        );

        // Call the fin-handler on the allocated application
        A::Lifecycle::at_sim_end(&mut self)?;
        self.profiler.finish(self.itr);

        if self.future_event_set.is_empty() && self.itr == 0 {
            if !self.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Empty simulation");
                println!("\u{23A2}  Ended at event #0 after 0s");
                println!("\u{23A3}");
            }

            let time = self.sim_time();
            return Ok((self.app, time, self.profiler));
        }

        if self.future_event_set.is_empty() {
            let time = self.sim_time();

            if !self.quiet {
                println!("\u{23A1}");
                println!("\u{23A2} Simulation ended");
                println!("\u{23A2}  Ended at event #{} after {}", self.itr, time);
                println!("\u{23A3}");
            }

            Ok((self.app, time, self.profiler))
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

            Ok((self.app, time, self.profiler))
        }
    }

    /// Processes the next event in the future event list by calling its handler.
    /// Returns `true` if the simulation should stop.
    ///
    /// This function requires the caller to guarantee that at least one
    /// event exists in the future event set.
    #[allow(clippy::should_implement_trait)]
    fn dispatch_event(&mut self) -> bool {
        if self.future_event_set.is_empty() {
            return true;
        }

        let (event, time) = self.future_event_set.fetch_next();

        if self.limit.applies(self.itr + 1, time) {
            self.future_event_set.add(time, event);
            return true;
        }

        self.itr += 1;

        // Let this be the only position where SimTime is changed
        SimTime::set_now(time);

        event.handle(self);

        false
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
    ///         Ok((_, time, profiler)) => {
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
    ///         Ok((_, time, profiler)) => {
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
        self.event_id += 1;
    }
}

cfg_net! {
    use crate::net::{gate::{GateRef, Connection},  HandleMessageEvent, message::Message, MessageExitingConnection, module::ModuleRef, NetEvents, Sim};

    impl<A> Runtime<Sim<A>> where
        A: EventLifecycle<Sim<A>>,{
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
            self.num_events_dispatched(),
            self.limit,
            self.num_events_scheduled(),
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
            self.num_events_dispatched(),
            self.limit,
            self.num_events_scheduled(),
            self.future_event_set.len()
        )
    }
}
