use std::slice;
use tracing::{
    field::Visit,
    span::{Attributes, Record},
    Level,
};

/// A set of key-value pairs associated with a span
#[derive(Debug)]
pub struct SpanFields {
    set: Vec<(&'static str, String)>,
}

impl SpanFields {
    fn new() -> Self {
        Self { set: Vec::new() }
    }

    pub(crate) fn record(&mut self, rc: &Record) {
        let mut vis = Vis(self);
        rc.record(&mut vis);
    }

    fn insert(&mut self, key: &'static str, value: String) {
        if let Some(entry) = self.set.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        } else {
            self.set.push((key, value));
        }
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get<'a>(&'a self, key: &str) -> Option<&'a String> {
        self.set
            .iter()
            .find_map(|(k, v)| if *k == key { Some(v) } else { None })
    }
}

impl<'a> IntoIterator for &'a SpanFields {
    type Item = &'a (&'static str, String);
    type IntoIter = slice::Iter<'a, (&'static str, String)>;
    fn into_iter(self) -> Self::IntoIter {
        self.set.iter()
    }
}

/// Input to a formatter to work with spans.
#[derive(Debug)]
pub struct SpanInfo {
    /// The fields
    pub fields: SpanFields,
    /// The span name
    pub name: &'static str,
    /// The level
    pub level: Level,
    pub(super) sc: usize,
}

impl SpanInfo {
    pub(super) fn from_attrs(attr: &Attributes) -> Self {
        let mut fields = SpanFields::new();

        let mut vis = Vis(&mut fields);
        attr.record(&mut vis);

        Self {
            name: attr.metadata().name(),
            fields,
            level: *attr.metadata().level(),
            sc: 1,
        }
    }
}

struct Vis<'a>(&'a mut SpanFields);
impl Visit for Vis<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name(), format!("{value:?}"));
    }
}
