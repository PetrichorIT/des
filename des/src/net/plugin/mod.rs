//! Module Plugins

cfg_async! {
    mod net;
    mod time;

    pub use net::TokioNetPlugin;
    pub use time::TokioTimePlugin;
}

use crate::net::message::Message;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use super::module::{with_mod_ctx, ModuleContext};

/// A module-specific plugin
pub trait Plugin {
    /// A message preprocessor which i called whenever a message
    /// arrives.
    ///
    /// This function may receive a message unless the message was never
    /// present or was consumed by a plugin with a higher priority.
    /// The function may return a message to pre processed by downstream
    /// modules and finally `handle_message`.
    ///
    /// Note this function is also called as part of the `at_sim_start`
    /// and `at_sim_end` cycles (called with None).
    ///
    fn capture(&mut self, msg: Option<Message>) -> Option<Message>;

    /// A deferred function, executed after the event processing has finished.
    ///
    /// This method is called once `handle_message` has finished in reverse priority
    /// order. It has no arguments because any message was at least consumed by `handle_message`
    ///
    /// Use this function to perform cleanup duties.
    ///
    fn defer(&mut self);
}

/// A handle to a plugin that allows the lifetime managment
/// of plugins.
#[derive(PartialEq, Eq, Hash)]
pub struct PluginHandle {
    id: usize,

    #[cfg(debug_assertions)]
    plugin_info: String,
}

impl Debug for PluginHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(debug_assertions)]
        return write!(f, "Hook #{} {{ {} }}", self.id, self.plugin_info);

        #[cfg(not(debug_assertions))]
        return write!(f, "Hook #{}", self.id);
    }
}

///
/// Creates a new module-specific plugin.
///
/// A plugin acts as a message preprocessor and postprocessor which handles messages before
/// they reach the `handle_message` function. In this preprocessing
/// the message can be:
///
/// - consumed, leading to no activitation of `handle_message`
/// - changed, still leading to an activiation of `handle_message` (unless another plugin consumes the message)
/// - remain untouched
/// - created, if a message was consume by a plugin with higher priority
///
/// Pugins are attached to the current module and only process message
/// bound for this module. Set a priority to define the order
/// in which the plugins are evaluated. Lower values equate to
/// a higher priority.
///
/// Plugins are usually created in the `at_sim_start` method of either the [`Module`]
/// or the [`AsyncModule`] trait. This is the case, because multiple calls of [`add_plugin`]
/// result in multiple distinct plugins being created. If a module is shutdown and restarted,
/// all plugins are removed at shutdown and can be created anew when `at_sim_start` is
/// called following the modules restart. Note that some basic plugins can also be created in the
/// module setup function (see [`set_setup_fn`]).
///
/// # Common patterns
///
/// There are usually three ways to build a plugin:
///
/// 1) A **Overserver** plugin only reads the message stream provided and returns the
///    input as output. Such a plugin can be used to collect link data, or for logging purposes.
///
/// 2) A ***Capture* plugin may consume the message if it determines that the message was
///    directed at this plugin. This results in a return of None, so finally no direct excution of
///    `handle_message`. Such plugins can be used ito implement e.g. Routing, so that only packets
///    directed at the current module are pushed towards `handle_message`, and other messages are rerouted
///    to other moduels.
///
/// 3) A ***Emitter** plugin is allways becomes active event if the input is None.
///    It can be used to set the enviroment or to handle any message-independent task
///    as part of the message processing. A example would be a Timing Plugin which sets
///    a timer when capture is called and ends this timer once defer is called, to
///    messasure the processing time of all downstream plugins and especially
///    `handle_message`.
///
pub fn add_plugin(plugin: impl Plugin + 'static, priority: usize) -> PluginHandle {
    with_mod_ctx(|ctx| ctx.add_plugin(plugin, priority))
}

///
/// Destroys the plugins described by this handle.
///
/// # Panics
///
/// This function panics, if the plugin was not defined in the context of this module.
/// Also panics if executed outside of a module context.
///
pub fn remove_plugin(plugin: PluginHandle) {
    with_mod_ctx(|ctx| ctx.remove_plugin(plugin));
}

thread_local! { static PLUGIN_ID: AtomicUsize = const { AtomicUsize::new(0) } }

impl ModuleContext {
    /// Refer to [`add_plugin`].
    pub fn add_plugin<T: Plugin + 'static>(&self, plugin: T, priority: usize) -> PluginHandle {
        let id = PLUGIN_ID.with(|c| c.fetch_add(1, Ordering::SeqCst));
        let entry = PluginEntry {
            id,
            plugin: Box::new(plugin),
            priority,
        };

        let mut plugins = self.plugins.borrow_mut();
        match plugins.binary_search(&entry) {
            Ok(at) | Err(at) => plugins.insert(at, entry),
        };

        PluginHandle {
            id,
            #[cfg(debug_assertions)]
            plugin_info: format!("{} @ {}", std::any::type_name::<T>(), self.path.path()),
        }
    }

    /// Refer to [`remove_plugin`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn remove_plugin(&self, handle: PluginHandle) {
        let mut plugins = self.plugins.borrow_mut();
        if let Some((idx, _)) = plugins.iter().enumerate().find(|(_, e)| e.id == handle.id) {
            plugins.remove(idx);
        } else {
            panic!("Hook with id #{} not found on this module", handle.id);
        }
    }
}

// # INTERNALS

pub(crate) struct PluginEntry {
    pub(crate) id: usize,
    pub(crate) plugin: Box<dyn Plugin>,
    pub(crate) priority: usize,
}

impl Deref for PluginEntry {
    type Target = Box<dyn Plugin>;
    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl DerefMut for PluginEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.plugin
    }
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

impl Debug for PluginEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookEntry").finish()
    }
}
