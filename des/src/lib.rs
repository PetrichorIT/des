// Lints
#![deny(unused_must_use)]
#![warn(clippy::pedantic)]
#![warn(missing_docs, missing_debug_implementations, unreachable_pub)]
#![allow(
    clippy::needless_doctest_main,
    clippy::module_name_repetitions,
    clippy::arc_with_non_send_sync
)]

//!
//! A discrete event simulator.
//!
//! DES is a discrete event simulation tool that makes building simulations for
//! networks easy. DES provides the tools to build a event simulation from the
//! groud up, with a implemented module system or with a asynchronous context in
//! mind.
//!
//! DES is able to provide tools for simulating network-like structures with [Modules](crate::module::Module).
//! These modules are self contained units with their own state, connected via [Channels](crate::channel::Channel)
//! (network links) that are attached to [Gates](crate::gate::Gate) (physical ports) on modules.
//! Modules can send messages (packtes) through these gates / channels to communicated
//! with other modules. Additionally modules can be created in a tree like structure,
//! providing links like [`parent`] or [`child`].
//!
//! These tools are available in the [`net`] module
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
//! as well as time-primitives like `des::time::sleep` can be
//! used.
//!
//! ```toml
//! des = { version = "*", features = [ "net", "async" ] }
//! ```
//!
//! While this feature activates smaller additions to the existing functionallity of
//! [`net`], it also contains a full reexport of [tokio](https://docs.rs/tokio) with modifications
//! to fit the simulation context. This version of tokio is implicitly reexported with the
//! newly added feature sim to integrate into a simulation context and thus does NOT
//! provide access to the [`fs`](https://docs.rs/tokio/latest/tokio/fs/index.html),
//! [`signal`](https://docs.rs/tokio/latest/tokio/signal/index.html) or modules.
//! Additionally this version only supports current-thread runtimes.
//!
//! However it supports all synchronisation primitives (excluding Barrier)
//! through the [`sync`](tokio::sync) module, asynchronous green tasks
//! through [`task`](tokio::task), custom runtimes through [`runtime`](tokio::runtime)
//! and simulation specific time primitives through [`time`] replacing the
//! standart [`time`](https://docs.rs/tokio/latest/tokio/time/index.html) module,
//! aswell as simulation specifc network primitives replacing the standart
//! [`net`](https://docs.rs/tokio/latest/tokio/net/index.html) module.
//!
//! Look for the `pingpong-*` examples for more detailed explanations.
//!
//! [`time`]: crate::time
//! [`net`]: crate
//! [`runtime`]: crate::runtime
//! [`parent`]: crate::module::ModuleContext::parent
//! [`child`]: crate::module::ModuleContext::child

#[macro_use]
#[doc(hidden)]
pub mod macros;
pub mod prelude;
// pub mod runtime;
pub mod time;

pub mod tracing;
pub(crate) use des_sync_utils as sync;

cfg_macros! {
    pub use des_macros::*;
}

mod error;
mod path;

/// The simulation runtime.
pub mod runtime;

pub mod channel;
pub mod gate;
pub mod message;
pub mod module;
pub mod processing;
pub mod statistics;
pub mod topology;

pub use self::error::*;
pub use self::path::*;
pub use self::runtime::{
    Sim, SimBuilder, fail, globals, random, report, rng, sample, schedule_event,
};
