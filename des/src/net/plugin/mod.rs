//! Module Plugins

mod error;
pub use error::*;

mod periodic;
pub use periodic::PeriodicPlugin;

cfg_async! {
    mod time;
    pub use time::TokioTimePlugin;
}

use crate::net::message::Message;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    panic::{catch_unwind, UnwindSafe},
    sync::atomic::{AtomicUsize, Ordering},
};

use super::module::{with_mod_ctx, ModuleContext};

/// A module-specific plugin
///
/// Plugins can be created using [`add_plugin`] and destroyed using [`remove_plugin`].
pub trait Plugin: UnwindSafe {
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

    /// A message pre-processor executed at the sim_start stage.
    fn capture_sim_start(&mut self) {}

    /// A message pre-processor executed at the sim_end stage.
    fn capture_sim_end(&mut self) {}

    /// A deferred function, executed after the event processing has finished.
    ///
    /// This method is called once `handle_message` has finished in reverse priority
    /// order. It has no arguments because any message was at least consumed by `handle_message`
    ///
    /// Use this function to perform cleanup duties.
    ///
    fn defer(&mut self);

    /// A message post-processor executed at the sim_start stage.
    fn defer_sim_start(&mut self) {}

    /// A message post-processor executed at the sim_end stage.
    fn defer_sim_end(&mut self) {}
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
    with_mod_ctx(|ctx| ctx.add_plugin(plugin, priority, true))
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

/// Returns the status of the plugin
///
/// # Panics
///
/// This function panics, if the plugin was ot defined in the context of this module.
/// Also panics if executed outside of a module context.
pub fn plugin_status(plugin: &PluginHandle) -> PluginStatus {
    with_mod_ctx(|ctx| ctx.plugin_status(plugin))
}

static PLUGIN_ID: AtomicUsize = AtomicUsize::new(0);

impl ModuleContext {
    /// Refer to [`add_plugin`].
    pub fn add_plugin<T: Plugin + 'static>(
        &self,
        plugin: T,
        priority: usize,
        just_created: bool,
    ) -> PluginHandle {
        let id = PLUGIN_ID.fetch_add(1, Ordering::SeqCst);
        let entry = PluginEntry {
            id,
            state: PluginState::Idle(Box::new(plugin)),
            typ: TypeId::of::<T>(),
            priority,
            just_created,
        };

        let mut plugins = self
            .plugins
            .try_write()
            .expect("cannot create new plugin while other plugin is active");
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
        let mut plugins = self
            .plugins
            .try_write()
            .expect("cannot remove plugin while other plugin is active");
        if let Some((idx, _)) = plugins.iter().enumerate().find(|(_, e)| e.id == handle.id) {
            plugins.remove(idx);
        } else {
            panic!("Plugin with id #{} not found on this module", handle.id);
        }
    }

    /// Refer to [`plugin_status`].
    pub fn plugin_status(&self, handle: &PluginHandle) -> PluginStatus {
        let plugins = self
            .plugins
            .try_read()
            .expect("cannot probe plugin while other plugin is active");
        if let Some((idx, _)) = plugins.iter().enumerate().find(|(_, e)| e.id == handle.id) {
            if matches!(plugins[idx].state, PluginState::Paniced(_)) {
                PluginStatus::Paniced
            } else {
                PluginStatus::Active
            }
        } else {
            panic!("Plugin with id #{} not found on this module", handle.id);
        }
    }
}

/// The status of plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginStatus {
    /// The plugin is still able to receive and capture data.
    Active,
    /// The plugin had a malfuction and paniced.
    Paniced,
}

// # INTERNALS

pub(crate) struct PluginEntry {
    pub(super) id: usize,
    pub(super) state: PluginState,
    pub(super) typ: TypeId,
    pub(super) priority: usize,
    pub(super) just_created: bool,
}

unsafe impl Send for PluginEntry {}
unsafe impl Sync for PluginEntry {}

pub(super) enum PluginState {
    Idle(Box<dyn Plugin>),
    Running(),
    Paniced(Box<dyn Any + Send>),
}

impl PluginState {
    fn transition_to_runnning(&mut self) -> Option<Box<dyn Plugin>> {
        let mut swap = Self::Running();
        std::mem::swap(self, &mut swap);
        if let Self::Idle(plugin) = swap {
            Some(plugin)
        } else {
            std::mem::swap(self, &mut swap);
            None
        }
    }
}

impl PluginEntry {
    pub(super) fn try_capture(&mut self, msg: Option<Message>) -> Option<Message> {
        if let Some(mut plugin) = self.state.transition_to_runnning() {
            let result = catch_unwind(|| {
                let res = plugin.capture(msg);
                (plugin, res)
            });

            match result {
                Ok((plugin, msg)) => {
                    self.state = PluginState::Idle(plugin);
                    return msg;
                }
                Err(panic) => {
                    log::error!("Plugin #{} paniced at Plugin::capture", self.id);
                    self.state = PluginState::Paniced(panic);
                    return None;
                }
            }
        } else {
            msg
        }
    }

    pub(super) fn try_capture_sim_start(&mut self) {
        if let Some(mut plugin) = self.state.transition_to_runnning() {
            let result = catch_unwind(|| {
                plugin.capture_sim_start();
                plugin
            });

            match result {
                Ok(plugin) => self.state = PluginState::Idle(plugin),
                Err(panic) => {
                    log::error!("Plugin #{} paniced at Plugin::capture_sim_start", self.id);
                    self.state = PluginState::Paniced(panic)
                }
            }
        }
    }

    pub(super) fn try_capture_sim_end(&mut self) {
        if let Some(mut plugin) = self.state.transition_to_runnning() {
            let result = catch_unwind(|| {
                plugin.capture_sim_end();
                plugin
            });

            match result {
                Ok(plugin) => self.state = PluginState::Idle(plugin),
                Err(panic) => {
                    log::error!("Plugin #{} paniced at Plugin::capture_sim_end", self.id);
                    self.state = PluginState::Paniced(panic)
                }
            }
        }
    }

    pub(super) fn try_defer(&mut self) {
        if let Some(mut plugin) = self.state.transition_to_runnning() {
            let result = catch_unwind(|| {
                plugin.defer();
                plugin
            });

            match result {
                Ok(plugin) => self.state = PluginState::Idle(plugin),
                Err(panic) => {
                    log::error!("Plugin #{} paniced at Plugin::defer", self.id);
                    self.state = PluginState::Paniced(panic)
                }
            }
        }
    }

    pub(super) fn try_defer_sim_start(&mut self) {
        if let Some(mut plugin) = self.state.transition_to_runnning() {
            let result = catch_unwind(|| {
                plugin.defer_sim_start();
                plugin
            });

            match result {
                Ok(plugin) => self.state = PluginState::Idle(plugin),
                Err(panic) => {
                    log::error!("Plugin #{} paniced at Plugin::defer_sim_start", self.id);
                    self.state = PluginState::Paniced(panic)
                }
            }
        }
    }

    pub(super) fn try_defer_sim_end(&mut self) {
        if let Some(mut plugin) = self.state.transition_to_runnning() {
            let result = catch_unwind(|| {
                plugin.defer_sim_end();
                plugin
            });

            match result {
                Ok(plugin) => self.state = PluginState::Idle(plugin),
                Err(panic) => {
                    log::error!("Plugin #{} paniced at Plugin::defer_sim_end", self.id);
                    self.state = PluginState::Paniced(panic)
                }
            }
        }
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

// # Experimental
