use std::{any::Any, sync::Arc};

use fxhash::FxHashMap;

use crate::sync::Mutex;

/// A global storage for externally oberservable value
#[derive(Debug, Clone)]
pub struct Watcher {
    context: String,
    map: Arc<WatcherValueMap>,
}

#[derive(Debug)]
pub(crate) struct WatcherValueMap {
    values: Mutex<FxHashMap<String, Box<dyn Any>>>,
}

impl Watcher {
    /// Writes a value to the global store, overriding the previous value on this key if existent
    pub fn write<T: Any>(&self, key: &str, value: T) {
        let mut map = self.map.values.lock();
        map.insert(format!("{}#{}", self.context, key), Box::new(value));
    }

    /// Reads a value using a clousure
    pub fn read_and<T: Any, R>(&self, key: &str, f: impl FnOnce(&T) -> R) -> Option<R> {
        let map = self.map.values.lock();
        let value = map.get(&format!("{}#{}", self.context, key))?;
        Some(f(value.downcast_ref()?))
    }

    /// Reads and copies a value
    #[must_use]
    pub fn read_clone<T: Any + Clone>(&self, key: &str) -> Option<T> {
        self.read_and::<T, T>(key, Clone::clone)
    }
}

impl WatcherValueMap {
    pub(crate) fn watcher_for(self: Arc<Self>, context: String) -> Watcher {
        Watcher { context, map: self }
    }
}

impl Default for WatcherValueMap {
    fn default() -> Self {
        WatcherValueMap {
            values: Mutex::new(FxHashMap::default()),
        }
    }
}
