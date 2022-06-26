#![feature(unsize)]
#![feature(dispatch_from_dyn)]
#![feature(coerce_unsized)]
#![feature(arbitrary_self_types)]
#![feature(const_option_ext)]
#![feature(box_into_inner)]
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(clippy::needless_doctest_main)]
//!
//! A discrete event simulator.
//!
//! DES is a discrete event simulation tool that makes building simulations for
//! networks easy. DES provides the tools to build a event simulation from the
//! groud up, with a implemented module system or with a asynchronous context in
//! mind.
//!
//! # Building a simple event simulation
//!
//! At its core DES provides the tools to easily and efficently build an event simulation
//! with completely custom events. This can be done independent of features used,
//! but usually only optimization features like `cqueue` or montioring tools like `internal-metrics`
//! are used in this context.
//!
//! ```
//! use des::prelude::*;
//!
//! enum MyEventSet {
//!     EventA { what_happend: String },
//!     EventB { ack: bool },
//! }
//!
//! impl EventSet<MyApp> for MyEventSet {
//!     fn handle(self, _rt: &mut Runtime<MyApp>) {
//!         // Do something
//!     }
//! }
//!
//! #[derive(Default)]
//! struct MyApp {
//!     global_value: usize,
//!     logs: Vec<String>,
//! }
//!
//! impl Application for MyApp {
//!     type EventSet = MyEventSet;
//! }
//!
//! fn main() {
//!     let app = MyApp::default();
//!     let rt = Runtime::new(app);
//!
//!     let result = rt.run();
//! }
//! ```
//!
//! This simulation will now provide a [`runtime`](crate::runtime) with
//! [`time`](crate::time) managment and a future event set to execute events.
//! If a event is executed [MyEventSet::handle](crate::runtime::EventSet:handle)
//!  will be called with the runtime
//! as parameter. If new events are to be created as result of a event execution this
//! mutable reference can be used to add new events to the future event set.
//!
//! The [Application](crate::runtime::Application) object (in this case 'MyApp') is used as a global context handle that
//! it stored inside the runtime. It can be accessed via 'rt.app' and can be used
//! to record [`metrics`](crate::metrics) during the simulation. Note that the [EventSet](crate::runtime::EventSet)
//! and the [Application](crate::runtime::Application) are linked via a trait with generic parameters. This means
//! that 'MyEvents' could implement [EventSet](crate::runtime::EventSet) a second time for another application.
//!
//! Additionally DES provides access to the [`util`] module to easier crate event-sets ,
//! aswell as access to a [`prelude`](crate::prelude).
//!
//! # Using a module oriented system
//!
//! DES is able to provide tools for simulating network-like structures with [Modules](crate::net::Module).
//! These modules are self contained units with their own state, connected via [Channels](crate::net::Channel)
//! (network links) that are attached to [Gates](crate::net::Gate) (physical ports) on modules.
//! Modules can send messages (packtes) through these gates / channels to communicated
//! with other modules. Additionally modules can be created in a tree like structure,
//! providing links like [parent](crate::net::ModuleCore::parent) or
//! [child(with_name)](crate::net::ModuleCore::child).
//!
//! These tools are available in the [`net`](crate::net) module
//! when the feature `net` or `net-ipv6` is active.
//!
//! ```toml
//! des = { version = "*", features = [ "net" ] }
//! ```
//!
//! # Asynchrounous simulation
//!
//! As a final addition DES provides tools for dealing with the simulation of
//! asynchronous systems through the feature `async`.
//! These tools are build onto of the feature `net` and
//! help with asynchronously managing a call to [`handle_message`](crate::net::Module::handle_message).
//!
//! ```toml
//! des = { version = "*", features = [ "net", "async" ] }
//! ```
//!
//! While this feature activates smaller additions to the existing functionallity of
//! [`net`](crate::net), it also contains a full reexport of [tokio](https://docs.rs/tokio) with modifications
//! to fit the simulation context. For example the [signal](https://docs.rs/tokio/latest/tokio/signal/index.html)
//! module is NOT reexported since awaiting signals will block the entire simulation and does serve
//! not purpose since signals to the simulation process have nothing to do
//! with signals to the async codebase.
//!
//! However modules like [`fs`](crate::fs), [`io`](crate::io), [`process`](crate::process),
//! [`task`](crate::task) and [`stream`](crate::stream) are reexported to
//! make DES a swap in replacement for tokio, to minimize differences in the codebase
//! between a prototype and a simulation-ready prototype.  It should be noted
//! that neither the filesystem nor the process management should be used excessivly
//! since they block the entire simulation. Thus they should only be used when absoloutly
//! nessecary (e.g. when reading the config files at sim_start).
//!
//! The [`tokio::net`](https://docs.rs/tokio/latest/tokio/net/index.html) module however is
//! NOT reexported since this communication layer is implemented by the simulation
//! itself in its own [`net`](crate::net) module. However those two modules have nothing in common
//! since DES is not designed for direct network access.
//!
//!

pub mod prelude;

pub mod metrics;
pub mod runtime;
pub mod time;
pub mod util;

#[cfg(feature = "net")]
pub mod net;

#[cfg(feature = "async")]
pub use des_tokio::*;

// # Features
//
// | Feature          | Description                                                              |
// |------------------|--------------------------------------------------------------------------|
// | net              | Adds a module oriented design-abstraction that provides its own events.  |
// | net-ipv6         | Configures the net module to use IPv6 addresses.                         |
// | cqueue           | Configures the runtime to use a calender queue for better performance.   |
// | internal-metrics | Collects internal metrics about the runtime, to improve parametrization. |
// | async            | Provides utilites and modifications for simulating asynchronous systems including a full reexport of safe tokio funtions. |
//
