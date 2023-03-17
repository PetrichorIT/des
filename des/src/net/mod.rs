//!
//! Tools for building a module/net oriented simulation.
//!

mod par;
mod path;
mod runtime;
mod topology;

pub mod channel;
pub mod gate;
pub mod message;
pub mod module;
pub mod plugin;

pub(crate) use self::runtime::HandleMessageEvent;
pub(crate) use self::runtime::MessageAtGateEvent;
pub(crate) use self::runtime::NetEvents;

pub use self::runtime::globals;
pub use self::runtime::NetworkRuntime;
pub use self::runtime::NetworkRuntimeGlobals;

pub use self::par::*;
pub use self::path::ObjectPath;

pub use self::topology::TopoEdge;
pub use self::topology::TopoNode;
pub use self::topology::Topology;
