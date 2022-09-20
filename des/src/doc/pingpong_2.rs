//! Ping-Pong as a generic network-simulation
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
//! This network-simulation is created by using the features `net`.
//!
//! ### NDL
//!
//! When using the more abstract tools provided by the feature `net`,
//! you must firstly define the topology of network you want to simulate.
//! This can be done using the `NetworkDescriptionLanguage` [ndl] in a file
//! ending with '.ndl'. This language describes a network through
//! modules, gates and channels. At first lets decribe the module required
//! for our example:
//!
//! ```text
//! // 'Main.ndl'
//! module Ping {
//!     gates:
//!         in @input
//!         out @output
//! }
//!
//! module Pong {
//!     gates:
//!         in @input
//!         out @output
//! }
//! ```
//!
//! We define two module, 'Ping' and 'Pong' that both possess two gates. Gates describe a
//! physical or virtual port of a network node and are used to route messages. Gates
//! can be connected to each other to forward messages to other modules. This connection
//! can be enhanced by suppling a channel, that will delay messages on this link.
//! Such connections can be descirbed in the parent element of a module, which can
//! be either a module itself, or a subsystem. A subsystem describes either part of the
//! test case or the complete test case itself. Thus we further define:
//!
//! ```text
//! // 'Main.ndl'
//! link MyLink {
//!     bitrate: 100000
//!     latency: 0.1
//!     jitter: 0.0
//! }
//!
//! subsystem MyTestCase {
//!     nodes:  
//!         ping: Ping
//!         pong: Pong
//!     connections:
//!         ping/out --> MyLink --> pong/in
//!         pong/out --> MyLink --> ping/in
//! }
//! ```
//!
//! # The Modules
//!
//! Once we have defined the network topology, modules can be defined in rust code.
//! For that you may define a struct or enum of the with the same name as the described
//! module. To link the type and the module use the [`NdlModule`](crate::prelude::NdlModule)
//! macro. Note that you must provide the macro with a relative path to the workspace.
//!
//! > Note that the test cases are ignored, since they require access to the filesystem at compile time.
//!
//! ```ignore
//! # use des::prelude::*;
//! #[NdlModule("src")]
//! struct Ping {
//!     pongs_recv: usize,
//!     pings_send: usize,
//! }
//! #[NdlModule("src")]
//! struct Pong {
//!     pings_recv: usize,
//!     pongs_send: usize,
//! }
//! ```
//!
//! Now DES knows how to construct modules with the correct gates, and which types to use.
//! However should the modules contain fields, they will need a constructor to build
//! the inital state. To provide this constructor they must implement the [NameableModule](crate::net::NameableModule)
//! trait. This trait is automatically derived on empty types, but must be manually implemented
//! for all other case. Note that the [`NdlModule`](crate::prelude::NdlModule) macro attached a new field
//! `__core` to the type:
//!
//! ```rust
//! # use des::prelude::*;
//! # #[NdlModule]
//! # struct Ping { pongs_recv: usize, pings_send: usize }
//! # #[NdlModule]
//! # struct Pong { pings_recv: usize, pongs_send: usize }
//! impl Module for Ping {
//!     fn new() -> Self {
//!         Self {
//!             pongs_recv: 0,
//!             pings_send: 0,
//!         }
//!     }
//!     /* ... */
//! }
//! impl Module for Pong {
//!     fn new() -> Self {
//!         Self {
//!             pings_recv: 0,
//!             pongs_send: 0,
//!         }
//!     }
//!     /* ... */
//! }
//! ```
//!
//! Once module construction is finished you can define the behaviour of the module by implementing
//! the [`Module`](crate::net::Module) trait. Noteably you can use the [`handle_message`](crate::net::Module::handle_message)
//! function to react to arriving packets
//!
//! ```
//! # use des::prelude::*;
//! # #[NdlModule]
//! # struct Ping { pongs_recv: usize, pings_send: usize }
//! # #[NdlModule]
//! # struct Pong { pings_recv: usize, pongs_send: usize }
//! const PING: MessageKind = 10;
//! const PONG: MessageKind = 42;
//! const WAKEUP: MessageKind = 69;
//!
//! impl Module for Ping {
//! # fn new() -> Self { todo!() }
//!     /* ... */
//!
//!     fn at_sim_start(&mut self, _stage: usize) {
//!         // Create the inital wakeup event.
//!         schedule_at(Message::new().kind(WAKEUP).build(), SimTime::ZERO)
//!     }
//!
//!     fn handle_message(&mut self, msg: Message) {
//!         match msg.header().kind {
//!             WAKEUP => {
//!                 // Send a PING every 1s, for the first 30s
//!                 send(Message::new().kind(PING).build(), "out");
//!                 self.pings_send += 1;
//!                 if SimTime::now().as_secs() < 30 {
//!                     schedule_in(msg, Duration::from_secs(1));
//!                 }
//!             },
//!             PONG => self.pongs_recv += 1,
//!             _ => todo!()
//!         }    
//!     }
//!
//!     fn at_sim_end(&mut self) {
//!         assert_eq!(self.pongs_recv, 30);
//!         assert_eq!(self.pings_send, 30);
//!     }
//! }
//!
//! impl Module for Pong {
//! # fn new() -> Self { todo!() }
//!     /* ... */
//!
//!     fn handle_message(&mut self, msg: Message) {
//!         assert_eq!(msg.header().kind, PING);
//!         self.pings_recv += 1;
//!         send(Message::new().kind(PONG).build(), "out");
//!         self.pongs_send += 1;
//!     }
//!
//!     fn at_sim_end(&mut self) {
//!         assert_eq!(self.pings_recv, 30);
//!         assert_eq!(self.pongs_send, 30);
//!     }
//! }
//! ```
//!
//! ### The app
//!
//! Now that we have defined the modules we can do the same with the test case / the subsystem.
//! Define a struct with the same name, and bind it using the [`NdlSubsystem`](crate::prelude::NdlSubsystem)
//! macro.
//!
//! ```ignore
//! # use des::prelude::*;
//! #[NdlSubsystem("src")]
//! #[derive(Debug, Default)]
//! struct MyTestCase {}
//! ```
//!
//! Now we have defined everything to create the simulation. To do that create a instance of
//! the application, call the [`build_rt`] function and use the provided [`NetworkRuntime`]
//! to power a simulation.
//!
//! ```ignore
//! # use des::prelude::*;
//! # #[NdlSubsystem()]
//! # #[derive(Debug, Default)]
//! # struct MyTestCase {}
//! fn main() {
//!     # return;
//!     let app = MyTestCase::default().build_rt();
//!     let rt = Runtime::new(app);
//!     let result = rt.run();
//!     println!("{:?}", result);
//! }
//! ```
