//! Standard deviation

use serde::Serialize;
use std::f64;

use crate::module::PropType;

/// Standard deviation
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StdDev {
    min: f64,
    max: f64,
    sum: f64,
    sum_of_squares: f64,
    count: usize,
}

impl Default for StdDev {
    fn default() -> Self {
        StdDev {
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
    /// Record a new value for this standard deviation statistic.
    ///
    /// # Panics
    ///
    /// This function panics when executed outside a node context.
    pub fn record(&mut self, value: f64) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.sum_of_squares += value * value;
        self.count += 1;
    }

    #[must_use]
    pub fn min(&self) -> f64 {
        if self.count > 0 { self.min } else { f64::NAN }
    }

    #[must_use]
    pub fn max(&self) -> f64 {
        if self.count > 0 { self.max } else { f64::NAN }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn mean(&self) -> f64 {
        self.sum / self.count as f64
    }

    #[must_use]
    #[allow(clippy::float_cmp, clippy::cast_precision_loss)]
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

    #[must_use]
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
}

impl PropType for StdDev {
    fn is_statistic(&self) -> bool {
        true
    }

    fn as_value(&self) -> serde_norway::Value {
        serde_norway::to_value(self).expect("failed encoding")
    }
}
