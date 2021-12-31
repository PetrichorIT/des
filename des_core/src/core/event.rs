use crate::*;
use std::{
    cmp::*,
    fmt::{Debug, Display},
    marker::PhantomData,
};

pub trait EventSuperstructure<A>: Sized {
    fn handle(self, rt: &mut Runtime<A, Self>);
}

pub trait Event<A>: Sized {
    type EventSuperstructure: EventSuperstructure<A>;

    fn handle(self, rt: &mut Runtime<A, Self::EventSuperstructure>);
}

pub(crate) struct EventNode<A, E: EventSuperstructure<A>> {
    pub(crate) time: SimTime,
    pub(crate) id: usize,
    pub(crate) event: E,

    _phantom: PhantomData<A>,
}

impl<A, E: EventSuperstructure<A>> EventNode<A, E> {
    #[inline(always)]
    pub fn handle(self, rt: &mut Runtime<A, E>) {
        self.event.handle(rt)
    }

    pub fn create_into(rt: &mut Runtime<A, E>, event: E, time: SimTime) -> Self {
        Self {
            id: rt.num_events_dispatched(),
            event,
            time,

            _phantom: PhantomData,
        }
    }
}

impl<A, E: EventSuperstructure<A>> PartialEq for EventNode<A, E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A, E: EventSuperstructure<A>> Eq for EventNode<A, E> {}

impl<A, E: EventSuperstructure<A>> PartialOrd for EventNode<A, E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A, E: EventSuperstructure<A>> Ord for EventNode<A, E> {
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

impl<A, E: EventSuperstructure<A>> Debug for EventNode<A, E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {:?} }}",
            self.id, self.time, self.event
        )
    }
}

impl<A, E: EventSuperstructure<A>> Display for EventNode<A, E>
where
    E: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {} }}",
            self.id, self.time, self.event
        )
    }
}
