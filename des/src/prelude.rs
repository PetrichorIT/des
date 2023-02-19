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
pub use crate::runtime::EventLifecycle;
pub use crate::runtime::EventSet;

pub use crate::runtime::random;
pub use crate::runtime::sample;

#[allow(deprecated)]
pub use crate::runtime::sim_time;

pub use crate::logger::Logger;

//
// # Metrics & Misc
//

pub use crate::stats::OutVec;
#[doc(hidden)]
pub use crate::stats::Statistic;
pub use crate::stats::StdDev;

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
    pub use crate::net::message::MessageType;

    pub use crate::net::message::{send, send_in, send_at, schedule_in, schedule_at};

    pub use crate::net::NetworkRuntime;
    pub use crate::net::NetworkRuntimeGlobals;

    pub use crate::net::channel::Channel;
    pub use crate::net::channel::ChannelMetrics;
    pub use crate::net::channel::ChannelRef;

    pub use crate::net::gate::Gate;
    pub use crate::net::gate::GateRef;
    pub use crate::net::gate::GateServiceType;


    pub use crate::net::module::Module;
    pub use crate::net::module::ModuleId;
    pub use crate::net::module::ModuleRef;
    pub use crate::net::module::ModuleReferencingError;

    pub use crate::net::module::{
        child, gate, gates, module_id, module_name, module_path, par, par_for, parent, pars, shutdow_and_restart_at, shutdow_and_restart_in, shutdown
    };

    pub use crate::net::subsystem::Subsystem;
    pub use crate::net::subsystem::SubsystemRef;
    pub use crate::net::subsystem::SubsystemContext;
    pub use crate::net::subsystem::SubsystemId;

    pub use crate::net::ObjectPath;
    pub use crate::net::Parameters;

    pub use crate::net::TopoNode;
    pub use crate::net::Topology;

    cfg_ndl! {
        pub use crate::ndl::NdlApplication;
        pub use crate::ndl::Registry;
    }

    cfg_async! {
        pub use ::tokio;
        pub use crate::net::module::AsyncModule;
    }

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
