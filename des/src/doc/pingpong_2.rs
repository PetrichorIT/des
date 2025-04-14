//! Ping-Pong as a generic network-simulation
//!
//! # The task
//!
//! Two nodes `Pinger` and `Ponger` want to communicate with each other
//! using a bidirection channel. `Pinger` sends a total of 30 `Ping`-messages
//! in intervals of 1s. `Ponger` receives these messages and
//! responds with a `Pong`-Message, that 'Pinger' receives. Both
//! `Pinger` and `Ponger` count the number of messages received and send
//! by them.
//!
//! # The simulation
//!
//! This network-simulation is created by using the features `net`.
//!
//! ## Network elements
//!
//! There are three kinds of components in a network-simulation:
//! - **modules / nodes / hosts**
//! - **gates**
//! - **channels**
//!
//! **Modules** are self contained objects that act as a network host. They contain
//! custom software to react to incoming messages from the underlying network. A host
//! can generally be seperated into two parts: the 'physical' host provided by the simulation
//! that enables integratio into the simulation runtime, and the custom software provided
//! by the user of this framework.
//!
//! Each node may contain an abitrary number of **gates**. Gates can be used to send
//! and receive messages into the underlying network layer. A node will automatically
//! receive any messages that exists a gate belonging to the node, but may choose which
//! gate will be used to send outgoing messages. Gates can be linked into bidirectional
//! chains that will forward messages to other gates, potentially belonging to other
//! nodes. A message will only 'exit' a gate at the end of a chain.
//!
//! By default gate chains will forward messages without delay. This however does not
//! accuratly represent reality. **Channels** can be attached to any connection between
//! two gates in a chain to delay messages traveling along the gate chain. This delay
//! will be computed based on the messages size and the channels metrics. Dependent
//! on configuration channels can also cause gate-chains to drop messages should the
//! channel be still busy sending other messages.
//!
//! ## Pure `net` implementation
//!
//! First of, lets define the two modules used in this simulation: `Pinger` and `Ponger`
//! and some constants we need along the way.
//!
//! ```
//! use des::prelude::*;
//!
//! #[derive(Default)]
//! struct Pinger {
//!     pings_send: usize,
//!     pongs_recv: usize,
//! }
//!
//! #[derive(Default)]
//! struct Ponger {
//!     pings_recv: usize,
//!     pongs_send: usize,
//! }
//!
//! const PING: MessageKind = 42;
//! const PONG: MessageKind = 43;
//! const INTERVAL: MessageKind = 44;
//! ```
//!
//! To act as a module, means to implement the trait [`Module`]. This trait defines
//! all nessecary interfaces for the software part of a host.
//!
//! Let first implement `Pinger`
//!
//! ```
//! # use des::prelude::*;
//! # struct Pinger { pings_send: usize, pongs_recv: usize }
//! # const PING: MessageKind = 42;
//! # const PONG: MessageKind = 43;
//! # const INTERVAL: MessageKind = 44;
//! impl Module for Pinger {
//!     fn at_sim_start(&mut self, stage: usize) {
//!         /* Schedule the first interval tick at t=0 */
//!         schedule_at(Message::default().kind(INTERVAL), SimTime::ZERO);
//!     }
//!
//!     fn handle_message(&mut self, msg: Message) {
//!         match msg.header().kind {
//!             INTERVAL => {
//!                 /* Send a ping message */
//!                 send(Message::default().kind(PING), "to-pong");
//!                 self.pings_send += 1;
//!
//!                 /* Reschedule the interval until t=30 */
//!                 if SimTime::now().as_secs() < 30 {
//!                     schedule_in(Message::default().kind(INTERVAL), Duration::from_secs(1))
//!                 }
//!             },
//!             PONG => self.pongs_recv += 1,
//!             _ => todo!()
//!         }
//!     }
//!
//!     fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
//!         assert_eq!(self.pings_send, 30);
//!         assert_eq!(self.pongs_recv, 30);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! The `at_sim_start` function will be called before the simulation is started (effectivly at t=0). This function
//! can be used on inject the first messages into the simulation, in our case the first tick of the interval. By calling
//! `schedule_at` or `schedule_in` a nodes sends a message to itself, without the need for a gate(-chain). This is most useful
//! when modelling time dependent features like timeouts or intervals. Each message will be annotated with a [`MessageKind`]
//! to keep them distingushable.
//!
//! The `handle_message` method is the core of a module. This function is called every time a message either exists a gate or arrives
//! thanks to a `schedule_*`. In our example, we differentiate between two possible events: An interval tick or the arrival of a pong
//! message. Should an interval tick happen we send a ping message and reschedule a interval tick (until we reach t=30). Sending
//! messages is done via the `send` function. This function requires a gate descriptor as a second parameter. In our case we will
//! create a gate called `to-pong` on `Pinger` that will be used to send messages.
//!
//! Finally the `at_sim_end` function will be called if the simulation ends. This function can be used to check results of the simulation.
//! Note that any new event created during the execution of this function will NOT be executed by the runtime. So calls to `send` will
//! never result in the arrival of a message at the other end of the gate chain.
//!
//! Lets follow up with the `Ponger`:
//!
//! ```
//! # use des::prelude::*;
//! # struct Ponger { pongs_send: usize, pings_recv: usize }
//! # const PING: MessageKind = 42;
//! # const PONG: MessageKind = 43;
//! impl Module for Ponger {
//!     fn handle_message(&mut self, msg: Message) {
//!         assert_eq!(msg.header().kind, PING);
//!         self.pings_recv += 1;
//!
//!         send(Message::default().kind(PONG), "to-ping");
//!         self.pongs_send += 1;
//!     }
//!
//!     fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
//!         assert_eq!(self.pongs_send, 30);
//!         assert_eq!(self.pings_recv, 30);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! As can be seen, nothing special here. Since `Ponger` does not send interval ticks, only `Pong` messages
//! are expected to be received, so `handle_message` can be simplified. Note that `Ponger` expects
//! a gate called `to-ping` to be created.
//!
//! Finally lets define the simulations physical components:
//!
//! ```
//! # use des::prelude::*;
//! # #[derive(Default)]
//! # struct Pinger {}
//! # impl Module for Pinger {}
//! # #[derive(Default)]
//! # struct Ponger {}
//! # impl Module for Ponger {}
//!
//! fn main() {
//!     let mut sim = Sim::new(());
//!     sim.node("pinger", Pinger::default());
//!     sim.node("ponger", Ponger::default());
//!
//!     let to_pong = sim.gate("pinger", "to-pong");
//!     let to_ping = sim.gate("ponger", "to-ping");
//!
//!     let channel = Channel::new(ChannelMetrics {
//!         bitrate: 8_000_000,
//!         latency: Duration::from_millis(5),
//!         jitter: Duration::ZERO,
//!         drop_behaviour: ChannelDropBehaviour::Drop,
//!     });
//!
//!     to_pong.connect(to_ping, Some(channel));
//!
//!     /* ... */
//! }
//! ```
//! The type [`Sim`] can be used to create network simulations. Sim contains an inner type `A` implementing
//! [`EventLifecycle`] to attach custom sim-start and sim-end actions to any network simulation. The unit
//! type `()` also implements `EventLifecycle` and is used as the default inner type. Nodes can be created using the
//! method [`Sim::node`]. This method requires a path and a instance of some type that implements [`Module`] no create
//! a physical node with an attached software component. In our case we create the instance of `Pinger` and `Ponger`
//! using the `Default` trait.
//!
//! Gates can be created using the method [`Sim::gate`]. These gate can be chained together, potentially
//! with a channel as part of the connection, using the [`Gate::connect`] method. In our case we
//! created two gates connected to each other with a channel as created aboth.
//!
//! The `Sim` object itself is a fully valid application and can thus be passed to the [`Builder`]:
//!
//! ```
//! # use des::prelude::*;
//! fn main() {
//!     /* ... */
//!     # let sim = Sim::new(());
//!
//!     /* Lets work with a conservative limit of 60s  */
//!     let rt = Builder::new().max_time(60.0.into()).build(sim);
//!     match rt.run() {
//!         /* ... */
//!         # _ => {}
//!     }
//! }
//! ```
//!
//! ## Building the physical components using feature `ndl`
//!
//! While defining the physical characteristica of a network was easy in this case, it
//! becomes bothersome in larger simulation. Therefore there exists NDL a description language
//! for the physical layout of a network. NDL allows for the reusable definition of all gates and channels
//! attached to a certain kind of node. In our example NDL remains rather simple.
//!
//! Lets define our topology in a file called `main.ndl`:
//! ```text
//! module Pinger {
//!     gates {
//!         to-pong
//!     }
//! }
//!
//! module Ponger {
//!     gates {
//!         to-ping
//!     }
//! }
//!
//! link MyLink {
//!     bitarate: 8000000,
//!     latency: 0.01,
//!     jitter: 0.0,
//! }
//!
//! module Main {
//!     submodules {
//!         pinger: Pinger,
//!         ponger: Ponger,
//!     }
//!
//!     connections {
//!         pinger/to-pong <-- MyLink --> ponger/to-ping,
//!     }
//! }
//!
//! entry Main;
//! ````
//!
//! First we define two modules `Pinger` and `Ponger` the both contain a simple gate
//! called `to-ping` and `to-pong` respectivly. Further we create another module that
//! composes these two together. These 'meta-modules' are often used in NDL since NDL
//! produces a complete topological module tree as output. `Main` defines two submodules
//! `pinger` and `ponger` and connects them using a link with our well-known delay characteristica.
//! Finally we define `Main` as the entry point to our simulation (aka. as the root of the module tree).
//!
//! Then we can create a simulation using this description:
//!
//! ```
//! # use des::{prelude::*, registry};
//! # #[derive(Default)]
//! # struct Pinger {}
//! # impl Module for Pinger {}
//! # #[derive(Default)]
//! # struct Ponger {}
//! # impl Module for Ponger {}
//! fn main() {
//!     # return;
//!     let registry = registry![Pinger, Ponger, else _];
//!     let sim = Sim::ndl("path/to/main.ndl", registry).expect("failed to generate NDL based simulation");
//!     let rt = Builder::new().build(sim);
//!     match rt.run() {
//!         /* ... */
//!         # _ => {}
//!     }
//! }
//! ```
//!
//! Use can create a NDL based simulation using the constructor [`Sim::ndl`]. This function expect a path to an
//! NDL file and a [`Registry`]. Since NDL only defines the physical components of a network simulation, the user
//! must still provide the software attached to nodes. This is done through a registry, a type that creates software
//! components based on the path and NDL name of created physical nodes. A registry can be most simply creted using the
//! [`registry`] macro. Note that **all** NDL module must be provided a software component, so this also includes the
//! 'meta-modules' like `Main`. To cirumvent this problem registry can provide fallback modules that are assigned if no better
//! software component could be found. The final term `else _` enables the default fallback module in the macro.
//!
//!
//! [`Module`]: crate::net::module::Module
//! [`Gate`]: crate::net::gate::Gate
//! [`Gate::connect`]: crate::net::gate::Gate::connect
//! [`MessageKind`]: crate::net::message::MessageKind
//! [`EventLifecycle`]: crate::runtime::EventLifecycle
//! [`Sim`]: crate::net::Sim
//! [`Sim::node`]: crate::net::Sim::node
//! [`Sim::gate`]: crate::net::Sim::gate
//! [`Sim::ndl`]: crate::net::Sim::ndl
//! [`Builder`]: crate::runtime::Builder
//! [`Registry`]: crate::ndl::Registry
