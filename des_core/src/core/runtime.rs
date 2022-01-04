use crate::core::interning::*;
use crate::core::*;

use lazy_static::lazy_static;
use log::warn;
use rand::{
    distributions::Standard,
    prelude::{Distribution, StdRng},
    rngs::OsRng,
    Rng, SeedableRng,
};
use std::{
    any::type_name,
    collections::BinaryHeap,
    fmt::{Debug, Display},
};
use util::SyncCell;

lazy_static! {
    pub(crate) static ref RTC: SyncCell<Option<RuntimeCore>> = SyncCell::new(None);
}

///
/// Returns the current simulation time of the currentlly active
/// runtime session.
///
#[inline(always)]
pub fn sim_time() -> SimTime {
    unsafe { (*RTC.get()).as_ref().unwrap().sim_time }
}

///
/// Returns the current simulation time formatted with the
/// simulations base unit.
///
#[inline(always)]
pub fn sim_time_fmt() -> String {
    unsafe {
        SimTimeUnit::fmt_compact(
            (*RTC.get()).as_ref().unwrap().sim_time,
            (*RTC.get()).as_ref().unwrap().sim_base_unit,
        )
    }
}

///
/// Generates a random instance of type T with a Standard distribution.
///
pub fn rng<T>() -> T
where
    Standard: Distribution<T>,
{
    unsafe { (*RTC.get()).as_mut().unwrap().rng.gen() }
}

///
/// Generates a random instance of type T with a distribution
/// of type D.
///
pub fn sample<T, D>(distr: D) -> T
where
    D: Distribution<T>,
{
    unsafe { (*RTC.get()).as_mut().unwrap().rng.sample(distr) }
}

#[derive(Debug)]
pub(crate) struct RuntimeCore {
    pub sim_time: SimTime,
    pub sim_base_unit: SimTimeUnit,

    // Rt limits
    pub event_id: usize,
    pub itr: usize,
    pub max_itr: usize,

    // interning
    pub interner: Interner,

    // Misc
    pub rng: StdRng,
}

impl RuntimeCore {
    pub fn new(
        sim_time: SimTime,
        sim_base_unit: SimTimeUnit,
        event_id: usize,
        itr: usize,
        max_itr: usize,
        rng: StdRng,
    ) -> &'static SyncCell<Option<RuntimeCore>> {
        let rtc = Self {
            sim_time,
            sim_base_unit,

            event_id,
            itr,
            max_itr,

            interner: Interner::new(),

            rng,
        };

        if let Err(e) = StandardLogger::setup() {
            eprintln!("{}", e)
        }

        unsafe { *RTC.get() = Some(rtc) };

        &RTC
    }
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
/// - Create an 'App' struct that implements the trait [Application].
/// This struct will hold the systems state and define the event set used in the simulation.
/// - Create your events that handle the logic of you simulation. They must implement [Event] with the generic
/// parameter A, where A is your 'App' struct.
/// - To bind those two together create a enum that implements [EventSet] that holds all your events.
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
pub struct Runtime<A: Application> {
    /// The contained runtime application, defining globals and the used event set.
    pub app: A,

    core: &'static SyncCell<Option<RuntimeCore>>,
    future_event_heap: BinaryHeap<EventNode<A>>,
}

impl<A: Application> Runtime<A> {
    fn core(&self) -> &RuntimeCore {
        unsafe { (*self.core.get()).as_ref().unwrap() }
    }

    fn core_mut(&mut self) -> &mut RuntimeCore {
        unsafe { (*self.core.get()).as_mut().unwrap() }
    }

    ///
    /// Returns the number of events that were dispatched on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_dispatched(&self) -> usize {
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
    /// Returns the rng.
    ///
    pub fn rng<T>(&mut self) -> T
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

    ///
    /// Creates a new [Runtime] Instance using an application as core,
    /// and accepting events of type [Event<A>].
    ///
    /// # Examples
    ///
    /// ```
    /// use dse::*;
    ///
    /// #[derive(Debug)]
    /// struct App(usize,  String);
    ///
    /// let app = App(42, String::from("Hello there!"));
    /// let rt = Runtime::new(app);
    /// ```
    ///
    pub fn new(app: A) -> Self {
        Self::new_with(app, RuntimeOptions::default())
    }

    pub fn new_with(app: A, options: RuntimeOptions) -> Self {
        let mut this = Self {
            core: RuntimeCore::new(
                SimTime::ZERO,
                options.sim_base_unit,
                0,
                0,
                options.max_itr,
                options.rng,
            ),
            app,
            future_event_heap: BinaryHeap::with_capacity(64),
        };

        A::at_simulation_start(&mut this);
        this
    }

    ///
    /// Processes the next event in the future event list by calling its handler.
    /// Returns true if there is another event in queue, false if not.
    ///
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        if self.num_events_received() > self.max_itr() {
            return false;
        }

        self.core_mut().itr += 1;

        let node = self.future_event_heap.pop().unwrap();
        self.core_mut().sim_time = node.time;

        node.handle(self);
        !self.future_event_heap.is_empty()
    }

    ///
    /// Runs the application until it terminates or exceeds it max_itr.
    ///
    pub fn run(mut self) -> Option<(A, SimTime)> {
        if self.future_event_heap.is_empty() {
            warn!(target: "des::core", "Running simulation without any events. Think about adding some inital events.");
            return None;
        }

        while self.next() {}

        if self.future_event_heap.is_empty() {
            Some(self.finish())
        } else {
            None
        }
    }

    ///
    /// Decontructs the runtime and returns the application and the final sim_time.
    ///
    pub fn finish(self) -> (A, SimTime) {
        let t1 = self.sim_time();
        self.core().interner.fincheck();

        (self.app, t1)
    }

    ///
    /// Adds and event to the future event heap, that will be handled in 'duration'
    /// time units.
    ///
    pub fn add_event_in(&mut self, event: impl Into<A::EventSet>, duration: SimTime) {
        self.add_event(event, self.sim_time() + duration)
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that sim_time, or this
    /// function will panic.
    ///
    pub fn add_event(&mut self, event: impl Into<A::EventSet>, time: SimTime) {
        assert!(time >= self.sim_time());

        let node = EventNode::create_into(self, event.into(), time);
        self.core_mut().event_id += 1;
        self.future_event_heap.push(node);
    }
}

#[cfg(feature = "net")]
use crate::net::*;

#[cfg(feature = "net")]
impl<A> Runtime<NetworkRuntime<A>> {
    pub fn add_message_onto(&mut self, gate_id: GateId, message: Message, time: SimTime) {
        let event = MessageAtGateEvent {
            gate_id,
            handled: false,
            message: std::mem::ManuallyDrop::new(message),
        };

        self.add_event(NetEvents::MessageAtGateEvent(event), time)
    }

    pub fn handle_message_on(&mut self, module_id: ModuleId, message: Message, time: SimTime) {
        let event = HandleMessageEvent {
            module_id,
            handled: false,
            message: std::mem::ManuallyDrop::new(message),
        };

        self.add_event(NetEvents::HandleMessageEvent(event), time)
    }
}

impl<A: Application> Debug for Runtime<A> {
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
            self.future_event_heap.len()
        )
    }
}

impl<A: Application> Display for Runtime<A> {
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
            self.future_event_heap.len()
        )
    }
}

// OPTS

///
/// Options for sepcificing the behaviour of the core runtime
/// independent of the app logic.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    /// The base unit for representing time stamps.
    /// Defaults to [SimTimeUnit::Undefined].
    pub sim_base_unit: SimTimeUnit,
    /// The random number generator used internally.
    /// This can be seeded to ensure reproducability.
    /// Defaults to a [OsRng] which does NOT provide reproducability.
    pub rng: StdRng,
    /// The maximum number of events processed by the simulation. Defaults to [usize::MAX].
    pub max_itr: usize,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            sim_base_unit: SimTimeUnit::Undefined,
            rng: StdRng::from_rng(OsRng::default()).unwrap(),
            max_itr: !0,
        }
    }
}
