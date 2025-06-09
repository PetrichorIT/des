//! Implementing a Ping-Pong application using a custom build event simulation.
//!
//! ## The task
//!
//! The simulation should describe 30 individual 'pings', spaced one second apart, being send to a peer
//! and responded by with a equivalent 'pong'. The number of received pings and pongs should be counted
//! in the global scope.
//!
//! # Requirements
//!
//! This implementation only uses the base implementation of `des` and does not require any features or dependencies.
use des::{event_set, prelude::*};

// ## Defining Events
//
// The first thing to do is to define events required for the simulation. In this
// case there will be the following events:
//
// - Interval: A repeating event that once executed, sends a 'ping' an reschedules itself.
//   After the 30th iteration it stops rescheduling itself.
// - Ping: A event that represents the sending of a 'ping'. When the event is executed it represents
//   the receiving of a 'ping' message
// - Pong: same as the Ping event.

struct Interval(usize);
struct Ping;
struct Pong;

// The buisness logic of an event is defined in the `Event` trait.
// This trait is generic over an `App: Application`, which defines the global scope of a simulation.
//
// All logic is defined in the `handle` function, which is called when an event is due.
// This function is provided with a exclusive borrow to the runtime, to enqueue new events
// or interact with the global scope.
//
// In the case of the `Interval` event, we schedule a `Ping` event that takes 1.5 seconds to arrive
// and then reschedule the `Interval` event itself, as long as the contained counter is greater than 0.

impl Event<PingPongApp> for Interval {
    fn handle(self, runtime: &mut Runtime<PingPongApp>) {
        runtime.add_event_in(Ping, Duration::from_secs_f64(1.5));
        if self.0 != 0 {
            runtime.add_event_in(Interval(self.0 - 1), Duration::from_secs(1));
        }
    }
}

// The `Ping` and `Pong` events do additionaly access global state, using the
// `app` property of the runtime. This field represents the generic global scope
// provided to the trait implementation, in this case the `PingPongApp`.
//
// Since the state is mutably accesable, events can modify the global state.

impl Event<PingPongApp> for Ping {
    fn handle(self, runtime: &mut Runtime<PingPongApp>) {
        runtime.app.pings_received += 1;
        runtime.add_event_in(Pong, Duration::from_secs(1));
    }
}

impl Event<PingPongApp> for Pong {
    fn handle(self, runtime: &mut Runtime<PingPongApp>) {
        runtime.app.pongs_received += 1;
    }
}

// ## Event sets
//
// Since this simulation uses more than one event type, these types need to be combined to form a
// singular event set. An event set is just another type that implements the `Event` trait, and multiplexes
// between its variants. The `event_set!` macro implements such a multimplexing type easily.

event_set! {
    enum PingPongEvent {
        type App = PingPongApp;

        Ping(Ping),
        Pong(Pong),
        Interval(Interval),
    };
}

// ## Applications
//
// To bind a set of events to a global simulation scope, we need to define an application.
// An application is some tpye, that represents the global state and implements the trait `Application`:
//
// This trait defines two associates types:
// - EventSet: The type of events that should be used for this application.
// - Lifecycle: A proxy type that can be used to manage the simulation lifecycle (usually Self)

struct PingPongApp {
    pings_received: usize,
    pongs_received: usize,
}

impl Application for PingPongApp {
    type EventSet = PingPongEvent;
    type Lifecycle = Self;
}

// The most imporant definition of the `EventLifecycle` trait that must be implemented
// for any proxy in the `Application::Lifecylce` position, is `at_sim_start`.
//
// This function is called once the simulation has been started and has access to all
// internal APIs and globals. Think of it like the 0th event.
//
// In this case we simply create the inital `Interval` event with a counter set to 29
// so that the `Interval` repeats 30 times, thus sending 30 pings.

impl EventLifecycle for PingPongApp {
    fn at_sim_start(runtime: &mut Runtime<Self>)
    where
        Self: Application,
    {
        runtime.add_event_in(Interval(29), Duration::ZERO);
    }
}

// ## Running the simulation
//
// The last thing to do is to create an application, and put it into a runtime, using the `Builder` API
// to build a runtime. The resulting `rt` object can either be run, one event at a time using the `dispatch_*`
// methods, or all at once using `run`. All dispatch calls are failable, since the simulation may emit
// `RuntimeError`s that cause the simulation run to fail. This can be used to e.g. search for failures
// of algorithms under specific seeds.
//
// Once the simulation is finished, results from the global scope as well as metadata can be
// inspeced.

fn main() -> Result<(), RuntimeError> {
    let app = PingPongApp {
        pings_received: 0,
        pongs_received: 0,
    };
    let rt = Builder::new().build(app);
    let (app, _, profile) = rt.run()?;

    assert_eq!(app.pings_received, 30);
    assert_eq!(app.pongs_received, 30);

    assert_eq!(profile.event_count, 90);

    Ok(())
}
