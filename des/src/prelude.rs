//!
//! Convenience re-export of common members.
//!

//
// # Generic core exports
//

pub use crate::assert_eq_time;

pub use crate::runtime::Runtime;
pub use crate::runtime::RuntimeOptions;
pub use crate::runtime::RuntimeResult;

pub use crate::time::Duration;
pub use crate::time::SimTime;

pub use crate::runtime::Application;
pub use crate::runtime::Event;
pub use crate::runtime::EventSet;

pub use crate::runtime::random;
pub use crate::runtime::sample;
pub use crate::runtime::sim_time;

pub use crate::runtime::ScopedLogger;
pub use crate::runtime::StandardLogger;

//
// # Metrics & Misc
//

pub use crate::stats::OutVec;
#[doc(hidden)]
pub use crate::stats::Statistic;
pub use crate::stats::StdDev;

pub use crate::util::Ptr;
pub use crate::util::PtrConst;
pub use crate::util::PtrMut;

pub use crate::util::PtrWeak;
pub use crate::util::PtrWeakConst;
pub use crate::util::PtrWeakMut;

//
// # feature = "net"
//

cfg_net! {
    pub use crate::net::NetworkRuntime;
    pub use crate::net::NetworkRuntimeGlobals;

    pub use crate::net::Channel;
    pub use crate::net::ChannelMetrics;
    pub use crate::net::ChannelRef;
    // pub use crate::net::ChannelRefMut;

    pub use crate::net::Gate;
    pub use crate::net::GateDescription;
    pub use crate::net::GateRef;
    // pub use crate::net::GateRefMut;
    pub use crate::net::GateServiceType;
    // pub use crate::net::IntoModuleGate;

    pub use crate::net::CustomSizeBody;
    pub use crate::net::Message;
    pub use crate::net::MessageBody;
    pub use crate::net::MessageId;
    pub use crate::net::MessageKind;
    pub use crate::net::MessageHeader;
    pub use crate::net::MessageType;

    pub use crate::net::Module;
    // pub use crate::net::ModuleCore;
    pub use crate::net::ModuleId;
    pub use crate::net::ModuleRef;
    // pub use crate::net::ModuleRefMut;
    pub use crate::net::ModuleReferencingError;
    // pub use crate::net::NameableModule;
    // pub use crate::net::StaticModuleCore;

    pub use crate::net::{
        child, gate, gates, module_id, module_name, module_path, par, parent, pars, schedule_at,
        schedule_in, send, send_at, send_in, shutdow_and_restart_at, shutdow_and_restart_in, shutdown,
    };


    pub use crate::net::StaticSubsystemCore;
    pub use crate::net::SubsystemCore;
    pub use crate::net::SubsystemId;

    pub use crate::net::ObjectPath;
    pub use crate::net::Parameters;

    pub use crate::net::NodeDefinition;
    pub use crate::net::Topology;

    cfg_async! {
        pub use crate::net::AsyncModule;

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
