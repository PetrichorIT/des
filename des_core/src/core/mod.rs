mod event;

mod logger;
mod runtime;
mod sim_time;

// # Feature "pubintering"

#[cfg(not(feature = "pubinterning"))]
pub(crate) mod interning;

#[cfg(feature = "pubinterning")]
pub mod interning;

//
// # Exposed publics
//

pub use self::sim_time::SimTime;
pub use self::sim_time::SimTimeUnit;

pub use self::event::Application;
pub use self::event::Event;
pub use self::event::EventSet;

pub use self::runtime::rng;
pub use self::runtime::sample;
pub use self::runtime::sim_time;
pub use self::runtime::sim_time_fmt;
pub use self::runtime::Runtime;
pub use self::runtime::RuntimeOptions;

//
// # Hidden publics
//

pub(crate) use self::event::EventNode;
pub(crate) use self::logger::StandardLogger;
#[allow(unused)]
pub(crate) use self::runtime::RTC;
