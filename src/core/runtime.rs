use rand::{
    distributions::Standard,
    prelude::{Distribution, StdRng},
    rngs::OsRng,
    Rng, SeedableRng,
};

use crate::*;
use std::{
    any::type_name,
    collections::BinaryHeap,
    fmt::{Debug, Display},
};

static mut RT_CORE: Option<RuntimeCore> = None;

/// Return the simulation time of the current runtime.
pub fn sim_time() -> SimTime {
    unsafe { RT_CORE.as_ref().unwrap().sim_time }
}

pub fn sim_time_fmt() -> String {
    unsafe {
        SimTimeUnit::fmt_compact(
            RT_CORE.as_ref().unwrap().sim_time,
            RT_CORE.as_ref().unwrap().sim_base_unit,
        )
    }
}

/// Return the rng of the current runtime.
pub fn rng<T>() -> T
where
    Standard: Distribution<T>,
{
    unsafe { RT_CORE.as_mut().unwrap().rng.gen() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeCore {
    pub sim_time: SimTime,
    pub sim_base_unit: SimTimeUnit,

    // Rt limits
    pub event_id: usize,
    pub itr: usize,
    pub max_itr: usize,

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
    ) -> &'static mut Self {
        let rtc = Self {
            sim_time,
            sim_base_unit,
            event_id,
            itr,
            max_itr,
            rng,
        };

        unsafe {
            RT_CORE = Some(rtc);
            RT_CORE.as_mut().unwrap()
        }
    }
}

///
/// The core component of a simulation handeling
/// global utils, event scheduling and
/// application management.
///
pub struct Runtime<A> {
    pub app: A,

    core: &'static mut RuntimeCore,
    future_event_heap: BinaryHeap<EventNode<A>>,
}

impl<A> Runtime<A> {
    ///
    /// Returns the number of events that were dispatched on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_dispatched(&self) -> usize {
        self.core.event_id
    }

    ///
    /// Returns the number of events that were recieved & handled on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_received(&self) -> usize {
        self.core.itr
    }

    ///
    /// Returns the maximum number of events will be received on this [Runtime] instance before
    /// the instance shuts down.
    ///
    #[inline(always)]
    pub fn max_itr(&self) -> usize {
        self.core.max_itr
    }

    ///
    /// Sets the maximum number of iterations for this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn set_max_itr(&mut self, value: usize) {
        self.core.max_itr = value;
    }

    ///
    /// Returns the current simulation time.
    ///
    pub fn sim_time(&self) -> SimTime {
        self.core.sim_time
    }

    ///
    /// Returns the rng.
    ///
    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.core.rng
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
        if self.core.itr > self.max_itr() {
            return false;
        }

        self.core.itr += 1;

        let mut node = self.future_event_heap.pop().unwrap();
        self.core.sim_time = node.time();

        println!(
            ">> Event [{}] at {}",
            node.id(),
            SimTimeUnit::fmt_compact(self.core.sim_time, self.core.sim_base_unit)
        );

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
        (self.app, self.core.sim_time)
    }

    ///
    /// Adds and event to the future event heap, that will be handled in 'duration'
    /// time units.
    ///
    pub fn add_event_in<T: 'static + Event<A>>(&mut self, event: T, duration: SimTime) {
        self.add_event(event, self.core.sim_time + duration)
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that sim_time, or this
    /// function will panic.
    ///
    pub fn add_event<T: 'static + Event<A>>(&mut self, event: T, time: SimTime) {
        assert!(time >= self.core.sim_time);

        let node = EventNode::create_into(self, event, time);
        self.core.event_id += 1;
        self.future_event_heap.push(node);
    }
}

impl<A> Debug for Runtime<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Runtime<{}> {{ sim_time: {} (itr {} / {}) dispached: {} enqueued: {} }}",
            type_name::<A>(),
            self.core.sim_time,
            self.core.itr,
            if self.core.max_itr == !0 {
                String::from("inf")
            } else {
                format!("{}", self.core.max_itr)
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
            self.core.sim_time,
            self.core.itr,
            if self.core.max_itr == !0 {
                String::from("inf")
            } else {
                format!("{}", self.core.max_itr)
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
