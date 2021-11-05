use super::*;
use std::collections::BinaryHeap;
use std::fmt::*;

pub struct Runtime<A> {
    pub app: A,

    // Rt limits
    event_id: usize,
    itr: usize,
    max_itr: usize,

    // Rt Core
    sim_time: SimTime,
    future_event_heap: BinaryHeap<EventNode<A>>,
}

impl<A: Debug> Runtime<A> {
    ///
    /// Returns the number of events that were dispatched on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_dispatched(&self) -> usize {
        self.event_id
    }

    ///
    /// Returns the number of events that were recieved & handled on this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn num_events_received(&self) -> usize {
        self.itr
    }

    ///
    /// Returns the maximum number of events will be received on this [Runtime] instance before
    /// the instance shuts down.
    ///
    #[inline(always)]
    pub fn max_itr(&self) -> usize {
        self.max_itr
    }

    ///
    /// Sets the maximum number of iterations for this [Runtime] instance.
    ///
    #[inline(always)]
    pub fn set_max_itr(&mut self, value: usize) {
        self.max_itr = value;
    }

    ///
    /// Returns the current simulation time.
    ///
    pub fn sim_time(&self) -> SimTime {
        self.sim_time
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
        Self {
            app,
            sim_time: SimTime::ZERO,
            event_id: 0,
            itr: 0,
            max_itr: usize::MAX,
            future_event_heap: BinaryHeap::new(),
        }
    }

    ///
    /// Processes the next event in the future event list by calling its handler.
    /// Returns true if there is another event in queue, false if not.
    ///
    pub fn next(&mut self) -> bool {
        if self.itr > self.max_itr {
            return false;
        }

        self.itr += 1;

        let node = self.future_event_heap.pop().unwrap();
        self.sim_time = node.time();

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
        (self.app, self.sim_time)
    }

    ///
    /// Adds and event to the future event heap, that will be handled in 'duration'
    /// time units.
    ///
    pub fn add_event_in<T: 'static + Event<A>>(&mut self, event: T, duration: SimTime) {
        self.add_event(event, self.sim_time + duration)
    }

    ///
    /// Adds and event to the furtue event heap that will be handled at the given time.
    /// Note that this time must be in the future i.e. greated that sim_time, or this
    /// function will panic.
    ///
    pub fn add_event<T: 'static + Event<A>>(&mut self, event: T, time: SimTime) {
        assert!(time >= self.sim_time);

        let node = EventNode::create_into(self, event, time);
        self.event_id += 1;
        self.future_event_heap.push(node);
    }
}
