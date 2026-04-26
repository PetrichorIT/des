use crate::{Sim, runtime::NetEvents, time::SimTime};

mod event_set;
pub(crate) use event_set::*;

mod types;
pub use types::*;

/// A trait describing a sink for events, usually the future event set of the runtime.
#[allow(unused)]
pub trait EventSink<E> {
    /// Adds an event to the sink.
    fn add(&mut self, event: E, time: SimTime);
}

impl<A: SimLifecycle> EventSink<NetEvents> for Sim<A> {
    fn add(&mut self, event: NetEvents, time: SimTime) {
        self.add_event(event, time);
    }
}

impl<E> EventSink<E> for Vec<(E, SimTime)> {
    fn add(&mut self, event: E, time: SimTime) {
        self.push((event, time));
    }
}
