use super::{
    registry::{PluginEntry, PluginState},
    Plugin,
};
use std::{any::Any, fmt, panic, sync::Arc};

/// A policy that defines the behaviour if a plugin paniced.
#[derive(Default, Clone)]
pub enum PluginPanicPolicy {
    /// This option captures the paniced state of the plugin and deactivats it.
    /// This means that he plugin must still be manually removed form the plugin list.
    /// The plugins status will indicate the paniced state.
    #[default]
    Capture,

    /// This option passes the panic through to the simulation context,
    /// crashing the entire simulation.
    Abort,

    /// This option removes the current version of the plugin from the module
    /// and adds a new plugin in its place. The new plugin will share
    /// all configuration parameters.
    Restart(Arc<dyn Fn() -> Box<dyn Plugin>>),
}

impl PluginPanicPolicy {
    pub(super) fn activate(&self, entry: &mut PluginEntry, payload: Box<dyn Any + Send>) {
        match self {
            Self::Capture => {}
            Self::Abort => panic::resume_unwind(payload),
            Self::Restart(creation_fn) => {
                entry.core = Some(creation_fn());
                entry.state = PluginState::JustCreated;
            }
        }
    }
}

impl fmt::Debug for PluginPanicPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Abort => write!(f, "Policy::Abort"),
            Self::Capture => write!(f, "Policy::Capture"),
            Self::Restart(_) => write!(f, "Policy::Restart"),
        }
    }
}

/// The status of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginStatus {
    /// The plugin is in the process of being created.
    /// It will be active once the next event arrives.
    StartingUp,
    /// The plugin is in th process of shutting down.
    /// It will not exist once the next event arrives.
    ShutingDown,
    /// The plugin is running smoothly.
    Active,
    /// The plugin paniced.
    Paniced,
    /// The plugin is gone
    Gone,
}

impl PluginStatus {
    pub(super) fn from_entry(entry: &PluginEntry) -> Self {
        match entry.state {
            PluginState::Paniced => Self::Paniced,
            PluginState::PendingRemoval => Self::ShutingDown,
            _ => Self::Active,
        }
    }
}
