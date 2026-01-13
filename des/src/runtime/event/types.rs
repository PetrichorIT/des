use crate::runtime::Runtime;

use std::fmt;

///
/// A trait that defines an runtime application
/// that depends on a event set to be processed by the
/// runtime and a lifecylce managment.
///
pub trait Application: Sized {
    /// The error type that can be returned by the application.
    type Error: fmt::Debug;
    /// The set of events used in the simulation.
    type EventSet: Event<Self>;

    ///
    /// A function that is called only once at the start of the simulation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use std::convert::Infallible;
    /// # struct Worker;
    /// # impl Worker { fn initalize(&mut self) {}}
    /// # enum MyEventSet { EventA, EventB }
    /// # impl Event<MyApp> for MyEventSet {
    /// #   fn handle(self, rt: &mut Runtime<MyApp>)  -> Result<(), Infallible> { Ok(()) }
    /// # }
    /// struct MyApp { workers: Vec<Worker> };
    /// impl Application for MyApp {
    ///     type Error = Infallible;
    ///     type EventSet = MyEventSet;
    ///
    ///     fn at_sim_start(runtime: &mut Runtime<Self>) -> Result<(), Infallible> {
    ///         runtime.app.workers.iter_mut().for_each(|w| w.initalize());
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function may return an error that will in turn stop the simulation startup.
    #[allow(unused_variables)]
    fn at_sim_start(runtime: &mut Runtime<Self>) -> Result<(), Self::Error> {
        Ok(())
    }

    ///
    /// A function that is called once the simulation reachted its limit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use std::convert::Infallible;
    /// # struct Worker;
    /// # impl Worker { fn finish(&mut self) {}}
    /// # enum MyEventSet { EventA, EventB }
    /// # impl Event<MyApp> for MyEventSet {
    /// #   fn handle(self, rt: &mut Runtime<MyApp>)  -> Result<(), Infallible> { Ok(()) }
    /// # }
    /// struct MyApp { workers: Vec<Worker> };
    /// impl Application for MyApp {
    ///     type Error = Infallible;
    ///     type EventSet = MyEventSet;
    ///
    ///     fn at_sim_end(rt: &mut Runtime<Self>) -> Result<(), Infallible> {
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
    fn at_sim_end(runtime: &mut Runtime<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
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
    /// There is an implicit type bound that the Apps event set must contain
    /// the Self type as a variant. This is usually guaranteed by macro-generting event sets,
    /// but could lead to unexpected behaviour if not done properly in custom
    /// event set implementations.
    ///
    /// # Errors
    ///
    /// This function may return an error that will in turn stop the simulation.
    fn handle(self, runtime: &mut Runtime<App>) -> Result<(), App::Error>;
}

impl<A: Application> Event<A> for () {
    fn handle(self, _: &mut Runtime<A>) -> Result<(), A::Error> {
        Ok(())
    }
}

///
/// A runtime unqiue identifier for a event.
///
pub(crate) type EventId = usize;
