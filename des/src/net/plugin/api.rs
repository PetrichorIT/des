use std::{any::TypeId, fmt};
use crate::net::module::{with_mod_ctx, ModuleContext, module_id, ModuleId};
use super::{Plugin, PluginStatus, PluginPanicPolicy, registry::{PluginEntry, PluginState}};

/// Adds a plugin with the default policy.
/// 
/// See [`add_plugin_with`] for more information.
pub fn add_plugin<T: Plugin>(plugin: T, priority: usize) -> PluginHandle  {
    add_plugin_with(plugin, priority, PluginPanicPolicy::default())
}

/// Attaches a plugin to the current plugin, 
/// using the provided priority and panic policy.
/// 
/// The plugin will be added into the plugin registry however it will only
/// become active in the next event. Its position in the plugin queue
/// is dependent on the provided priority. 
/// 
/// Note that this API should be used for user-defined plugins
/// and not plugins that implement base functionalies associated
/// to either the simulation, or the used framework itself.
/// Accordingly priorites provided by this API are shifted by two bits
/// so that framwork-level plugins may use other the reserved bits 
/// to achive lower priorites that user-level plugins.
/// 
/// The order of plugins with the same priority is not 
/// guranteed, but deterministic.
/// 
/// The provided panic policy is used to act upon an panic that occured within a plugins execution.
/// All policies will discard the current plugin-state, but further consequences differ.
/// 
/// # Examples
/// 
/// ```rust
/// # use des::net::plugin::*;
/// # use des::prelude::*;
/// struct OpinionatedPlugin {
///     trigger: MessageKind,
/// }
/// impl Plugin for OpinionatedPlugin {
///     fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
///         if msg.header().kind == self.trigger {
///             panic!("I dont like this number ... I quit")
///         }
///         Some(msg)
///     }
/// }
/// 
/// # struct M;
/// # impl Module for M {
/// #    fn new() -> Self { Self }
/// // ...
/// fn at_sim_start(&mut self, _: usize) {
///     let handle = add_plugin_with(
///         OpinionatedPlugin { trigger: 12 },
///         1,
///         PluginPanicPolicy::Abort,
///     );
///    assert_eq!(handle.status(), PluginStatus::StartingUp);
/// 
///    // That should be fine
///    schedule_in(Message::new().id(18).build(), Duration::from_secs(10));
/// }
/// // ...
/// # }
/// #
/// ```
pub fn add_plugin_with<T: Plugin>(plugin: T, priority: usize, policy: PluginPanicPolicy) -> PluginHandle {
    let priority = (priority << 2) | 0b011;
    with_mod_ctx(|ctx| ctx.add_plugin(plugin, priority, policy))
}

/// Runs the provided clousure on the module state retuned by 
/// [`Plugin::state`] if a plugin of type 'P' was found.
/// 
/// Returns 'None' otherwise.
pub fn get_plugin_state<P: Plugin, S: 'static>() -> Option<S> {
    with_mod_ctx(|ctx| ctx.get_plugin_state::<P, S>())
}


/// A handle to a plugin on the current module.
pub struct PluginHandle {
    id: usize,
    mod_id: ModuleId,

    #[cfg(debug_assertions)]
    plugin_info: String,
}

impl PluginHandle {
    /// Indicates that status of the plugin.
    /// 
    /// # Panics 
    /// 
    /// This function panics if the plugin describes by this handle does not 
    /// belong to the current module.
    #[must_use]
    pub fn status(&self) -> PluginStatus {
        assert_eq!(
            self.mod_id,
             module_id(), 
             "Cannot share plugin handles between modules, handles are module specific (handle for {}, mod is {})", 
             self.mod_id, 
             module_id()
        );
        with_mod_ctx(|ctx| ctx.plugin_status(self))
    }

    /// Removes this plugin from the module.
    /// 
    /// # Panics 
    /// 
    /// This function panics if the plugin describes by this handle does not 
    /// belong to the current module.
    pub fn remove(self) {
        assert_eq!(
            self.mod_id,
             module_id(), 
             "Cannot share plugin handles between modules, handles are module specific (handle for {}, mod is {})", 
             self.mod_id, 
             module_id()
        );
        with_mod_ctx(|ctx| ctx.remove_plugin(self));
    }
}

impl fmt::Debug for PluginHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(not(debug_assertions))]
        return f
            .debug_struct("PluginHandle")
            .field("id", &self.id)
            .finish();

        #[cfg(debug_assertions)]
        return f
            .debug_struct("PluginHandle")
            .field("id", &self.id)
            .field("info", &self.plugin_info)
            .finish();
    }
}

impl ModuleContext {
    /// Refer to [`add_plugin`].
    pub fn add_plugin<T: Plugin + 'static>(
        &self,
        plugin: T,
        priority: usize,
        policy: PluginPanicPolicy,
    ) -> PluginHandle {
        let entry = PluginEntry {
            id: 0,
            gen: 0,
            core: Some(Box::new(plugin)),
            state: PluginState::JustCreated,
            typ: TypeId::of::<T>(),
            priority,
            policy,
        };

        let id = self.plugins
            .try_write()
            .expect("Failed to fetch write lock: add_plugin")
            .add(entry);

        PluginHandle {
            id,
            mod_id: self.id,
            #[cfg(debug_assertions)]
            plugin_info: format!("{} @ {}", std::any::type_name::<T>(), self.path.as_str()),
        }
    }

    /// Refer to [`PluginHandle::remove`].
    ///
    /// # Panics
    ///
    /// This function panics, if the handle in invalid.
    #[allow(clippy::needless_pass_by_value)]
    pub fn remove_plugin(&self, handle: PluginHandle) {
        self.plugins
            .try_write()
            .expect("Failed to fetch write lock: remove_plugin")
            .remove(handle.id);
    }

    /// Refer to [`PluginHandle::status`]
    ///
    /// # Panics
    ///
    /// This function panics, if the handle in invalid.
    pub fn plugin_status(&self, handle: &PluginHandle) -> PluginStatus {
        self.plugins
            .try_write()
            .expect("Failed to fetch write lock: remove_plugin")
            .status(handle.id)
    }

    /// Returns the plugin state mutably.
    pub fn get_plugin_state<P: Plugin, S: 'static>(&self) -> Option<S> {
        match self.plugins
            .try_read()
            .expect("failed to fetch read lock: get_plugin_state<T>")
            .iter()
            .find(|p| p.typ == TypeId::of::<P>())?
            .core
            .as_ref()
            .unwrap()
            .state()
            .downcast::<S>() 
        {
            Ok(v) => Some(*v),
            Err(_) => None
        }
    }
}
