//! Module properties

use crate::sync::Mutex;
use std::{
    any::Any,
    fmt::Debug,
    io::{Error, ErrorKind},
    marker::PhantomData,
    sync::Arc,
};

mod store;
mod yaml;

use serde::{de::DeserializeOwned, Serialize};
use serde_yml::Value;
pub use store::*;
pub use yaml::*;

/// A composite trait that needs to be implemented by all property types.
///
/// This trait is a manual combination of the three traits `Any`, `Serialize` and `Deserialize`.
/// This allows the implementation of this trait independently of the other traits, even for foreign systems,
/// evading the orphan rule.
pub trait PropType: Any {
    /// Returns a reference to the underlying `Any` trait object.
    ///
    /// This function should always be implemented as follows, and is only nessecary
    /// because of type systems limitations:
    /// ```rust
    /// # use std::any::Any;
    /// # struct A;
    /// # impl A {
    /// fn as_any(&self) -> &dyn Any {
    ///     self
    /// }
    /// # }
    /// ```
    fn as_any(&self) -> &dyn Any;

    /// Returns a reference to the underlying `Any` trait object mutably.
    ///
    /// This function should always be implemented as follows, and is only nessecary
    /// because of type systems limitations:
    /// ```rust
    /// # use std::any::Any;
    /// # struct A;
    /// # impl A {
    /// fn as_any_mut(&mut self) -> &mut dyn Any {
    ///     self
    /// }
    /// # }
    /// ```
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Reliably transforms the properties value into a `Value`.
    ///
    /// If no serialization is possible, a placeholder value should be returned.
    fn as_value(&self) -> Value;

    /// Deserialize a `Value` into concrete propet tyoe.
    ///
    /// # Errors
    ///
    /// If no deserialization is possible, an error is returned.
    fn from_value(value: Value) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<T: DeserializeOwned + Serialize + Any> PropType for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_value(&self) -> Value {
        serde_yml::from_str::<Value>(&serde_yml::to_string(&self).unwrap()).unwrap()
    }

    fn from_value(value: Value) -> Result<Self, Error>
    where
        Self: Sized,
    {
        serde_yml::from_value(value).map_err(Error::other)
    }
}

/// A handle to a property without a known type.
///
/// This handle can only be used for operations that do not require type information like:
/// - Removing a property, by moving it into the `Absent` state.
/// - Serializing a properties internal value into a YAML `Value`.
///
/// To transform a untyped handle into a typed one, use the `RawProp::is` function
/// to find the correct type and the `RawProp::typed` function to create a typed handle.
#[derive(Clone)]
pub struct RawProp {
    slot: Arc<Mutex<Entry>>,
}

impl RawProp {
    fn access<R>(&self, f: impl FnOnce(&Entry) -> R) -> R {
        let slot = self.slot.lock();
        f(&slot)
    }

    fn access_mut<R>(&mut self, f: impl FnOnce(&mut Entry) -> R) -> R {
        let mut slot = self.slot.lock();
        f(&mut slot)
    }

    /// Clears the property, moving it into the `Absent` state.
    ///
    /// This method works independently of the property's type or state.
    pub fn clear(&mut self) {
        self.access_mut(|entry| *entry = Entry::None);
    }

    /// Converts the property into a YAML `Value`.
    ///
    /// This method returns `None` if the property is in the `Absent` state.
    /// This method returns the configuration value, if the property is in the `InitalizedFromParam` state.
    #[must_use]
    pub fn as_value(&self) -> Option<Value> {
        self.access(|entry| match entry {
            Entry::None => None,
            Entry::Yaml(value) => Some(value.clone()),
            Entry::Some(value) => Some(value.as_value()),
        })
    }

    /// Checks, whether a property is able to hold a value of the given type `T`.
    ///
    /// This method only returns `false` if the property is in the `Present` state, with
    /// a value of a different type.
    #[must_use]
    pub fn is<T: PropType>(&self) -> bool {
        self.access(|entry| match entry {
            Entry::None | Entry::Yaml(_) => true,
            Entry::Some(value) => value.as_any().is::<T>(),
        })
    }

    /// Transforms the untyped property handle into a typed one.
    ///
    /// # Errors
    ///
    /// This function may fail in two cases:
    /// - The precondition `RawHandle::is::<T>()` is not met.
    /// - The property is in the `InitializedFromParam` state and the deserialization into the type `T` fails.
    pub fn typed<T: PropType>(self) -> Result<Prop<T, false>, Error> {
        if self.is::<T>() {
            let mut lock = self.slot.lock();
            if let Entry::Yaml(value) = &*lock {
                *lock = Entry::Some(Box::new(T::from_value(value.clone())?));
            }
            drop(lock);

            Ok(Prop {
                raw: self,
                _phantom: PhantomData,
            })
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "type missmatch"))
        }
    }
}

/// A typed property associated with a module.
///
/// > To access a property without knowing its type, see [`RawProp`].
///
/// > The generic parameter `PRESENT` whether the property is in the `Present` state, or in an unknown state (`Present` or `Absent`).
///
/// Properties are used to expose key-value pairs on a module. This can be used to
/// communicate data to other modules, external observers or to store configuration settings.
///
/// All properties store a value of an arbitrary type `T` which implements the `PropType` trait.
/// When accessed, the properties value can be accessed without cloning using the `Prop::map` or `Prop::update`
/// functions.  All accesses must be made in `Cell`-like API, since properties are stored in global scope,
/// protected by a mutex. Since external sources may access properties from other threads, Props are also
/// thread-safe.
///
/// # Accessing a property
///
/// Properties can be accessed using `ModuleContext::prop` or `ModuleContext::prop_raw`. Properties
/// can be in one of three states:
///
/// - `Absent` / `Uninitalized`: The property has not yet been set to any value. Calls to `ModuleContext::prop`
///   will return a property independent of the type `T`, since the property has not yet been fixed to a specific type.
/// - `Initialized`: The property holds a value of type `T`. Only calls with the correct type `T` will return a `Prop` handle.
/// - `InitializedFromParam`: The property has been initialized with a YAML value loaded from a configuration file. Upon calling
///   `ModuleContext::prop`, the property will try to parse the YAML value into a value of type `T`. If successful, the property will
///   transition to the `Initialized` state. Else no handle will be returned.
///
/// # The `PropType` trait
///
/// All properties must implement the `PropType` trait, that is a composite trait of the following three traits:
///
/// - `Any`: Since properties store arbitrary types, the `Any` trait allows downcasting if needed. This allows typed
///   access using the `Prop<T>` handles.
/// - `Serialize`: All properties must provide an implementation to serialize their values into a YAML `Value`. This
///   allows external sources to interact with the property, without concrete knowledge of the property's type.
/// - `Deserialize`: All properties must provide an implementation to deserialize their values from a YAML `Value`. This allows
///   inital values in the `InitializedFromParam` state.
pub struct Prop<T, const PRESENT: bool = false> {
    raw: RawProp,
    _phantom: PhantomData<T>,
}

// We can ensure that the case Entry::YAML does not happen in typed Prop<>

/// `Prop<T, PRESENT = false>`
///
/// The following methods are defined for a `Prop<T>` handle with an unknown internal state.
/// This means that a value of type `T` might or might not be present.
///
/// If the handle should be transitioned into the `Present` state,
/// use any of the following methods:
/// - `upgrade`: Unwraps the poperties present state into an `Option`
/// - `or_*`: Initializes the internal value if needed.
impl<T: PropType> Prop<T, false> {
    /// Retrieves the value of the property, by cloning it.
    /// This function returns an `Option<T>` based on the property's internal state.
    ///
    /// This method returns an owned value of the globally shared property, that can be freely used
    /// and modified. Changes to the returned value will not affect the stored property. Use
    /// `set` or `update` to update the stored property.
    #[must_use]
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.map(|value| value.cloned())
    }

    /// Executes a closure on the value of the property.
    /// The clousure is provided with an `Option<&T>` based on the property's internal state.
    ///
    /// This method can be used to perform operations on the property's value without modifying or cloning it.
    /// Note that any returend value `R` must not reference the global property.
    ///
    /// > Note that `Prop::update` is not provided for the non-upgraded prop handle. To mutate the internal value, upgrade the handle first,
    /// > or use `RawProp::clear` to reset the value.
    ///
    /// # Panics
    ///
    /// Panics if the prop-type has changed since the creation of
    /// the handle.
    pub fn map<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Option<&T>) -> R,
    {
        self.raw.access(|slot| {
            f(slot.as_option().map(|v| {
                v.as_any()
                    .downcast_ref()
                    .expect("prop-type has changed, this handle is invalid")
            }))
        })
    }

    /// Returns an upgraded prop handle, if the property is in the `Present` state,
    /// otherwise returns `None`.
    #[must_use]
    pub fn upgrade(self) -> Option<Prop<T, true>> {
        if self.raw.access(Entry::is_some) {
            Some(Prop {
                raw: self.raw,
                _phantom: PhantomData,
            })
        } else {
            None
        }
    }

    /// A shorthand for `present().expect(msg)`.
    ///
    /// # Panics
    ///
    /// Panics if the upgrade fails.
    #[must_use]
    pub fn expect(self, msg: &str) -> Prop<T, true> {
        self.upgrade().expect(msg)
    }

    /// Returns an upgraded prop handle, by moving the property into the `Present` state
    /// if needed, by initalizing the prop using the provided closure.
    ///
    /// If the property is already in the `Present` state, the closure is not called.
    pub fn or_else<F>(mut self, f: F) -> Prop<T, true>
    where
        F: FnOnce() -> T,
    {
        self.raw.access_mut(|v| {
            if v.is_none() {
                *v = Entry::Some(Box::new(f()));
            }
        });
        Prop {
            raw: self.raw,
            _phantom: PhantomData,
        }
    }

    /// Returns an upgraded prop handle, by moving the property into the `Present` state
    /// if needed, by initalizing the prop using the provided value.
    ///
    /// If the property is already in the `Present` state, the value is dropped.
    pub fn or(self, value: T) -> Prop<T, true> {
        self.or_else(|| value)
    }

    /// Returns an upgraded prop handle, by moving the property into the `Present` state
    /// if needed, by initalizing the prop using the a default value, defined by `T::default`.
    ///
    /// If the property is already in the `Present` state, no changes are made.
    pub fn or_default(self) -> Prop<T, true>
    where
        T: Default,
    {
        self.or_else(T::default)
    }
}

/// `Prop<T, PRESENT = true>`
///
/// The following methods are defined for a `Prop<T>` handle with an internal state of `Present`.
/// This means that a value of type `T` is guaranteed to be present, and all subsequent operations
/// maintain that state.
impl<T: PropType> Prop<T, true> {
    /// Retrieves the value of the property, by cloning it.
    ///
    /// This method returns an owned value of the globally shared property, that can be freely used
    /// and modified. Changes to the returned value will not affect the stored property. Use
    /// `set` or `update` to update the stored property.
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.map(T::clone)
    }

    /// Executes a closure on the value of the property.
    ///
    /// This method can be used to perform operations on the property's value without modifying or cloning it.
    /// Note that any returend value `R` must not reference the global property.
    ///
    /// # Panics
    ///
    /// Panics if the properties type has changed, since the creation
    /// of the handle.
    pub fn map<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.raw.access(|slot| {
            f(slot
                .as_option()
                .expect("unreachable")
                .as_any()
                .downcast_ref()
                .expect("prop-type has changed, handle is invalid"))
        })
    }

    /// Executes a closure with mutable access on the value of the property.
    ///
    /// This method can be used to perform operations on the property's value without modifying or cloning it.
    /// Note that any returend value `R` must not reference the global property.
    ///
    /// # Panics
    ///
    /// Panics if the properties type has changed, since the creation
    /// of the handle.
    pub fn update<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.raw.access_mut(|slot| {
            Some(f(slot
                .as_option_mut()
                .expect("unreachable")
                .as_any_mut()
                .downcast_mut()
                .expect("prop-type has changed, handle is invalid")))
        })
    }
}

/// `Prop<T>`
///
/// The following functions a implemented indepenent of the handles upgrade-status,
/// aka the properties internal state.
impl<T: PropType, const PRESENT: bool> Prop<T, PRESENT> {
    /// Sets the value of a property.
    ///
    /// # Panics
    ///
    /// Panics if the properties type has changed, since the creation
    /// of the handle.
    pub fn set(&mut self, value: T) {
        self.raw.access_mut(|slot| {
            assert!(
                slot.as_option()
                    .is_none_or(|prev_value| (*prev_value).as_any().is::<T>()),
                "cannot use this prop, since other instance has changed the type"
            );
            *slot = Entry::Some(Box::new(value));
        });
    }

    /// See [`RawProp::as_value`],
    #[must_use]
    pub fn as_value(&self) -> Option<Value> {
        self.raw.as_value()
    }

    /// See [`RawProp::clear`].
    ///
    /// This method consumes that current handle, since the typing guarantee will be lost if the property is cleared.
    pub fn clear(mut self) {
        self.raw.clear();
    }
}

impl<T: PropType + Debug> Debug for Prop<T, false> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map(|value| f.debug_struct("Prop").field("value", &value).finish())
    }
}

impl<T: PropType + Debug> Debug for Prop<T, true> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map(|value| f.debug_struct("Prop").field("value", &value).finish())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;

    use super::*;

    #[test]
    fn update() -> Result<(), Error> {
        let mut props = Props::default();
        let mut prop = props.get::<Vec<usize>>("list")?.or_default();

        prop.update(|l| l.push(1));
        assert_eq!(prop.get(), vec![1]);

        prop.update(|l| l.push(2));
        assert_eq!(prop.get(), vec![1, 2]);
        Ok(())
    }

    #[test]
    fn prop() {
        let mut props = Props::default();
        let mut list = props.get::<Vec<String>>("addrs").unwrap().or_default();

        assert_eq!(list.get(), Vec::<String>::new());

        list.set(Vec::new());
        list.update(|v| v.push("127.0.0.1".to_string()));
        list.update(|v| v.push("192.168.0.1".to_string()));

        assert_eq!(
            list.get(),
            vec!["127.0.0.1".to_string(), "192.168.0.1".to_string()]
        );

        drop(list);

        let list = props.get::<Vec<String>>("addrs").unwrap().or_default();
        assert_eq!(
            list.get(),
            vec!["127.0.0.1".to_string(), "192.168.0.1".to_string()]
        );
    }
}
