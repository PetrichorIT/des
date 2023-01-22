use super::PluginState;
use crate::net::module::with_mod_ctx;
use std::{
    any::{type_name, Any, TypeId},
    error::Error,
    fmt::{Debug, Display},
};

/// A error that occures in reponse to a plugin activity.
pub struct PluginError {
    kind: PluginErrorKind,
    internal: String,
}

impl PluginError {
    /// Returns the kind of the Error.
    #[must_use]
    pub fn kind(&self) -> PluginErrorKind {
        self.kind
    }

    /// Call this function is you have determined that
    /// a plugin of type T should have been active and should have provide some service
    /// but did not. This method will figure out what caused the plugin to
    /// not provide its service
    #[must_use]
    pub fn expected<T: Any>() -> PluginError {
        // let current_plugin = ;
        let type_id = TypeId::of::<T>();

        with_mod_ctx(|ctx| {
            let plugins = ctx.plugins.try_read().expect(
                "Failed to get read loa on plugins at error creation: uncreitain code path",
            );
            let plugin = plugins.iter().find(|plugin| plugin.typ == type_id);

            if let Some(plugin) = plugin {
                match plugin.state {
                    PluginState::Idle | PluginState::JustCreated => PluginError {
                        kind: PluginErrorKind::PluginWithLowerPriority,
                        internal: format!(
                            "expected plugin of type {} was found, but not yet active due to priority",
                            type_name::<T>()
                        ),
                    },
                    PluginState::Running => PluginError {
                        kind: PluginErrorKind::PluginMalfunction,
                        internal: if plugin.core.is_none() { 
                            format!(
                                "expected plugin of type {} was found, but is self",
                                type_name::<T>()
                            )
                        } else {
                            format!(
                                "expected plugin of type {} was found, but malfunctioned",
                                type_name::<T>()
                            )
                        },
                    },
                    PluginState::Paniced => PluginError {
                        kind: PluginErrorKind::PluginPaniced,
                        internal: format!(
                            "expected plugin of type {} was found, but paniced",
                            type_name::<T>()
                        ),
                    },
                    PluginState::PendingRemoval => PluginError {
                        kind: PluginErrorKind::PluginNotFound,
                        internal: format!(
                            "expected plugin of type {}, but no such plugin exists anymore",
                            type_name::<T>()
                        ),
                    }
                }
            } else {
                PluginError {
                    kind: PluginErrorKind::PluginNotFound,
                    internal: format!(
                        "expected plugin of type {}, but no such plugin exists",
                        type_name::<T>()
                    ),
                }
            }
        })
    }
}

impl Debug for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.internal, self.kind)
    }
}

impl Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.internal, self.kind)
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
                Self::PluginNotFound => "ENOTFOUND",
                Self::PluginPaniced => "EPANICED",
                Self::PluginWithLowerPriority => "EINACTIVE",
                Self::PluginMalfunction => "EMALFUNCTION",
            }
        )
    }
}
