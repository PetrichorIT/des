//!
//! Convenience re-export of common members.
//!

//
// # Generic core exports
//

pub use crate::time::Duration;
pub use crate::time::SimTime;

//
// # feature = "net"
//

pub use crate::message::Header;
pub use crate::message::Message;
pub use crate::message::MessageBody;
pub use crate::message::MessageId;
pub use crate::message::MessageKind;

pub use crate::message::{schedule_at, schedule_in, send, send_at, send_in};

pub use crate::runtime::Globals;
pub use crate::runtime::Sim;
pub use crate::runtime::Spawner;

pub use crate::channel::Channel;
pub use crate::channel::ChannelDropBehaviour;
pub use crate::channel::ChannelRef;
pub use crate::channel::DatarateChannel;
pub use crate::channel::DatarateChannelMetrics;
pub use crate::channel::SendError;

pub use crate::gate::Gate;
pub use crate::gate::GateRef;
pub use crate::gate::IntoGate;

pub use crate::topology::Topology;

pub use crate::module::Module;
pub use crate::module::ModuleRef;

pub use crate::module::{current, try_current};

pub use crate::ObjectPath;
pub use crate::processing::ProcessingElement;

pub use std::net::IpAddr;
pub use std::net::Ipv4Addr;
pub use std::net::Ipv6Addr;
pub use std::net::SocketAddr;
pub use std::net::SocketAddrV4;
pub use std::net::SocketAddrV6;

pub use des_macros::*;
