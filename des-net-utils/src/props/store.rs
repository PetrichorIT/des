use std::{
    any::{type_name, Any, TypeId},
    io::{Error, ErrorKind},
    marker::PhantomData,
    sync::Arc,
};

use fxhash::FxHashMap;
use serde::de::DeserializeOwned;
use serde_yml::{from_value, Value};

use crate::sync::Mutex;

use super::Prop;

/// The properties associated with a component.
#[derive(Debug, Default)]
pub struct Props {
    mapping: FxHashMap<String, Entry>,
}

#[derive(Debug)]
pub(super) enum Entry {
    Yaml(Value),
    Value(Arc<Mutex<Box<dyn Any>>>),
}

impl Props {
    /// Sets a YAML value for a property. This will be used as the preinitialized
    /// value and will be decoded once the property is accessed.
    pub fn set(&mut self, key: String, val: Value) {
        self.mapping.insert(key, Entry::Yaml(val));
    }

    /// The keys of all properties.
    pub fn keys(&self) -> Vec<String> {
        self.mapping.keys().cloned().collect()
    }

    /// Retrieves a property value via a `Prop` handle.
    pub fn get<T>(&mut self, key: &str) -> Result<Prop<T>, Error>
    where
        T: Any,
        T: DeserializeOwned,
        T: Default,
    {
        let entry = self
            .mapping
            .entry(key.to_string())
            .or_insert_with(|| Entry::Value(Arc::new(Mutex::new(Box::new(T::default())))));

        match &*entry {
            Entry::Value(slot) => {
                let lock = slot.lock();
                if (*lock).is::<T>() {
                    Ok(Prop {
                        slot: slot.clone(),
                        _phantom: PhantomData,
                    })
                } else {
                    Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "typeid missmatch {:?} != {:?} {}",
                            slot.type_id(),
                            TypeId::of::<T>(),
                            type_name::<T>()
                        ),
                    ))
                }
            }
            Entry::Yaml(str) => {
                let parsed =
                    from_value::<T>(str.clone()).map_err(|e| Error::new(ErrorKind::Other, e))?;
                let slot: Arc<Mutex<Box<dyn Any>>> = Arc::new(Mutex::new(Box::new(parsed)));
                *entry = Entry::Value(slot.clone());

                Ok(Prop {
                    slot,
                    _phantom: PhantomData,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yml::Number;

    #[test]
    fn get_yaml_number() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("u8".to_string(), Value::Number(Number::from(32u8)));
        assert_eq!(props.get::<u8>("u8")?.get(), 32);

        props.set(
            "u8_but_usize".to_string(),
            Value::Number(Number::from(32u8)),
        );
        assert_eq!(props.get::<usize>("u8_but_usize")?.get(), 32);

        props.set(
            "u8_but_isize".to_string(),
            Value::Number(Number::from(32u8)),
        );
        assert_eq!(props.get::<isize>("u8_but_isize")?.get(), 32);

        Ok(())
    }

    #[test]
    fn get_yaml_string() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("string".to_string(), Value::String("hello".to_string()));
        assert_eq!(props.get::<String>("string")?.get(), "hello");

        Ok(())
    }

    #[test]
    fn get_yaml_bool() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("bool".to_string(), Value::Bool(true));
        assert_eq!(props.get::<bool>("bool")?.get(), true);

        Ok(())
    }

    #[test]
    fn get_yaml_failure() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("string".to_string(), Value::String("hello".to_string()));
        assert!(props.get::<u8>("string").is_err());

        // value remains unchanged
        assert_eq!(props.get::<String>("string")?.get(), "hello");

        Ok(())
    }

    #[test]
    fn get_default_no_yaml() -> Result<(), Error> {
        let mut props = Props::default();
        assert_eq!(props.get::<String>("string")?.get(), "");

        Ok(())
    }
}
