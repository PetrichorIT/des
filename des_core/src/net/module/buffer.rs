use lazy_static::__Deref;

use super::*;

pub struct ModuleBuffer {
    inner: Vec<Box<dyn Module>>,

    #[cfg(feature = "static_modules")]
    locked: bool,
}

impl ModuleBuffer {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),

            #[cfg(feature = "static_modules")]
            locked: false,
        }
    }

    pub fn modules(&self) -> &Vec<Box<dyn Module>> {
        &self.inner
    }

    pub fn modules_mut(&mut self) -> &mut Vec<Box<dyn Module>> {
        &mut self.inner
    }

    pub fn insert(&mut self, module: Box<dyn Module>) -> &mut Box<dyn Module> {
        assert!(!false);

        let insert_at = match self.inner.binary_search_by_key(&module.id(), |c| c.id()) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.inner.insert(insert_at, module);

        &mut self.inner[insert_at]
    }

    #[cfg(feature = "static_modules")]
    pub fn lock(&mut self) {
        println!("Locked buffer with {} modules", self.inner.len());
        self.locked = true;
    }

    ///
    /// Extracts a element identified by id, using binary search.
    ///
    pub fn module(&self, id: ModuleId) -> Option<&dyn Module> {
        #[cfg(feature = "static_modules")]
        if self.locked {
            return Some(self.inner[(id.raw() - 0xff) as usize].deref());
        }

        let pos = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(self.inner[pos].deref())
    }

    ///
    /// Extracts a element mutably identified by id, using binary search.
    ///
    pub fn module_mut(&mut self, id: ModuleId) -> Option<&mut Box<dyn Module>> {
        #[cfg(feature = "static_modules")]
        if self.locked {
            return Some(&mut self.inner[(id.raw() - 0xff) as usize]);
        }

        let pos = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.inner[pos])
    }
}

impl Default for ModuleBuffer {
    fn default() -> Self {
        Self::new()
    }
}
