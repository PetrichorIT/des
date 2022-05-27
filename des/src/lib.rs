#![feature(unsize)]
#![feature(dispatch_from_dyn)]
#![feature(coerce_unsized)]
#![feature(arbitrary_self_types)]
#![feature(const_option_ext)]

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
//! | internal-metrics | Collects internal metrics about the runtime, to improve parametrization. |
//!

pub mod prelude;

pub mod metrics;
pub mod runtime;
pub mod time;
pub mod util;

#[cfg(feature = "net")]
pub mod net;
