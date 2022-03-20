mod event;

mod logger;
mod runtime;
mod sim_time;

// # Feature "pubintering"

#[cfg(not(feature = "pub-interning"))]
pub(crate) mod interning;

#[cfg(feature = "pub-interning")]
pub mod interning;

//
// # Exposed publics
//

pub use self::sim_time::SimTime;

pub use self::event::Application;
pub use self::event::Event;
pub use self::event::EventId;
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
pub(crate) use self::runtime::RTC;
