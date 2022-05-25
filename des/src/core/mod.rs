//!
//! The core functionallity of a discrete event simulator.
//!

mod event;
mod logger;
mod runtime;
mod sim_time;

//
// # Exposed publics
//

pub use self::sim_time::Duration;
pub use self::sim_time::SimTime;

pub use self::event::Application;
pub use self::event::Event;
pub use self::event::EventSet;

pub use self::runtime::rng;
pub use self::runtime::sample;
pub use self::runtime::sim_time;
pub use self::runtime::Runtime;
pub use self::runtime::RuntimeOptions;
pub use self::runtime::RuntimeResult;

//
// # Hidden publics
//

pub(crate) use self::logger::StandardLogger;
#[allow(unused)]
pub(crate) use self::runtime::get_rtc_ptr;
#[allow(unused)]
pub(crate) use self::runtime::RuntimeCore;
#[allow(unused)]
pub(crate) use self::runtime::RTC;
