#![feature(unsize)]
#![feature(dispatch_from_dyn)]
#![feature(coerce_unsized)]
#![feature(arbitrary_self_types)]

//!
//! A discrete event simulator.
//!
//!
//! # Features
//!
//! | Feature          | Description                                                              |
//! |------------------|--------------------------------------------------------------------------|
//! | net              | Adds a module oriented design-abstraction that provides its own events.  |
//! | net-ipv6         | Configures the net module to use IPv6 addresses.                         |
//! | cqueue           | Configures the runtime to use a calender queue for better performance.   |
//! | simtime-u128     | Configures the runtime to use a high precsion time primitiv.             |
//! | internal-metrics | Collects internal metrics about the runtime, to improve parametrization. |
//!

pub(crate) mod core;
pub(crate) mod metrics;
pub(crate) mod util;

#[cfg(feature = "net")]
mod net;

//
// # Generic core exports
//

pub use crate::core::*;

//
// # Metrics & Misc
//

pub use crate::metrics::Statistic;
pub use crate::metrics::StdDev;

//
// # feature = "net"
//

#[cfg(feature = "net")]
pub use crate::net::*;

pub use util::mm::Mrc;
pub use util::spmc::*;
