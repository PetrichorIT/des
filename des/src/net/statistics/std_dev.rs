//! Standard deviation

use std::f64;

use crate::{net::statistics::Statistic, prelude::current};

/// Standard deviation
#[derive(Debug, Clone, PartialEq)]
pub struct StdDev {
    name: String,
    min: f64,
    max: f64,
    sum: f64,
    sum_of_squares: f64,
    count: usize,
}

impl Default for StdDev {
    fn default() -> Self {
        StdDev {
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: 0.0,
            sum_of_squares: 0.0,
            count: 0,
        }
    }
}

#[allow(missing_docs)]
impl StdDev {
    /// Create a new standard deviation statistic with the given name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        StdDev {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Record a new value for this standard deviation statistic.
    pub fn record(&mut self, value: f64) {
        let current = current();
        let globals = current.globals();
        let mut stats = globals.statistics.lock().expect("failed lock");
        let collector = stats.get_collector::<StdDev>(current.path.clone(), self.name.clone());
        collector.record_inner(value);
    }

    fn record_inner(&mut self, value: f64) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.sum_of_squares += value * value;
        self.count += 1;
    }

    pub fn min(&self) -> f64 {
        (self.count > 0).then_some(self.min).unwrap_or(f64::NAN)
    }

    pub fn max(&self) -> f64 {
        (self.count > 0).then_some(self.max).unwrap_or(f64::NAN)
    }

    pub fn mean(&self) -> f64 {
        self.sum / self.count as f64
    }

    pub fn variance(&self) -> f64 {
        if self.count == 0 {
            f64::NAN
        } else if self.min == self.max {
            0.0
        } else {
            let variance = (self.sum_of_squares - self.sum.powi(2) / self.count as f64)
                / (self.count - 1) as f64;
            if variance < 0.0 { 0.0 } else { variance }
        }
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
}

impl Statistic for StdDev {
    type Result = Self;
}
