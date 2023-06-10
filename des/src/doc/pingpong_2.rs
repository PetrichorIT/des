//! Ping-Pong as a generic network-simulation
//!
//! # The task
//!
//! Two nodes 'Ping' and 'Pong' want to communicate with each other
//! using a bidirection channel. 'Ping' sends a total of 30 `Ping`-messages
//! in intervals of 1s. 'Pong' receives these messages and
//! responds with a `Pong`-Message, that 'Ping' receives. Both
//! 'Ping' and 'Pong' count the number of messages received and send
//! by them.
//!
//! # The simulation
//!
//! This network-simulation is created by using the features `net` and `ndl`.
//!
//! ### NDL
//!
//! The feature `net` provides the core abstractions for a network-like
//! simulation. These are Modules, Gates and Channels. Modules represent
//! network nodes with custom state and behaviour. They are user defined
//! and can be created by implementing the [`Module`](crate::net::module::Module)
//! trait on a type. Gates act as physical (or logical) ports on a module. They can be
//! chained together into gate-chains, thus connecting multiple modules.
//! By default gate-chains act as link with infinite bandwith and zero latency.
//! If a gate-chain should act as a real physical link would do, Channels
//! can be attached to a gate chain to define the delay / drop metric of the link.
//!
//! While the feature `net` provides the appropiate base abstractions, creating
//! a network can be tiresome. However, using the feature `ndl`, users
//! can automatically create networks by defining just the networks
//! topology using the `NetworkDescriptionLanguage`. Such definitions
//! can be placed in files ending in '.ndl'. This language desribes networks
//! as a topology of modules, gates and links, without requiring any custom
//! logic that will later be associated with the modules. At first let's
//! describe the network at hand:
//!
//! ```text
//! // 'Main.ndl'
//! module Ping {
//!     gates {
//!         in @input,
//!         out @output,
//!     }
//! }
//!
//! module Pong {
//!     gates {
//!         in @input,
//!         out @output,
//!     }
//! }
//! ```
//!
//! We define two module, 'Ping' and 'Pong' that both possesing two gates.
//! Links in NDL are unidirectional so each modules requires two to facilitate bidirectional
//! communication. Gates can also be annotated with their typ (input or output) to prevent
//! unwanted topologies. Using this definition, both modules can be sure, that all incoming
//! packets must come via the 'in' gate. Now using the basic definition of our two modules
//! we may create our network.
//!
//! ```text
//! // 'Main.ndl'
//! link MyLink {
//!     bitrate: 100000
//!     latency: 0.1
//!     jitter: 0.0
//! }
//!
//! module MyNetwork {
//!     submodules {
//!         ping: Ping,
//!         pong: Pong,
//!     }
//!     connections {
//!         ping/out --> MyLink --> pong/in,
//!         pong/out --> MyLink --> ping/in,
//!     }
//! }
//!
//! entry MyNetwork;
//! ```
//!
//! The module `MyNetwork` represents the entry point to our simulation. While `MyNetwork` itself could
//! act as a network node, it is more of an abstract composite node in this example. By declaring
//! two submodules 'ping' and 'pong' we declare, that each instance of `MyNetwork` should contain
//! a `Ping` and a `Pong` instance. In the connections section we define a link (gate-chain)
//! between the output gate of 'ping' and the input gate if 'pong' (and vice versa).
//! This gate chain will be augmented using a Channel with the characteristics defined
//! on `MyLink`. Finally we declare the module `MyNetwork` to be the entry point / root of
//! our network.
//!
//! # The Modules
//!
//! Once we have defined the network topology, modules can be defined in rust code.
//! For that you may define a struct or enum of the with the same name as the described
//! module.
//!
//! ```
//! # use des::prelude::*;
//! struct Ping {
//!     pongs_recv: usize,
//!     pings_send: usize,
//! }
//! struct Pong {
//!     pings_recv: usize,
//!     pongs_send: usize,
//! }
//! ```
//!
//! To be a module, this type must implement the trait [`Module`](crate::net::module::Module).
//! This trait provides a number of available functions,
//! but only [`Module::new`](crate::net::module::Module::new)m is required on all modules.
//! This function should be used to create a new instance of the custom state for a
//! network node. Note that this function is not nessecryly executed within the context
//! of an event, so dont put complex custom logic here.
//!
//! ```rust
//! # use des::prelude::*;
//! # struct Ping { pongs_recv: usize, pings_send: usize }
//! # struct Pong { pings_recv: usize, pongs_send: usize }
//! /* ... */
//!
//! impl Module for Ping {
//!     fn new() -> Self {
//!         Self {
//!             pongs_recv: 0,
//!             pings_send: 0,
//!         }
//!     }
//!     /* ... */
//! }
//!
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
//! The `Module` trait also provide some other useful functions, that can be overrided.
//! [`Module::handle_message`](crate::net::module::Module::handle_message)
//!  is called when a packet arrives at the module. This function
//! is the heart of most network simulations.
//! [`Module::at_sim_start`](crate::net::module::Module::at_sim_start) provides a way to
//! handle more complex logic when the simulation is stared, but now within a fully constructed
//! topology.  [`Module::at_sim_end`](crate::net::module::Module::at_sim_end)
//! can be used to make module-specific actions once the simulation is finished,
//! such as writing metrics to a file, or deallocating internal containers.
//!
//! ```
//! # use des::prelude::*;
//! # struct Ping { pongs_recv: usize, pings_send: usize }
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
//! Now that we have defined the **real** modules we can do the same with the  more abstract modules.
//! Since we dont have any intersting buisness logic for this module, we just
//! insert some placeholder code.
//!
//! ```
//! # use des::prelude::*;
//! /* ... */
//!
//! struct MyTestCase;
//! impl Module for MyTestCase {
//!     fn new() -> MyTestCase {
//!         Self
//!     }
//! }
//! ```
//!
//! Now we have defined everything to create the simulation. To do that create an
//! [`NdlApplication`](crate::ndl::NdlApplication) to load our network topology. This application requies
//! a [`Registry`](crate::ndl::Registry) of all known modules types, to link the Ndl-Modules to their rust struct.
//! This application can be used to instantiate a [`NetworkApplication`](crate::net::NetworkApplication)
//! (provided by feature `net`),
//! which in turn can be passed to the core [`Runtime`](crate::runtime::Runtime) of [`des`](crate).
//! This runtime can than be executed, to run the simulation to its end.
//!
//! ```
//! # use des::prelude::*;
//! # use des::registry;
//! # struct Ping;
//! # impl Module for Ping { fn new() -> Self { Self }}
//! # struct Pong;
//! # impl Module for Pong { fn new() -> Self { Self }}
//! # struct MyTestCase;
//! # impl Module for MyTestCase { fn new() -> Self { Self }}
//! /* ... */
//!
//! fn main() {
//!     # return;
//!     let app = NdlApplication::new("main.ndl", registry![Ping, Pong, MyTestCase]).unwrap();
//!     let rt = Builder::new().build(NetworkApplication::new(app));
//!     let result = rt.run();
//!     println!("{:?}", result);
//! }
//! ```
