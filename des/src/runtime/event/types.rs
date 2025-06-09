use crate::runtime::{Runtime, RuntimeError};

///
/// A trait that defines an runtime application
/// that depends on a event set to be processed by the
/// runtime and a lifecylce managment.
///
pub trait Application: Sized {
    ///
    /// The set of events used in the simulation.
    ///
    type EventSet: Event<Self>;
    ///
    /// A global type, defining the behavior at sim start / sim end
    ///
    type Lifecycle: EventLifecycle<Self>;
}

///
/// A type that can handle an event, specific to the given aplication,
/// and associated event set.
///
/// Note that events in an event set dont need to implement this trait,
/// unless the event set is derived using the [`event_set`](crate::event_set)
/// macros. Nonetheless is it advised to use this trait to better isolate different events
/// and their associated data.
///
pub trait Event<App>
where
    App: Application,
{
    ///
    /// A function to handle an upcoming event represented as a specific
    /// instance of a event type.
    ///
    /// There is an implicit type bound that the Apps event set must contain
    /// the Self type as a variant. This is usually guaranteed by macro-generting event sets,
    /// but could lead to unexpected behaviour if not done properly in custom
    /// event set implementations.
    ///
    fn handle(self, runtime: &mut Runtime<App>);
}

///
/// A type that defines the lifecycle behaviour of an application A.
///
pub trait EventLifecycle<A = Self> {
    ///
    /// A function that is called only once at the start of the simulation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # struct Worker;
    /// # impl Worker { fn initalize(&mut self) {}}
    /// # enum MyEventSet { EventA, EventB }
    /// # impl Event<MyApp> for MyEventSet {
    /// #   fn handle(self, rt: &mut Runtime<MyApp>) {}
    /// # }
    /// struct MyApp { workers: Vec<Worker> };
    /// impl Application for MyApp {
    ///     type EventSet = MyEventSet;
    ///     type Lifecycle = Self;
    /// }
    /// impl EventLifecycle for MyApp {
    ///     fn at_sim_start(runtime: &mut Runtime<Self>) {
    ///         runtime.app.workers.iter_mut().for_each(|w| w.initalize());
    ///     }
    /// }
    /// ```
    ///
    #[allow(unused_variables)]
    fn at_sim_start(runtime: &mut Runtime<A>)
    where
        A: Application,
    {
    }

    ///
    /// A function that is called once the simulation reachted its limit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # struct Worker;
    /// # impl Worker { fn finish(&mut self) {}}
    /// # enum MyEventSet { EventA, EventB }
    /// # impl Event<MyApp> for MyEventSet {
    /// #   fn handle(self, rt: &mut Runtime<MyApp>) {}
    /// # }
    /// struct MyApp { workers: Vec<Worker> };
    /// impl Application for MyApp {
    ///     type EventSet = MyEventSet;
    ///     type Lifecycle = Self;
    /// }
    /// impl EventLifecycle for MyApp {
    ///     fn at_sim_end(rt: &mut Runtime<Self>) -> Result<(), RuntimeError> {
    ///         rt.app.workers.iter_mut().for_each(|w| w.finish());
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function may return an error, if some situation occured, that
    /// indicates an overall failure of the simulation. This error will be propagated
    /// to [`Runtime::run`].
    #[allow(unused_variables)]
    fn at_sim_end(runtime: &mut Runtime<A>) -> Result<(), RuntimeError>
    where
        A: Application,
    {
        Ok(())
    }
}

impl<A: Application> Event<A> for () {
    fn handle(self, _: &mut Runtime<A>) {}
}

impl<A> EventLifecycle<A> for () {}

///
/// A runtime unqiue identifier for a event.
///
pub(crate) type EventId = usize;

impl<F: FnMut() -> R, R> Application for F {
    type EventSet = ();
    type Lifecycle = Self;
}

impl<F: FnMut() -> R, R> EventLifecycle<F> for F {
    fn at_sim_start(runtime: &mut Runtime<F>)
    where
        F: Application,
    {
        (runtime.app)();
    }
}
