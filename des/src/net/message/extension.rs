use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    panic::UnwindSafe,
};

/// Extensions that may be attached to messages.
///
/// These extensions act as additional metadata attached to a message, that are not
/// counted as a phyiscal part of the message. Thus their size does not matter.
/// Upon cloning extensions will be lost.
#[derive(Default)]
pub struct Extensions {
    #[cfg(not(debug_assertions))]
    extensions: BTreeMap<TypeId, Box<dyn Any + Send>>,
    #[cfg(debug_assertions)]
    extensions: BTreeMap<TypeId, (Box<dyn Any + Send>, &'static str)>,
}

impl Extensions {
    /// Whether the extensions are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// The number of extensions attached to the message.
    #[must_use]
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Checks whether an extension of type `T` is present.
    #[must_use]
    pub fn has<T: Any + Send>(&self) -> bool {
        self.extensions.contains_key(&TypeId::of::<T>())
    }

    /// Adds an extension of type `T` to the message.
    pub fn set<T: Any + Send>(&mut self, extension: T) {
        #[cfg(not(debug_assertions))]
        self.extensions
            .insert(TypeId::of::<T>(), Box::new(extension));
        #[cfg(debug_assertions)]
        self.extensions.insert(
            TypeId::of::<T>(),
            (Box::new(extension), std::any::type_name::<T>()),
        );
    }

    /// Adds an extension of type `T` to the message if it is not already present.
    pub fn set_if_absent<T: Any + Send>(&mut self, extension: T) {
        if !self.has::<T>() {
            self.set::<T>(extension);
        }
    }

    /// Retrieves an extension of type `T` from the message.
    #[must_use]
    pub fn get<T: Any + Send>(&self) -> Option<&T> {
        self.extensions.get(&TypeId::of::<T>()).and_then(|e| {
            #[cfg(not(debug_assertions))]
            return e.downcast_ref();
            #[cfg(debug_assertions)]
            return e.0.downcast_ref();
        })
    }

    /// Retrieves a mutable reference to an extension of type `T` from the message.
    #[must_use]
    pub fn get_mut<T: Any + Send>(&mut self) -> Option<&mut T> {
        self.extensions.get_mut(&TypeId::of::<T>()).and_then(|e| {
            #[cfg(not(debug_assertions))]
            return e.downcast_mut();
            #[cfg(debug_assertions)]
            return e.0.downcast_mut();
        })
    }

    /// Removes an extension of type `T` from the message.
    #[allow(clippy::missing_panics_doc)]
    pub fn remove<T: Any + Send>(&mut self) -> Option<T> {
        self.extensions.remove(&TypeId::of::<T>()).map(|v| {
            #[cfg(not(debug_assertions))]
            return *v.downcast::<T>().expect("illegal state");
            #[cfg(debug_assertions)]
            return *v.0.downcast::<T>().expect("illegal state");
        })
    }

    /// Clears all extensions from the message.
    pub fn clear(&mut self) {
        self.extensions.clear();
    }
}

impl Debug for Extensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(not(debug_assertions))]
        return self.extensions.keys().collect::<BTreeSet<_>>().fmt(f);
        #[cfg(debug_assertions)]
        return self
            .extensions
            .values()
            .map(|v| v.1)
            .collect::<BTreeSet<_>>()
            .fmt(f);
    }
}

// SAFETY:
// Since this type does not provide any interior API that could cause a panic,
// it is unwind safe, even if the contained extensions types are not, since
// at insertion they are to be considered valid. Any operations on the values
// that may lead to an invalid state happens externally, thus will be observed by
// the compiler, introdcuing the required !UnwindSafe bound if needed
impl UnwindSafe for Extensions {}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    #[test]
    fn size_of() {
        assert!(mem::size_of::<Extensions>() < 32);
    }

    #[test]
    fn access_values() {
        let mut extensions = Extensions::default();
        extensions.set(10u32);
        extensions.set(20u64);

        assert!(extensions.has::<u32>());
        assert!(extensions.has::<u64>());
        assert!(!extensions.has::<String>());

        assert_eq!(extensions.get::<u32>().unwrap(), &10);
        assert_eq!(extensions.get::<u64>().unwrap(), &20);
    }

    #[test]
    fn delete_value() {
        let mut extensions = Extensions::default();
        extensions.set(10u32);
        extensions.set(20u64);

        assert_eq!(extensions.get::<u32>().unwrap(), &10);

        let v = extensions.remove::<u32>();
        assert_eq!(v, Some(10));

        assert!(!extensions.has::<u32>());
        assert!(extensions.has::<u64>());
        assert!(!extensions.has::<String>());

        assert_eq!(extensions.get::<u32>(), None);
    }

    #[test]
    fn mutate_value() {
        let mut extensions = Extensions::default();
        extensions.set(10u32);
        extensions.set(20u64);

        *extensions.get_mut::<u32>().unwrap() += 1;

        assert_eq!(extensions.get::<u32>().unwrap(), &11);
        assert_eq!(extensions.get::<u64>().unwrap(), &20);
    }

    #[test]
    fn set_if_absent() {
        let mut extensions = Extensions::default();
        extensions.set(10u32);
        extensions.set(20u64);

        extensions.set_if_absent(30u32);

        assert_eq!(extensions.get::<u32>().unwrap(), &10);
        assert_eq!(extensions.get::<u64>().unwrap(), &20);

        extensions.set("hello world!");

        assert_eq!(extensions.get::<&'static str>().unwrap(), &"hello world!");
    }
}
