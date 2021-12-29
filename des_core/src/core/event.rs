use crate::*;
use std::{
    cmp::*,
    fmt::{Debug, Display},
};

///
/// A trait that provides the possibility of the type to be called by a runtime
/// of generic type A
///
pub trait Event<A> {
    fn handle(self: Box<Self>, rt: &mut Runtime<A>);
}

pub(crate) struct EventNode<A: Sized> {
    time: SimTime,
    id: usize,
    event: Box<dyn Event<A>>,
}

impl<A: Sized> EventNode<A> {
    ///
    /// Returns the id of the given event.
    ///
    #[inline(always)]
    #[allow(unused)]
    pub fn id(&self) -> usize {
        self.id
    }

    ///
    /// Returns the time to handle the event.
    #[inline(always)]
    pub fn time(&self) -> SimTime {
        self.time
    }

    ///
    /// Calls the embedded event handler.
    ///
    #[inline(always)]
    pub fn handle(self, rt: &mut Runtime<A>) {
        self.event.handle(rt)
    }

    pub fn create_into<T: 'static + Event<A>>(
        rt: &mut Runtime<A>,
        event: T,
        time: SimTime,
    ) -> Self {
        Self {
            id: rt.num_events_dispatched(),
            event: Box::new(event),
            time,
        }
    }
}

impl<A> PartialEq for EventNode<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A> Eq for EventNode<A> {}

impl<A> PartialOrd for EventNode<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A> Ord for EventNode<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else if self.time < other.time {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

impl<A> Debug for EventNode<A>
where
    dyn Event<A>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {:?} }}",
            self.id, self.time, self.event
        )
    }
}

impl<A> Display for EventNode<A>
where
    dyn Event<A>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {} }}",
            self.id, self.time, self.event
        )
    }
}
