use std::sync::Arc;

use fxhash::FxHashMap;
use serde_norway::Value;

use crate::{net::Error, sync::Mutex};

use super::{Prop, PropType, RawProp};

/// The properties associated with a component.
#[derive(Default)]
pub(crate) struct Props {
    mapping: FxHashMap<String, Arc<Mutex<Entry>>>,
}

pub(super) struct Entry {
    pub(super) value: Option<Box<dyn PropType>>,
    pub(super) is_statistic: bool,
}

impl Entry {
    pub(super) const fn none() -> Self {
        Entry {
            value: None,
            is_statistic: false,
        }
    }

    pub(super) fn some(value: impl PropType) -> Self {
        Entry {
            value: Some(Box::new(value)),
            is_statistic: false,
        }
    }

    pub(super) fn set(&mut self, new: Box<dyn PropType>) {
        self.is_statistic |= new.is_statistic();
        self.value = Some(new);
    }

    pub(super) fn is_some(&self) -> bool {
        self.value.is_some()
    }

    pub(super) fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub(super) fn as_value(&self) -> Option<Value> {
        self.value.as_ref().map(|value| value.as_value())
    }

    pub(super) fn try_transform<T: PropType>(&mut self) {
        let value = self.value.take();
        if let Some(value) = value {
            self.value = match T::transform(value) {
                Ok(value) => {
                    let coerced: Box<dyn PropType> = value;
                    Some(coerced)
                }
                Err(unchanged) => Some(unchanged),
            };
        }
    }

    pub(super) fn as_option(&self) -> Option<&dyn PropType> {
        match &self.value {
            Some(value) => Some(&**value),
            _ => None,
        }
    }

    pub(super) fn as_option_mut(&mut self) -> Option<&mut dyn PropType> {
        match &mut self.value {
            Some(value) => Some(&mut **value),
            _ => None,
        }
    }
}

impl Props {
    /// Sets a YAML value for a property. This will be used as the preinitialized
    /// value and will be decoded once the property is accessed.
    pub(crate) fn set(&mut self, key: String, val: Value) {
        self.mapping
            .entry(key)
            .or_insert(Arc::new(Mutex::new(Entry::some(val))));
    }

    /// The keys of all properties.
    #[must_use]
    pub(crate) fn keys(&self) -> Vec<String> {
        self.mapping.keys().cloned().collect()
    }

    pub(crate) fn get_raw(&mut self, key: &str) -> RawProp {
        let entry = self
            .mapping
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Entry::none())));

        RawProp {
            slot: entry.clone(),
        }
    }

    /// # Errors
    /// Returns an error if the typing of the property fails.
    pub(crate) fn get<T: PropType>(&mut self, key: &str) -> Result<Prop<T, false>, Error> {
        self.get_raw(key).typed::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_norway::Number;

    #[test]
    fn get_yaml_number() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("u8".to_string(), Value::Number(Number::from(32u8)));
        assert_eq!(props.get::<u8>("u8")?.or_default().get(), 32);

        props.set(
            "u8_but_usize".to_string(),
            Value::Number(Number::from(32u8)),
        );
        assert_eq!(props.get::<usize>("u8_but_usize")?.or_default().get(), 32);

        props.set(
            "u8_but_isize".to_string(),
            Value::Number(Number::from(32u8)),
        );
        assert_eq!(props.get::<isize>("u8_but_isize")?.or_default().get(), 32);

        Ok(())
    }

    #[test]
    fn get_yaml_string() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("string".to_string(), Value::String("hello".to_string()));
        assert_eq!(props.get::<String>("string")?.or_default().get(), "hello");

        Ok(())
    }

    #[test]
    fn get_yaml_bool() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("bool".to_string(), Value::Bool(true));
        assert!(props.get::<bool>("bool")?.or_default().get());

        Ok(())
    }

    #[test]
    fn get_yaml_failure() -> Result<(), Error> {
        let mut props = Props::default();

        props.set("string".to_string(), Value::String("hello".to_string()));
        assert!(props.get::<u8>("string").is_err());

        // value remains unchanged
        assert_eq!(props.get::<String>("string")?.or_default().get(), "hello");

        Ok(())
    }

    #[test]
    fn get_default_no_yaml() -> Result<(), Error> {
        let mut props = Props::default();
        assert_eq!(props.get::<String>("string")?.or_default().get(), "");

        Ok(())
    }
}
