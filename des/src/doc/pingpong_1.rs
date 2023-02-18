//! Ping-Pong as a simple event simulation
//!
//! # The task
//!
//! Two nodes 'Ping' and 'Pong' can communicate with each other
//! using a bidirection channel. 'Ping' sends 30 ping-messages
//! with at an interval of 1s. 'Pong' receives the messages and
//! responds with a pong-message, that 'Ping' receives. Both
//! 'Ping' and 'Pong' count the number of messages received and send
//! by them.
//!
//! # The simulation
//!
//! ### An event-set
//!
//! When constructing a simple event simulation, an event set must be provided.
//! This event-set should be able to represent all activity that can happen
//! within the scope of the simulation. If this cast this encompasses:
//!
//! - The 1s interval to send ping-messages
//! - The ping-messages that will be received by 'Pong'
//! - The pong-message that will be received by 'Ping'
//!
//! Accordingly an event set can be constructed:
//!
//! ```rust
//! use des::prelude::*;
//! use des::create_event_set;
//!
//! create_event_set!(
//!     enum PingPongEventSet {
//!         type App = PingPongApp;
//!     
//!         IntervalEvent(IntervalEvent),
//!         PingArrival(PingArrival),
//!         PongArrival(PongArrival),
//!     };
//! );
//!
//! struct IntervalEvent;
//! struct PingArrival;
//! struct PongArrival;
//! #
//! # struct PingPongApp { /* ... */ }
//! # impl Application for PingPongApp {
//! #    type EventSet = PingPongEventSet;
//! #    type Lifecycle = ();
//! # }
//! # impl Event<PingPongApp> for IntervalEvent { fn handle(self, _rt: &mut Runtime<PingPongApp>) {} }
//! # impl Event<PingPongApp> for PingArrival { fn handle(self, _rt: &mut Runtime<PingPongApp>) {} }
//! # impl Event<PingPongApp> for PongArrival { fn handle(self, _rt: &mut Runtime<PingPongApp>) {} }
//! ```
//!
//! ### An application
//!
//! However to define an event-set you must define an application first.
//! This application serves as a global persistent storage point that manages
//! the lifecycle of the simulation. To define a application define a
//! abitray type that implements the trait [Application](crate::runtime::Application).
//! In this example we will use the application to record messages and check the number
//! of messages after the simulation has concluded:
//!
//! ```rust
//! # use des::prelude::*;
//! #[derive(Debug)]
//! struct PingPongApp {
//!     pings_send: usize,
//!     pings_recv: usize,
//!     pongs_send: usize,
//!     pongs_recv: usize,
//! }
//!
//! impl Application for PingPongApp {
//!     type EventSet = PingPongEventSet;
//!     type Lifecycle = Self;
//! }
//! impl EventLifecycle for PingPongApp {
//!     fn at_sim_end(rt: &mut Runtime<Self>) {
//!         assert_eq!(rt.app.pings_send, 30);
//!         assert_eq!(rt.app.pings_recv, 30);
//!         assert_eq!(rt.app.pongs_send, 30);
//!         assert_eq!(rt.app.pongs_recv, 30);    
//!     }
//! }
//! # struct PingPongEventSet;
//! # impl EventSet<PingPongApp> for PingPongEventSet { fn handle(self, _: &mut Runtime<PingPongApp>) {} }
//! ```
//!
//! ### The event handlers
//!
//! Now we have to specify what happens when a event is executed. Therefor each event in the event set
//! must implement the trait [Event](crate::runtime::Event). This trait includes a handler that consumes
//! the event itself, while holding a reference to the runtime to create new event if nessecary. In
//! out example we shall first define the interval event which sends messages from 'Ping' to
//! 'Pong':
//!
//! ```rust
//! # use des::prelude::*;
//! # struct IntervalEvent;
//! impl Event<PingPongApp> for IntervalEvent {
//!     fn handle(self, rt: &mut Runtime<PingPongApp>) {
//!         // Send a ping message, that will arrive in 20ms
//!         rt.add_event_in(PingArrival, Duration::from_millis(20));
//!         rt.app.pings_send += 1;
//!         // reschedule the interval event
//!         if SimTime::now().as_secs() < 30 {
//!             rt.add_event_in(self, Duration::from_secs(1));
//!         }
//!     }
//! }
//! # struct PingPongApp { pings_send: usize }
//! # struct PingPongEventSet {}
//! # impl Application for PingPongApp { type EventSet = PingPongEventSet; type Lifecycle = (); }
//! # impl EventSet<PingPongApp> for PingPongEventSet { fn handle(self, _: &mut Runtime<PingPongApp>) {}}
//! # impl From<IntervalEvent> for PingPongEventSet { fn from(_: IntervalEvent) -> Self { todo!() }}
//! # struct PingArrival;
//! # impl From<PingArrival> for PingPongEventSet { fn from(_: PingArrival) -> Self { todo!() }}
//! ```
//!
//! After that lets, define what happens once the ping-message arrives:
//!
//! ```rust
//! # use des::prelude::*;
//! # struct PingArrival;
//! impl Event<PingPongApp> for PingArrival {
//!     fn handle(self, rt: &mut Runtime<PingPongApp>) {
//!         // Bounce back a pong message, that will arrive in 20ms
//!         rt.add_event_in(PongArrival, Duration::from_millis(20));
//!         rt.app.pings_recv += 1;
//!         rt.app.pongs_send += 1;
//!     }
//! }
//! # struct PingPongApp { pings_recv: usize, pongs_send: usize }
//! # struct PingPongEventSet {}
//! # impl Application for PingPongApp { type EventSet = PingPongEventSet; type Lifecycle = (); }
//! # impl EventSet<PingPongApp> for PingPongEventSet { fn handle(self, _: &mut Runtime<PingPongApp>) {}}
//! # impl From<PingArrival> for PingPongEventSet { fn from(_: PingArrival) -> Self { todo!() }}
//! # struct PongArrival;
//! # impl From<PongArrival> for PingPongEventSet { fn from(_: PongArrival) -> Self { todo!() }}
//! ```
//!
//! And finally lets define what happens once the pong arrives:
//!
//! ```rust
//! # use des::prelude::*;
//! # struct PongArrival;
//! impl Event<PingPongApp> for PongArrival {
//!     fn handle(self, rt: &mut Runtime<PingPongApp>) {
//!         rt.app.pongs_recv += 1;
//!     }
//! }
//! # struct PingPongApp { pongs_recv: usize }
//! # struct PingPongEventSet {}
//! # impl Application for PingPongApp { type EventSet = PingPongEventSet;type Lifecycle = ();  }
//! # impl EventSet<PingPongApp> for PingPongEventSet { fn handle(self, _: &mut Runtime<PingPongApp>) {}}
//! ```
//!
//! # The main function
//!
//! Now we have createa all that is nessecary to perform our event simulation.
//! Thus we must define the main function and provide the inital event that is nessecary.
//!
//! ````rust
//! # use des::prelude::*;
//! fn main() {
//!     # return;
//!     let app = PingPongApp {
//!         pings_send: 0, pings_recv: 0,
//!         pongs_send: 0, pongs_recv: 0,
//!     };
//!     let mut rt = Runtime::new(app);
//!     rt.add_event(IntervalEvent, SimTime::ZERO);
//!     let result = rt.run();
//!     println!("{:?}", result);
//! }
//! # #[derive(Debug)]
//! # struct PingPongApp { pings_send: usize, pings_recv: usize, pongs_send: usize, pongs_recv: usize }
//! # struct PingPongEventSet {}
//! # struct IntervalEvent;
//! # impl Application for PingPongApp { type EventSet = PingPongEventSet;type Lifecycle = ();  }
//! # impl EventSet<PingPongApp> for PingPongEventSet { fn handle(self, _: &mut Runtime<PingPongApp>) {}}
//! # impl From<IntervalEvent> for PingPongEventSet { fn from(_: IntervalEvent) -> Self { todo!()}}
//! ````
