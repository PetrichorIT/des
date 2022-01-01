use crate::*;
use std::{
    cmp::*,
    fmt::{Debug, Display},
    marker::PhantomData,
};

pub trait Application: Sized {
    type EventSuperstructure: EventSuperstructure<Self>;
}

pub trait EventSuperstructure<A>
where
    A: Application,
{
    fn handle(self, rt: &mut Runtime<A>);
}

pub trait Event<A: Application> {
    fn handle(self, rt: &mut Runtime<A>);
}

pub(crate) struct EventNode<A: Application> {
    pub(crate) time: SimTime,
    pub(crate) id: usize,
    pub(crate) event: A::EventSuperstructure,

    _phantom: PhantomData<A>,
}

impl<A: Application> EventNode<A> {
    #[inline(always)]
    pub fn handle(self, rt: &mut Runtime<A>) {
        self.event.handle(rt)
    }

    pub fn create_into(rt: &mut Runtime<A>, event: A::EventSuperstructure, time: SimTime) -> Self {
        Self {
            id: rt.num_events_dispatched(),
            event,
            time,

            _phantom: PhantomData,
        }
    }
}

impl<A: Application> PartialEq for EventNode<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A: Application> Eq for EventNode<A> {}

impl<A: Application> PartialOrd for EventNode<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Application> Ord for EventNode<A> {
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

impl<A: Application> Debug for EventNode<A>
where
    A::EventSuperstructure: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {:?} }}",
            self.id, self.time, self.event
        )
    }
}

impl<A: Application> Display for EventNode<A>
where
    A::EventSuperstructure: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {} }}",
            self.id, self.time, self.event
        )
    }
}
