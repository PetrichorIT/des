//!
//! Tools for building a module/net oriented simulation.
//!

mod error;
mod path;
pub(crate) mod runtime;

pub mod channel;
pub mod gate;
pub mod message;
pub mod module;
pub mod ndl;
pub mod processing;
pub mod topology;

pub use self::error::*;
pub use self::path::*;
pub use self::runtime::{
    Globals, IntoModuleTree, Sim, SimBuilder, SimLifecycle, Spawner, SpawnerKind, fail, globals,
    handlers, schedule_event,
};

/// Internal details only sometimes needed to e.g. implement a custom channel.
pub mod internals {
    pub use super::runtime::NetEvents;
    pub use super::runtime::{
        ChannelUnbusyNotif, HandleMessageEvent, MessageExitingConnection, ModuleRestartEvent,
        ModuleShutdownEvent,
    };

    cfg_async! {
        pub use super::runtime::AsyncWakeupEvent;
    }
}
