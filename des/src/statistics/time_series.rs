//! Time-series data
use serde::{Deserialize, Serialize};

use crate::{prelude::current, statistics::Statistic, time::SimTime};

/// Time series data collected as 64-bit floating point values.
///
/// This object only records a point when the y value changes.
#[derive(Debug, Clone)]
pub struct TimeSeries {
    key: String,
}

/// The result of a time series query.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TimeSeriesResult {
    /// An ordered time series of values.
    pub values: Vec<(SimTime, f64)>,
}

impl TimeSeries {
    /// Creates a new time series with the given key.
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
        }
    }

    /// Records a value in the time series.
    ///
    /// # Panics
    ///
    /// This function panics if not executed within a node context.
    #[allow(clippy::float_cmp)]
    pub fn record(&self, value: f64) {
        let current = current();
        let globals = current.globals();

        let mut stats = globals.statistics.lock().expect("failed lock");
        let collector = stats.get_collector::<TimeSeriesResult>(current.path(), self.key.clone());
        if collector.values.last().is_none_or(|last| last.1 != value) {
            collector.values.push((SimTime::now(), value));
        }
    }
}

impl Statistic for TimeSeries {
    type Result = TimeSeriesResult;
}
