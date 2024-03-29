use fxhash::{FxBuildHasher, FxHashMap};
use std::fmt;

use crate::net::processing::ProcessingElements;

type ModuleCreationFn = dyn Fn() -> ProcessingElements;

/// A registry for all user-defined modules.
///
/// This registry is used to link Rust-Structs to Ndl-Modules.
/// Create a registry with the [`registry`](crate::registry) macro.
pub struct Registry {
    map: FxHashMap<String, Box<ModuleCreationFn>>,
}

impl Registry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Registry {
        Self {
            map: FxHashMap::with_hasher(FxBuildHasher::default()),
        }
    }

    /// Adds a new entry to the registry.
    ///
    /// The entry will bind the Ndl-Module of parameter ty to the Rust-Struct
    /// created by the creation function.
    pub fn add(&mut self, ty: impl AsRef<str>, f: Box<ModuleCreationFn>) {
        self.map.insert(ty.as_ref().to_string(), f);
    }

    /// Gets the creation function for a given Ndl-Module ty.
    ///
    /// Will return None if no such module was registered.
    pub fn get(&self, ty: impl AsRef<str>) -> Option<&ModuleCreationFn> {
        let ty = ty.as_ref();
        self.map.get(ty).map(|b| &**b)
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry").finish()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
