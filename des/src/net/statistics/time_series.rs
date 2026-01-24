//! Time-series data

use serde_norway::{Mapping, Number, Value};

use crate::{net::module::PropType, time::SimTime};

/// Time series data collected as 64-bit floating point values.
///
/// This object only records a point when the y value changes.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TimeSeries<T> {
    /// An ordered time series of values.
    pub values: Vec<(SimTime, T)>,
}

impl<T> TimeSeries<T> {
    /// Records a value in the time series.
    pub fn record(&mut self, value: T) {
        self.values.push((SimTime::now(), value));
    }
}

impl<T: PropType> PropType for TimeSeries<T> {
    fn as_value(&self) -> serde_norway::Value {
        let mut mapping = Mapping::with_capacity(self.values.len());
        for (time, value) in &self.values {
            mapping.insert(
                Value::Number(Number::from(time.as_secs_f64())),
                value.as_value(),
            );
        }
        Value::Mapping(mapping)
    }

    fn from_value(_: serde_norway::Value) -> Result<Self, crate::net::Error>
    where
        Self: Sized,
    {
        todo!()
    }

    fn is_statistic(&self) -> bool {
        true
    }
}
