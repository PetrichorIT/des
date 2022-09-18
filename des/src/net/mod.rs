//!
//! Tools for building a module/net oriented simulation.
//!

mod channel;
mod common;
mod gate;
mod message;
mod module;
mod ndl;
mod runtime;
mod subsystem;
mod topology;

pub mod hooks;

pub(crate) use self::runtime::HandleMessageEvent;
pub(crate) use self::runtime::MessageAtGateEvent;
pub(crate) use self::runtime::NetEvents;

pub use self::runtime::globals;
pub use self::runtime::NetworkRuntime;
pub use self::runtime::NetworkRuntimeGlobals;

//
// # Channel definitions.
//

pub use self::channel::Channel;
pub use self::channel::ChannelMetrics;
pub use self::channel::ChannelRef;

//
// # Gate definitions
//

pub use self::gate::Gate;
pub use self::gate::GateDescription;
pub use self::gate::GateRef;
pub use self::gate::GateRefWeak;
pub use self::gate::GateServiceType;

//
// # Messages & Packets
//

pub use self::message::CustomSizeBody;
pub use self::message::Message;
pub use self::message::MessageBody;
pub use self::message::MessageBuilder;
pub use self::message::MessageHeader;
pub use self::message::MessageId;
pub use self::message::MessageKind;
pub use self::message::MessageType;

//
// # Modules
//

pub use self::module::Module;
pub use self::module::ModuleId;
pub use self::module::ModuleRef;
pub use self::module::ModuleReferencingError;

cfg_async! {
    pub use self::module::AsyncModule;
}

pub use self::module::{
    child, create_hook, gate, gates, module_id, module_name, module_path, par, parent, pars,
    schedule_at, schedule_in, send, send_at, send_in, shutdow_and_restart_at,
    shutdow_and_restart_in, shutdown,
};

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

//
// # Topology
//

pub use self::topology::NodeDefinition;
pub use self::topology::Topology;

//
// # Subsystem
//

pub use self::subsystem::Subsystem;
pub use self::subsystem::SubsystemContext;
pub use self::subsystem::SubsystemId;
pub use self::subsystem::SubsystemRef;
