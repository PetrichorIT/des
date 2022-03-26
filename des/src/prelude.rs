//!
//! Convenience re-export of common members.
//!

//
// # Generic core exports
//

pub use crate::core::Runtime;
pub use crate::core::RuntimeOptions;
pub use crate::core::RuntimeResult;

pub use crate::core::SimTime;

pub use crate::core::Application;
pub use crate::core::Event;
pub use crate::core::EventSet;

pub use crate::core::rng;
pub use crate::core::sample;
pub use crate::core::sim_time;

//
// # Metrics & Misc
//

pub use crate::metrics::OutVec;
#[doc(hidden)]
pub use crate::metrics::Statistic;
pub use crate::metrics::StdDev;

pub use crate::util::Mrc;
pub use crate::util::SpmcReader;
pub use crate::util::SpmcWriter;

//
// # feature = "net"
//

#[cfg(feature = "net")]
pub use crate::net::NetworkRuntime;

#[cfg(feature = "net")]
pub use crate::net::Channel;
#[cfg(feature = "net")]
pub use crate::net::ChannelMetrics;
#[cfg(feature = "net")]
pub use crate::net::ChannelRef;

#[cfg(feature = "net")]
pub use crate::net::Gate;
#[cfg(feature = "net")]
pub use crate::net::GateDescription;
#[cfg(feature = "net")]
pub use crate::net::GateRef;
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
pub use crate::net::BuildableModule;
#[cfg(feature = "net")]
pub use crate::net::Module;
#[cfg(feature = "net")]
pub use crate::net::ModuleCore;
#[cfg(feature = "net")]
pub use crate::net::ModuleId;
#[cfg(feature = "net")]
pub use crate::net::ModuleRef;
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