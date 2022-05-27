//!
//! Convenience re-export of common members.
//!

//
// # Generic core exports
//

pub use crate::runtime::Runtime;
pub use crate::runtime::RuntimeOptions;
pub use crate::runtime::RuntimeResult;

pub use crate::time::Duration;
pub use crate::time::SimTime;

pub use crate::runtime::Application;
pub use crate::runtime::Event;
pub use crate::runtime::EventSet;

pub use crate::runtime::rng;
pub use crate::runtime::sample;
pub use crate::runtime::sim_time;

//
// # Metrics & Misc
//

pub use crate::metrics::OutVec;
#[doc(hidden)]
pub use crate::metrics::Statistic;
pub use crate::metrics::StdDev;

pub use crate::util::Ptr;
pub use crate::util::PtrConst;
pub use crate::util::PtrMut;

pub use crate::util::PtrWeak;
pub use crate::util::PtrWeakConst;
pub use crate::util::PtrWeakMut;

//
// # feature = "net"
//

#[cfg(feature = "net")]
pub use crate::net::NetworkRuntime;
#[cfg(feature = "net")]
pub use crate::net::NetworkRuntimeGlobals;

#[cfg(feature = "net")]
pub use crate::net::Channel;
#[cfg(feature = "net")]
pub use crate::net::ChannelMetrics;
#[cfg(feature = "net")]
pub use crate::net::ChannelRef;
#[cfg(feature = "net")]
pub use crate::net::ChannelRefMut;

#[cfg(feature = "net")]
pub use crate::net::Gate;
#[cfg(feature = "net")]
pub use crate::net::GateDescription;
#[cfg(feature = "net")]
pub use crate::net::GateRef;
#[cfg(feature = "net")]
pub use crate::net::GateRefMut;
#[cfg(feature = "net")]
pub use crate::net::GateServiceType;
#[cfg(feature = "net")]
pub use crate::net::IntoModuleGate;

#[cfg(feature = "net")]
pub use crate::net::CustomSizeBody;
#[cfg(feature = "net")]
pub use crate::net::Message;
#[cfg(feature = "net")]
pub use crate::net::MessageBody;
#[cfg(feature = "net")]
pub use crate::net::MessageId;
#[cfg(feature = "net")]
pub use crate::net::MessageKind;
#[cfg(feature = "net")]
pub use crate::net::MessageMetadata;

#[cfg(feature = "net")]
pub use crate::net::NodeAddress;
#[cfg(feature = "net")]
pub use crate::net::Packet;
#[cfg(feature = "net")]
pub use crate::net::PacketHeader;
#[cfg(feature = "net")]
pub use crate::net::PortAddress;
#[cfg(feature = "net")]
pub use crate::net::NODE_ADDR_BROADCAST;
#[cfg(feature = "net")]
pub use crate::net::NODE_ADDR_LOOPBACK;

#[cfg(feature = "net")]
pub use crate::net::Module;
#[cfg(feature = "net")]
pub use crate::net::ModuleCore;
#[cfg(feature = "net")]
pub use crate::net::ModuleId;
#[cfg(feature = "net")]
pub use crate::net::ModuleRef;
#[cfg(feature = "net")]
pub use crate::net::ModuleRefMut;
#[cfg(feature = "net")]
pub use crate::net::ModuleReferencingError;
#[cfg(feature = "net")]
pub use crate::net::NameableModule;
#[cfg(feature = "net")]
pub use crate::net::StaticModuleCore;

#[cfg(feature = "net")]
pub use crate::net::ModulePath;
#[cfg(feature = "net")]
pub use crate::net::Parameters;

#[cfg(feature = "net")]
pub use crate::net::NodeDefinition;
#[cfg(feature = "net")]
pub use crate::net::Topology;

//
// Export the derives if net
//

#[cfg(feature = "net")]
pub use des_derive::*;
