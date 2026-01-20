//! Statistical results collection.
use std::{
    any::{Any, TypeId, type_name},
    collections::BTreeMap,
    fs::File,
    io::{self, BufWriter},
    path::Path,
};

use serde::Serialize;

use crate::net::ObjectPath;

pub mod histogram;
pub mod std_dev;
pub mod time_series;

/// Trait for statistical results.
pub trait Statistic: Any {
    /// The result type emitted by the statistic.
    type Result: Any;
}

/// The set of all collectors for statistics.
#[derive(Debug, Default)]
pub struct Statistics {
    collectors: BTreeMap<TypeId, Box<dyn Any>>,
}

type TypedBTree<T> = BTreeMap<(ObjectPath, String), T>;

impl Statistics {
    /// Gets a mutable reference to the collector of type T with the given key.
    pub fn get_collector<T: Any + Default>(&mut self, path: ObjectPath, key: String) -> &mut T {
        self.collectors
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedBTree::<T>::default()))
            .downcast_mut::<TypedBTree<T>>()
            .expect("illegal state")
            .entry((path, key))
            .or_default()
    }

    /// Returns all collectors of type T
    pub fn collectors<T: Statistic>(
        &self,
    ) -> impl Iterator<Item = (&(ObjectPath, String), &T::Result)> + '_ {
        if let Some(btree) = self.collectors.get(&TypeId::of::<T::Result>()) {
            btree
                .downcast_ref::<TypedBTree<T::Result>>()
                .expect("illegal state")
                .iter()
        } else {
            std::collections::btree_map::Iter::default()
        }
    }

    /// Exports all collectors of type T to YAML files in the given directory.
    pub fn export_collectors<T>(&mut self, dir: &Path) -> Result<(), io::Error>
    where
        T: Statistic,
        T::Result: Serialize,
    {
        for ((path, key), collector) in self.collectors::<T>() {
            let mut file_path = dir.to_path_buf();
            let typ = type_name::<T>().replace("::", "-");
            file_path.push(format!("{path}-{key}.{typ}.yml",));

            let file = BufWriter::new(File::create(file_path)?);
            serde_norway::to_writer(file, collector).map_err(|e| io::Error::other(e))?;
        }
        Ok(())
    }

    /// Exports all well-known collectors to YAML files in the given directory.
    pub fn export_well_known(&mut self, dir: &Path) -> Result<(), io::Error> {
        self.export_collectors::<time_series::TimeSeries>(dir)
    }
}
