use crate::core::*;
use std::{
    cmp::*,
    fmt::{Debug, Display},
    marker::PhantomData,
};

///
/// A trait that defines an runtime application
/// that depends on a event set to be processed by the
/// runtime.
///
pub trait Application: Sized {
    /// The set of events used in the simulation.
    type EventSet: EventSet<Self>;

    /// A function that is called only once at the start of the simulation.
    fn at_sim_start(_rt: &mut Runtime<Self>) {}
}

///
/// A type that can be used as a wrapper around all events
/// handled by an application A.
///
/// # Note
///
/// Note that ther is a cyclic dependecy between the event set
/// and the application.
/// This is due to the fact that Events allways defined those two parameters
/// to be related (since specific events of the event set require runtime params),
/// but this type information is willingly elided, to fit into the rust generics system.
///
pub trait EventSet<App>
where
    App: Application<EventSet = Self>,
{
    ///
    /// A function to handle an upcoming event represented as a instance
    /// of the event set.
    ///
    /// # Note
    ///
    /// Since events sets are usually macro-generated this is just a match statement that calls
    /// the handle function on the given variant, as defined by the trait [Event].
    ///
    fn handle(self, rt: &mut Runtime<App>);
}

///
/// A type that can handle an event, specific to the given aplication,
/// and associated event set.
///
pub trait Event<App>
where
    App: Application,
{
    ///
    /// A function to handle an upcoming event represented as a specific
    /// instance of a event type.
    ///
    /// # Note
    ///
    /// There is an implicit type bound that the Apps event set must contain
    /// the Self type as a variant. This is usually guaranteed by macro-generting event sets,
    /// but could lead to unexpected behaviour if not done properly in custom
    /// event set implementations.
    ///
    fn handle(self, rt: &mut Runtime<App>);
}

///
/// A runtime unqiue identifier for a event.
///
pub type EventId = u64;

///
/// A bin-heap node of a event from the applicaitons event set.
///
/// # Allocation
///
/// This node does not contain nested heap allocations by default,
/// only if the generic event itself requires heap allocations.
/// Nonetheless this node will be stored on the heap as it is
/// only used inside a [std::collections::BinaryHeap].
///
pub(crate) struct EventNode<A: Application> {
    /// The deadline timestamp for the event.
    pub(crate) time: SimTime,
    /// A runtime-specific unique identifier.
    pub(crate) id: EventId,
    /// The actual event.
    pub(crate) event: A::EventSet,

    /// A marker to preserve the type information concerning the application
    /// not only the Event set.
    _phantom: PhantomData<A>,
}

impl<A: Application> EventNode<A> {
    ///
    /// Delegation call to 'handle' on the event from the [EventSet].
    ///
    #[inline(always)]
    pub fn handle(self, rt: &mut Runtime<A>) {
        self.event.handle(rt)
    }

    ///
    /// Creates a event for the given runtime.
    ///
    pub fn create_into(rt: &Runtime<A>, event: A::EventSet, time: SimTime) -> Self {
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

    fn lt(&self, other: &Self) -> bool {
        other.time < self.time
    }

    fn le(&self, other: &Self) -> bool {
        other.time <= self.time
    }

    fn gt(&self, other: &Self) -> bool {
        other.time > self.time
    }

    fn ge(&self, other: &Self) -> bool {
        other.time >= self.time
    }
}

impl<A: Application> Ord for EventNode<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Inverted call should act as reverse
        other.time.cmp(&self.time)
    }
}

impl<A: Application> Debug for EventNode<A>
where
    A::EventSet: Debug,
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
    A::EventSet: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventNode {{ id: {} time: {} event: {} }}",
            self.id, self.time, self.event
        )
    }
}
