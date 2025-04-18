use std::{io::Error, sync::Arc};

use fxhash::FxHashMap;
use serde_yml::Value;

use crate::sync::Mutex;

use super::{Prop, PropType, RawProp};

/// The properties associated with a component.
#[derive(Default)]
pub struct Props {
    mapping: FxHashMap<String, Arc<Mutex<Entry>>>,
}

pub(super) enum Entry {
    None,
    Yaml(Value),
    Some(Box<dyn PropType>),
}

impl Entry {
    pub(super) fn is_some(&self) -> bool {
        match self {
            Entry::None => false,
            Entry::Yaml(_) => false,
            Entry::Some(_) => true,
        }
    }

    pub(super) fn is_none(&self) -> bool {
        match self {
            Entry::None => true,
            Entry::Yaml(_) => false,
            Entry::Some(_) => false,
        }
    }

    pub(super) fn as_option(&self) -> Option<&dyn PropType> {
        match self {
            Entry::None => None,
            Entry::Yaml(_) => None, // harder
            Entry::Some(val) => Some(&**val),
        }
    }

    pub(super) fn as_option_mut(&mut self) -> Option<&mut dyn PropType> {
        match self {
            Entry::None => None,
            Entry::Yaml(_) => None, // harder
            Entry::Some(val) => Some(&mut **val),
        }
    }
}

impl Props {
    /// Sets a YAML value for a property. This will be used as the preinitialized
    /// value and will be decoded once the property is accessed.
    pub fn set(&mut self, key: String, val: Value) {
        self.mapping
            .entry(key)
            .or_insert(Arc::new(Mutex::new(Entry::Yaml(val))));
    }

    /// The keys of all properties.
    pub fn keys(&self) -> Vec<String> {
        self.mapping.keys().cloned().collect()
    }

    pub fn get_raw(&mut self, key: &str) -> RawProp {
        let entry = self
            .mapping
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Entry::None)));

        RawProp {
            slot: entry.clone(),
        }
    }

    pub fn get<T: PropType>(&mut self, key: &str) -> Result<Prop<T, false>, Error> {
        self.get_raw(key).typed::<T>()
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
        assert_eq!(props.get::<bool>("bool")?.or_default().get(), true);

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
