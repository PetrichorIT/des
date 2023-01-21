//!
//! Tools for building a module/net oriented simulation.
//!

mod common;
mod ndl;
mod runtime;
mod topology;

pub mod channel;
pub mod gate;
pub mod message;
pub mod module;
pub mod plugin2;
pub mod subsystem;

pub(crate) use self::runtime::HandleMessageEvent;
pub(crate) use self::runtime::MessageAtGateEvent;
pub(crate) use self::runtime::NetEvents;

pub use self::runtime::globals;
pub use self::runtime::NetworkRuntime;
pub use self::runtime::NetworkRuntimeGlobals;

pub use self::ndl::BuildContext;
pub use self::ndl::__Buildable0;
pub use self::ndl::__Buildable1;
pub use self::ndl::__Buildable2;
pub use self::ndl::__Buildable3;
pub use self::ndl::__Buildable4;
pub use self::ndl::__Buildable5;
pub use self::ndl::__Buildable6;
pub use self::ndl::__Buildable7;

pub use self::common::ObjectPath;
pub use self::common::ObjectPathParseError;
pub use self::common::ParHandle;
pub use self::common::Parameters;

pub use self::topology::TopoEdge;
pub use self::topology::TopoNode;
pub use self::topology::Topology;
