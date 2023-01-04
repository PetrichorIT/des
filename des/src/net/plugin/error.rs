use std::{
    any::{type_name, Any, TypeId},
    error::Error,
    fmt::{Debug, Display},
};

use crate::net::module::with_mod_ctx;

use super::{PluginEntry, PluginState};

/// A error that occures in reponse to a plugin activity.
pub struct PluginError {
    kind: PluginErrorKind,
    internal: String,
}

impl PluginError {
    /// Returns the kind of the Error.
    pub fn kind(&self) -> PluginErrorKind {
        self.kind
    }

    /// Call this function is you have determined that
    /// a plugin of type T should have been active and should have provide some service
    /// but did not. This method will figure out what caused the plugin to
    /// not provide its service
    pub fn expected<T: Any>() -> PluginError {
        // let current_plugin = ;

        with_mod_ctx(|ctx| {
            if let Ok(plugins) = ctx.plugins.try_borrow() {
                let plugin = plugins
                    .iter()
                    .find(|plugin| plugin.typ == TypeId::of::<T>());

                if let Some(plugin) = plugin {
                    if matches!(plugin.state, PluginState::Paniced(_)) {
                        PluginError {
                            kind: PluginErrorKind::PluginPaniced,
                            internal: format!("expected plugin of type {}", type_name::<T>()),
                        }
                    } else {
                        // TODO: prio tests
                        PluginError {
                            kind: PluginErrorKind::PluginMalfunction,
                            internal: format!("expected plugin of type {}", type_name::<T>()),
                        }
                    }
                } else {
                    PluginError {
                        kind: PluginErrorKind::PluginNotFound,
                        internal: format!("expected plugin of type {}", type_name::<T>()),
                    }
                }
            } else {
                let plugins = ctx.plugins.as_ptr() as *const Vec<PluginEntry>;

                let plugin = unsafe { &*plugins }
                    .iter()
                    .find(|plugin| plugin.typ == TypeId::of::<T>());

                if let Some(plugin) = plugin {
                    if matches!(plugin.state, PluginState::Paniced(_)) {
                        PluginError {
                            kind: PluginErrorKind::PluginPaniced,
                            internal: format!("expected plugin of type {}", type_name::<T>()),
                        }
                    } else {
                        PluginError {
                            kind: PluginErrorKind::PluginMalfunction,
                            internal: format!("expected plugin of type {}", type_name::<T>()),
                        }
                    }
                } else {
                    PluginError {
                        kind: PluginErrorKind::PluginNotFound,
                        internal: format!("expected plugin of type {}", type_name::<T>()),
                    }
                }
            }
        })
    }
}

impl Debug for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -- {}", self.internal, self.kind)
    }
}

impl Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -- {}", self.internal, self.kind)
    }
}

impl Error for PluginError {}

/// The kind of plugin errors that can occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginErrorKind {
    /// This error indicates, that the existence of a plugin was expected,
    /// but no such plugin exists.
    PluginNotFound,
    /// This error indicates, that the existence of a plugin was expected,
    /// but the plugin paniced.
    PluginPaniced,
    /// This error indicates that a plugin should have been active, but was not
    /// since it has a lower priority than the origin point of the error.
    PluginWithLowerPriority,
    /// This error indicates that the existence and priority of the requested plugin
    /// are valid, but nonetheless the expected result did not occur.
    PluginMalfunction,
}

impl Display for PluginErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::PluginNotFound => "not found",
                Self::PluginPaniced => "paniced",
                Self::PluginWithLowerPriority => "lower priority not active",
                Self::PluginMalfunction => "malfunctioned",
            }
        )
    }
}
