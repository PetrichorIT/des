//!
//! Convenience re-export of common members.
//!

//
// # Generic core exports
//

pub use crate::runtime::Builder;
pub use crate::runtime::Runtime;
pub use crate::runtime::RuntimeError;

pub use crate::time::Duration;
pub use crate::time::SimTime;

pub use crate::runtime::Application;
pub use crate::runtime::Event;
pub use crate::runtime::EventLifecycle;
pub use crate::runtime::EventSet;

pub use crate::runtime::random;
pub use crate::runtime::sample;

//
// # feature = "net"
//

cfg_net! {
    pub use crate::net::message::CustomSizeBody;
    pub use crate::net::message::Message;
    pub use crate::net::message::MessageBody;
    pub use crate::net::message::MessageId;
    pub use crate::net::message::MessageKind;
    pub use crate::net::message::MessageHeader;

    pub use crate::net::message::{send, send_in, send_at, schedule_in, schedule_at};

    pub use crate::net::Sim;
    pub use crate::net::ScopedSim;
    pub use crate::net::Globals;
    pub use crate::net::Watcher;

    pub use crate::net::channel::Channel;
    pub use crate::net::channel::ChannelMetrics;
    pub use crate::net::channel::ChannelRef;
    pub use crate::net::channel::ChannelDropBehaviour;

    pub use crate::net::gate::Gate;
    pub use crate::net::gate::GateRef;

    pub use crate::net::topology::Topology;

    pub use crate::net::module::Module;
    pub use crate::net::module::ModuleId;
    pub use crate::net::module::ModuleRef;
    pub use crate::net::module::ModuleReferencingError;

    pub use crate::net::module::{
        current, shutdow_and_restart_at, shutdow_and_restart_in, shutdown
    };


    pub use crate::net::ObjectPath;
    pub use crate::net::{par, par_for};

    pub use crate::net::processing::ProcessingElement;

    pub use crate::net::ndl::Registry;
    pub use crate::net::ndl::RegistryCreatable;

    pub use std::net::IpAddr;
    pub use std::net::Ipv4Addr;
    pub use std::net::Ipv6Addr;
    pub use std::net::SocketAddr;
    pub use std::net::SocketAddrV4;
    pub use std::net::SocketAddrV6;

    //
    // Export the derives if net
    //

    pub use des_macros::*;
}
