//! Module properties

use crate::sync::Mutex;
use std::{any::Any, marker::PhantomData, sync::Arc};

mod store;
mod yaml;

pub use store::*;
pub use yaml::*;

/// A typed property associated with a module.
///
/// Properties are used to store external data published by a module.
/// All properties are typed, however the type is defined at runtime, with the first
/// creation of the property.
///
/// Since properties are globally shared, they use a `Cell`-like API to prevent undue sharing
/// of global values. Their initial value can be defined by configuration files.
#[derive(Debug)]
pub struct Prop<T> {
    slot: Arc<Mutex<Box<dyn Any>>>,
    _phantom: PhantomData<T>,
}

impl<T: Any> Prop<T> {
    /// Retrieves the value of the property, by cloning it.
    ///
    /// This method returns an owned value of the globally shared property, that can be freely used
    /// and modified. Changes to the returned value will not affect the stored property. Use
    /// `set` or `update` to update the stored property.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.map(|value| value.clone())
    }

    /// Executes a closure on the value of the property.
    ///
    /// This method can be used to perform operations on the property's value without modifying or cloning it.
    /// Note that any returend value `R` must not reference the global property
    pub fn map<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
        T: Clone,
    {
        let slot = self.slot.lock();
        f(slot.downcast_ref().expect("unreachable"))
    }

    /// Sets the value of a property.
    pub fn set(&mut self, value: T) {
        let mut slot = self.slot.lock();
        *slot = Box::new(value);
    }

    /// Executes a closure with mutable access to the property's value.
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut slot = self.slot.lock();
        f(slot.downcast_mut().expect("unreachable"))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;

    use super::*;

    #[test]
    fn update() -> Result<(), Error> {
        let mut props = Props::default();
        let mut prop = props.get::<Vec<usize>>("list")?;

        prop.update(|l| l.push(1));
        assert_eq!(prop.get(), [1]);

        prop.update(|l| l.push(2));
        assert_eq!(prop.get(), [1, 2]);
        Ok(())
    }

    #[test]
    fn prop() {
        let mut props = Props::default();
        let mut list = props.get::<Vec<String>>("addrs").unwrap();

        assert_eq!(list.get(), Vec::<String>::new());

        list.set(Vec::new());
        list.update(|v| v.push("127.0.0.1".to_string()));
        list.update(|v| v.push("192.168.0.1".to_string()));

        assert_eq!(
            list.get(),
            vec!["127.0.0.1".to_string(), "192.168.0.1".to_string()]
        );

        drop(list);

        let list = props.get::<Vec<String>>("addrs").unwrap();
        assert_eq!(
            list.get(),
            vec!["127.0.0.1".to_string(), "192.168.0.1".to_string()]
        );
    }
}
