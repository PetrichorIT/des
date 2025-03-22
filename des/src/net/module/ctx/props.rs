//! Module properties

use fxhash::FxHashMap;

use crate::sync::Mutex;
use std::{
    any::{type_name, Any, TypeId},
    io,
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

///
pub trait PropTyp: Any + Sized {
    ///
    fn from_par(str: &str) -> Result<Self, io::Error>;
}

impl<T: PropTyp> PropTyp for Vec<T> {
    fn from_par(str: &str) -> Result<Self, io::Error> {
        str.split(',')
            .filter(|s| !s.is_empty())
            .map(T::from_par)
            .collect::<Result<Vec<T>, io::Error>>()
    }
}

macro_rules! from_str_std_err {
    ($($t:ty),*) => {
        $(
            impl PropTyp for $t {
                fn from_par(str: &str) -> Result<$t, io::Error> {
                    match <$t as std::str::FromStr>::from_str(str) {
                        Ok(v) => Ok(v),
                        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
                    }
                }
            }

        )*
    };
}

from_str_std_err!(
    String, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, IpAddr, Ipv4Addr,
    Ipv6Addr
);

///
#[derive(Debug, Default)]
pub(crate) struct Props {
    mapping: FxHashMap<String, Arc<Mutex<Entry>>>,
}

#[derive(Debug)]
enum Entry {
    Empty,
    FromString(String),
    Value(Box<dyn Any>),
}

impl Props {
    pub(crate) fn set_str(&mut self, key: String, val: String) {
        self.mapping
            .insert(key, Arc::new(Mutex::new(Entry::FromString(val))));
    }

    pub(crate) fn keys(&self) -> Vec<String> {
        self.mapping.keys().cloned().collect()
    }

    pub(crate) fn get<T>(&mut self, key: &str) -> Result<Prop<T>, io::Error>
    where
        T: Any,
        T: PropTyp,
    {
        let entry = self
            .mapping
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Entry::Empty)));

        let mut lock = entry.lock();
        match &*lock {
            Entry::Value(val) if (**val).type_id() != TypeId::of::<T>() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "typeid missmatch {:?} != {:?} {}",
                        val.type_id(),
                        TypeId::of::<T>(),
                        type_name::<T>()
                    ),
                ))
            }
            Entry::FromString(str) => {
                let parsed = T::from_par(str)?;
                *lock = Entry::Value(Box::new(parsed))
            }
            _ => {}
        }

        Ok(Prop {
            slot: entry.clone(),
            _phantom: PhantomData,
        })
    }
}

/// Property
#[derive(Debug)]
pub struct Prop<T> {
    slot: Arc<Mutex<Entry>>,
    _phantom: PhantomData<T>,
}

impl<T: Any> Prop<T> {
    ///
    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.map(|value| value.clone())
    }

    ///
    pub fn map<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
        T: Clone,
    {
        let slot = self.slot.lock();
        match &*slot {
            Entry::Empty => None,
            Entry::Value(boxed) => Some(f(boxed.downcast_ref().expect("unreachable"))),
            _ => unreachable!(),
        }
    }

    ///
    pub fn set(&mut self, value: T) {
        let mut slot = self.slot.lock();
        *slot = Entry::Value(Box::new(value));
    }

    ///
    pub fn override_type<U: Any>(self, value: U) -> Prop<U> {
        *self.slot.lock() = Entry::Value(Box::new(value));
        Prop {
            slot: self.slot,
            _phantom: PhantomData,
        }
    }

    ///
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut slot = self.slot.lock();
        match &mut *slot {
            Entry::Empty => {}
            Entry::Value(boxed) => f(boxed.downcast_mut().expect("unreachable")),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prop() {
        let mut props = Props::default();
        let mut list = props.get::<Vec<String>>("addrs").unwrap();

        assert_eq!(list.get(), None);

        list.set(Vec::new());
        list.update(|v| v.push("127.0.0.1".to_string()));
        list.update(|v| v.push("192.168.0.1".to_string()));

        assert_eq!(
            list.get(),
            Some(vec!["127.0.0.1".to_string(), "192.168.0.1".to_string()])
        );

        drop(list);

        let list = props.get::<Vec<String>>("addrs").unwrap();
        assert_eq!(
            list.get(),
            Some(vec!["127.0.0.1".to_string(), "192.168.0.1".to_string()])
        );
    }
}
