//!
//! A discrete event simulator.
//!
//!
//! # Features
//!
//! - 'net' A module for simulating network like module structures.
//! - 'netipv6' A modifer for the net feature that uses 128 bit addresses.
//! - 'pub_interning' A modifier that enables public acess to the interner.
//! - 'simtime_u128' A modifier that enables the simulation to use u128 timestamps
//! for maximum precision (this is ca. 10% slower than default).
//! - 'static_gates' A modifier that enables further optimization when the user guarantees to
//! create no new gates after the simulation was started.
//!

pub(crate) mod core;
pub(crate) mod metrics;
pub(crate) mod util;

#[cfg(feature = "net")]
pub(crate) mod net;

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

pub use util::Indexable;
