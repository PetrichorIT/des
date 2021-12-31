use crate::*;

use lazy_static::lazy_static;
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

use super::logger::StandardLogger;

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
/// The core component of a simulation handeling
/// global utils, event scheduling and
/// application management.
///
pub struct Runtime<A> {
    pub app: A,

    core: &'static SyncCell<Option<RuntimeCore>>,
    future_event_heap: BinaryHeap<EventNode<A>>,
}

impl<A> Runtime<A> {
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
        Self {
            core: RuntimeCore::new(
                SimTime::ZERO,
                options.sim_base_unit,
                0,
                0,
                options.max_itr,
                options.rng,
            ),
            app,
            future_event_heap: BinaryHeap::new(),
        }
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
        self.core_mut().sim_time = node.time();

        node.handle(self);
        !self.future_event_heap.is_empty()
    }

    ///
    /// Runs the application until it terminates or exceeds it max_itr.
    ///
    pub fn run(mut self) -> Option<(A, SimTime)> {
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
    pub fn add_event_in<T: 'static + Event<A>>(&mut self, event: T, duration: SimTime) {
        self.add_event(event, self.sim_time() + duration)
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that sim_time, or this
    /// function will panic.
    ///
    pub fn add_event<T: 'static + Event<A>>(&mut self, event: T, time: SimTime) {
        assert!(time >= self.sim_time());

        let node = EventNode::create_into(self, event, time);
        self.core_mut().event_id += 1;
        self.future_event_heap.push(node);
    }
}

impl<A> Debug for Runtime<A> {
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

impl<A> Display for Runtime<A> {
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
/// Options for configuring a runtime independent of datacollection.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub sim_base_unit: SimTimeUnit,
    pub rng: StdRng,
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
