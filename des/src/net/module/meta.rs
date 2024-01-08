use std::any::Any;

/// Metadata exposed by a module.
#[derive(Debug)]
pub(super) struct Metadata {
    blobs: Vec<Box<dyn Any>>
}

impl Metadata {
    pub(super) fn new() -> Self {
        Self { blobs: Vec::new() }
    }

    /// Tries to retrieve a data object from the store.
    pub(super) fn get<T: Any>(&self) -> Option<&T> {
        self.blobs.iter().find_map(|blob| blob.downcast_ref())
    }

    pub(super) fn set<T: Any>(&mut self, value: T) {
        self.blobs.retain(|v| !v.is::<T>());
        self.blobs.push(Box::new(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_insert_and_retrive() {
        let mut meta = Metadata { blobs: Vec::new() };
        meta.set(String::from("Hello World!"));
        assert_eq!(meta.get::<String>(), Some(&String::from("Hello World!")));
        assert_eq!(meta.get::<u8>(), None);
    }
}