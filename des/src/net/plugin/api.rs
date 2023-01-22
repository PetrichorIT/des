use std::{any::TypeId, fmt};
use crate::net::module::{with_mod_ctx, ModuleContext, module_id, ModuleId};
use super::{Plugin, PluginStatus, PluginEntry, PluginState, PluginPanicPolicy};

/// Add a plugin
pub fn add_plugin<T: Plugin>(plugin: T, priority: usize) -> PluginHandle  {
    add_plugin_with(plugin, priority, PluginPanicPolicy::default())
}

/// Add a plugin
pub fn add_plugin_with<T: Plugin>(plugin: T, priority: usize, policy: PluginPanicPolicy) -> PluginHandle {
    with_mod_ctx(|ctx| ctx.add_plugin(plugin, priority, policy))
}

/// A handle to a plugin.
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
            plugin_info: format!("{} @ {}", std::any::type_name::<T>(), self.path.path()),
        }
    }

    /// Refer to [`remove_plugin`].
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

    /// Refer to [`plugin_status`].
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
}
