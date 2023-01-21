//! Plugin v2
use std::any::TypeId;
use std::panic::catch_unwind;

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

use super::module::with_mod_ctx;

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
    core: Option<Box<dyn Plugin>>,
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

/// Call this function when a message is send (via send / schedule_*)
/// to process the plugin output stream accordingly.
///
/// # Notes
///
/// Can be called from other plugins, also from plugin upstream or downstream
///
pub(crate) fn plugin_output_stream(msg: Message) -> Option<Message> {
    log::trace!("plugin capture of msg");
    with_mod_ctx(|ctx| {
        // (0) Create new downstream
        let mut plugins = ctx.plugins2.write();
        let prev = plugins.begin_downstream();

        // (1) Move the message allong the downstream, only using active plugins.
        //
        // 3 cases:
        // - call origin is in upstream-plugin (good since all plugins below are ::running with a core stored)
        // - call origin is main (good since all plugins are ::running with a core stored)
        // - call origin is in downstream branch
        //      - good since all plugins below are still ::running with a core and all aboth will be ignored ::idle
        //      - self is not an issue, since without a core not in itr
        //      - BUT: begin_downstream poisoined the old downstream info.
        let mut msg = msg;
        while let Some(plugin) = plugins.next_downstream() {
            let plugin = UnwindSafeBox(plugin);

            // (2) Capture the packet
            let result = catch_unwind(move || {
                let mut plugin = plugin;
                let msg = msg;

                let ret = plugin.0.capture_outgoing(msg);
                (plugin, ret)
            });

            // (3) Continue iteration if possible, readl with panics
            match result {
                Ok((r_plugin, r_msg)) => {
                    plugins.put_back_downstream(r_plugin.0, false);
                    if let Some(r_msg) = r_msg {
                        msg = r_msg;
                    } else {
                        plugins.resume_downstream_from(prev);
                        return None;
                    }
                }
                Err(p) => {
                    plugins.paniced_downstream(p);
                    plugins.resume_downstream_from(prev);
                    return None;
                }
            }
        }

        plugins.resume_downstream_from(prev);
        // (4) If the message survives, good.
        Some(msg)
    })
}
