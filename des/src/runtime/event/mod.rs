use crate::time::SimTime;

mod event_set;
pub(crate) use event_set::*;

/// A trait describing a sink for events, usually the future event set of the runtime.
#[allow(unused)]
pub trait EventSink<E> {
    /// Adds an event to the sink.
    fn add(&mut self, event: E, time: SimTime);
}

impl<A: Application> EventSink<A::EventSet> for Runtime<A> {
    fn add(&mut self, event: A::EventSet, time: SimTime) {
        self.add_event(event, time);
    }
}

impl<E> EventSink<E> for Vec<(E, SimTime)> {
    fn add(&mut self, event: E, time: SimTime) {
        self.push((event, time));
    }
}

mod types;
pub use types::*;

use super::Runtime;
