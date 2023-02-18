use std::collections::HashMap;
use std::fmt;

use crate::net::module::Module;

type ModuleCreationFn = dyn Fn() -> Box<dyn Module>;

/// A registry for all user-defined module
pub struct Registry {
    map: HashMap<String, Box<ModuleCreationFn>>,
}

impl Registry {
    /// Creates a new empty registry.
    pub fn new() -> Registry {
        Self {
            map: HashMap::new(),
        }
    }

    /// Adds a new entry to the registry.
    pub fn add(&mut self, ty: impl AsRef<str>, f: Box<ModuleCreationFn>) {
        self.map.insert(ty.as_ref().to_string(), f);
    }

    /// Retrieges the creation fn for a modile type.
    pub fn get(&self, s: impl AsRef<str>) -> Option<&ModuleCreationFn> {
        let s = s.as_ref();
        self.map.get(s).map(|b| &**b)
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry").finish()
    }
}
