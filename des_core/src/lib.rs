//!
//! A discrete event simulator.
//!
//!
//! # Features
//!
//! - 'net' A module for simulating network like module structures.
//! - 'net-ipv6' A modifer for the net feature that uses 128 bit addresses.
//! - 'net-static' A modifier that enables optimizations for static simulation enviroments.
//! - 'simtime-u128' A modifier that enables the simulation to use u128 timestamps
//! for maximum precision (this is ca. 10% slower than default).
//! - 'internal-metrics' A modifier that enables internal metrics for event runtime internal
//! parameters for debugging.
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

#[cfg(feature = "net")]
pub use util::Indexable;
