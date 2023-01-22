//! Plugin v2
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

mod common;
pub use self::common::*;

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

/// Call this function when a message is send (via send / schedule_*)
/// to process the plugin output stream accordingly.
///
/// # Notes
///
/// Can be called from other plugins, also from plugin upstream or downstream
///
pub(crate) fn plugin_output_stream(msg: Message) -> Option<Message> {
    // (0) Create new downstream
    with_mod_ctx(|ctx| ctx.plugins.write().begin_sub_downstream(None));

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
    while let Some(plugin) = with_mod_ctx(|ctx| ctx.plugins.write().next_downstream()) {
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
                with_mod_ctx(|ctx| ctx.plugins.write().put_back_downstream(r_plugin.0, false));
                if let Some(r_msg) = r_msg {
                    msg = r_msg;
                } else {
                    with_mod_ctx(|ctx| ctx.plugins.write().close_sub_downstream());
                    return None;
                }
            }
            Err(p) => {
                with_mod_ctx(|ctx| {
                    let mut plugins = ctx.plugins.write();
                    plugins.paniced_downstream(p);
                    plugins.close_sub_downstream();
                });

                return None;
            }
        }
    }

    with_mod_ctx(|ctx| ctx.plugins.write().close_sub_downstream());

    // (4) If the message survives, good.
    Some(msg)
}
