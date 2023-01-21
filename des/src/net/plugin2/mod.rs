//! Plugin v2
use std::any::TypeId;

use crate::prelude::Message;

mod panic;
pub use self::panic::{PluginPanicPolicy, PluginStatus};

mod api;
pub use self::api::{add_plugin, add_plugin_with, PluginHandle};

mod error;
pub use self::error::{PluginError, PluginErrorKind};

mod registry;
pub(crate) use self::registry::PluginRegistry;

mod util;
pub(crate) use self::util::UnwindSafeBox;

cfg_async! {
    mod time;
    pub use time::TokioTimePlugin;
}

/// A module-specific plugin.
pub trait Plugin: 'static {
    /// A handler for when an the event processing of a message starts.
    fn event_start(&mut self) {}

    /// A handler for when an the event processing of a message end.
    fn event_end(&mut self) {}

    /// A capture clause that can modify an incoming message.
    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }

    /// A capture clause that can modify an outgoing message.
    fn capture_outgoing(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }
}

// # Internals

pub(crate) struct PluginEntry {
    id: usize,
    priority: usize,

    typ: TypeId,
    plugin: Option<Box<dyn Plugin>>,
    state: PluginState,

    policy: PluginPanicPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PluginState {
    /// Plugin is not active, but alive, thus self.plugin contains a value.
    Idle,

    /// The plugin is currently being executed. This is only for debug purposes.
    Running,

    /// Plugin is not acitve, but alive, thus self.plugin contains a value.
    /// However it could be the case that the plugin should currently be active
    /// but is not. thus consider this plugin deactived if this state persists
    /// on the downstream path.
    JustCreated,

    /// Plugin in not active, because its dead, thus self.plugin is empty.
    Paniced,
}

impl PartialEq for PluginEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PluginEntry {}

impl PartialOrd for PluginEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PluginEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

// SAFTEY:
// Since plugin entries are stored in a cross thread context
// they must implement this traits. However plugins are not executed
// in a async context, so this does not really matter.
unsafe impl Send for PluginEntry {}
unsafe impl Sync for PluginEntry {}
