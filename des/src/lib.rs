// Lints
#![deny(unused_must_use)]
#![warn(clippy::pedantic)]
#![warn(missing_docs, missing_debug_implementations, unreachable_pub)]
#![allow(
    clippy::needless_doctest_main,
    rustdoc::broken_intra_doc_links,
    clippy::module_name_repetitions
)]

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
//! with completely generic event set. This can be done independent of features used,
//! but usually only optimization features like `cqueue` or montioring tools like `metrics`
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
//! If a event is executed [`MyEventSet::handle`](crate::runtime::EventSet:handle)
//! will be called with the runtime as parameter. If new events are to be created
//! as result of a event execution this mutable reference can be used
//! to add new events to the future event set.
//!
//! The [`Application`](crate::runtime::Application) object (in this case `MyApp`) is used as a global context handle that
//! it stored inside the runtime. It can be accessed via 'rt.app' and can be used
//! to record [`metrics`](crate::metrics) during the simulation. Note that the [`EventSet`](crate::runtime::EventSet)
//! and the [`Application`](crate::runtime::Application) are linked via a trait with generic parameters. This means
//! that `MyEvents` could implement [`EventSet`](crate::runtime::EventSet) a second time for another application.
//!
//! Additionally DES provides access to the [`util`] module to easier crate event-sets ,
//! aswell as access to a [`prelude`](crate::prelude).
//!
//! # Using a module oriented system
//!
//! DES is able to provide tools for simulating network-like structures with [Modules](crate::net::Module).
//! These modules are self contained units with their own state, connected via [Channels](crate::net::channel::Channel)
//! (network links) that are attached to [Gates](crate::net::gate::Gate) (physical ports) on modules.
//! Modules can send messages (packtes) through these gates / channels to communicated
//! with other modules. Additionally modules can be created in a tree like structure,
//! providing links like [`parent`](crate::net::ModuleCore::parent) or
//! [`child(...)`](crate::net::ModuleCore::child).
//!
//! These tools are available in the [`net`](crate::net) module
//! when the feature `net` is active.
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
//! help with asynchronously managing module activity. With this feature
//! active, network-primitives like `TcpListener` or `UdpSocket`,
//! as well as time-primitives like `tokio::time::sleep` can be
//! used.
//!
//! ```toml
//! des = { version = "*", features = [ "net", "async" ] }
//! ```
//!
//! While this feature activates smaller additions to the existing functionallity of
//! [`net`](crate::net), it also contains a full reexport of [tokio](https://docs.rs/tokio) with modifications
//! to fit the simulation context. This version of tokio is implicitly reexported with the
//! newly added feature sim to integrate into a simulation context and thus does NOT
//! provide access to the [`fs`](https://docs.rs/tokio/latest/tokio/fs/index.html),
//! [`signal`](https://docs.rs/tokio/latest/tokio/signal/index.html) or modules.
//! Additionally this version only supports current-thread runtimes.
//!
//! However it supports all synchronisation primitives (excluding Barrier)
//! through the [`sync`](tokio::sync) module, asynchronous green tasks
//! through [`task`](tokio::task), custom runtimes through [`runtime`](tokio::rumtime)
//! and simulation specific time primitives through [`sim`](tokio::sim) replacing the
//! standart [`time`](https://docs.rs/tokio/latest/tokio/time/index.html) module,
//! aswell as simulation specifc network primitives replacing the standart
//! [`net`](https://docs.rs/tokio/latest/tokio/net/index.html) module.
//!

pub mod doc;

#[macro_use]
#[doc(hidden)]
pub mod macros;

pub mod prelude;

pub mod runtime;
pub mod stats;
pub mod time;

cfg_cqueue! {
    pub(crate) mod cqueue;
}

cfg_net! {
    pub mod net;
}

cfg_async! {
    ///
    /// A modified version of tokio with added simulation support.
    ///
    pub use ::tokio as tokio;
}

// # Features
//
// | Feature          | Description                                                              |
// |------------------|--------------------------------------------------------------------------|
// | net              | Adds a module oriented design-abstraction that provides its own events.  |
// | cqueue           | Configures the runtime to use a calender queue for better performance.   |
// | metrics | Collects internal metrics about the runtime, to improve parametrization. |
// | async            | Provides utilites and modifications for simulating asynchronous systems including a full reexport of safe tokio funtions. |
//
