mod channel;
mod gate;
mod message;
mod module;
mod packet;
mod runtime;

// pub(crate) use self::runtime::ChannelUnbusyNotif; <unused>
// pub(crate) use self::runtime::CoroutineMessageEvent; <unused>
pub(crate) use self::runtime::HandleMessageEvent;
pub(crate) use self::runtime::MessageAtGateEvent;
pub(crate) use self::runtime::NetEvents;

pub use self::runtime::NetworkRuntime;

//
// # Channel definitions
//

pub use self::channel::Channel;
pub use self::channel::ChannelMetrics;

pub use self::channel::ChannelId;
pub use self::channel::CHANNEL_INSTANTANEOUS;
pub use self::channel::CHANNEL_NULL;
pub use self::channel::CHANNEL_SELF;

//
// # Gate definitions
//

pub use self::gate::Gate;
pub use self::gate::GateDescription;
pub use self::gate::IntoModuleGate;

pub use self::gate::GateId;
pub use self::gate::GATE_NULL;
pub use self::gate::GATE_SELF;

//
// # Messages & Packets
//

pub use self::message::Message;
pub use self::message::MessageBody;
pub use self::message::MessageId;
pub use self::message::MessageKind;
pub use self::message::MessageMetadata;

pub use self::packet::Packet;
pub use self::packet::PacketId;

pub use self::packet::PortAddress;

pub use self::packet::NodeAddress;
pub use self::packet::NODE_ADDR_BROADCAST;
pub use self::packet::NODE_ADDR_LOOPBACK;

//
// # Modules
//

pub use self::module::ModuleId;
pub use self::module::MODULE_NULL;

pub use self::module::Module;
pub use self::module::ModuleCore;
pub use self::module::NdlBuildableModule;
pub use self::module::NdlCompatableModule;
pub use self::module::StaticModuleCore;
