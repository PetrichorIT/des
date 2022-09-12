//!
//! A set of structs that help with the collection of statistical data.
//!
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use crate::time::SimTime;
use std::ops::RangeInclusive;

cfg_metrics! {
    mod runtime;
    pub use runtime::*;
}

mod stddev;
pub use stddev::*;

mod outvec;
pub use outvec::*;

mod timeline;
pub use timeline::*;

mod mean;
pub use mean::*;

///
/// A type that allows for statistical datacollection
/// inside a given runtime.
///
pub trait Statistic {
    ///
    /// The type of values that should be collected by
    /// this statistic.
    ///
    type Value;

    /// # Data colletion methods.

    ///
    /// Collects  a datapoint at a given time with a given weight.
    /// This function is required since it is the core of the data collection.
    ///
    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, sim_time: SimTime);

    ///
    /// Collects a weighted datapoint at the current simulation time.
    ///
    fn collect_weighted(&mut self, value: Self::Value, weight: f64) {
        self.collect_weighted_at(value, weight, SimTime::now());
    }

    ///
    /// Collects a non-weighted (w=1) datapoint at a given time.
    ///
    fn collect_at(&mut self, value: Self::Value, sim_time: SimTime) {
        self.collect_weighted_at(value, 1.0, sim_time);
    }

    ///
    /// Collects a non.weighted datapoint at the current time.
    ///
    fn collect(&mut self, value: Self::Value) {
        self.collect_weighted_at(value, 1.0, SimTime::now());
    }

    /// Indicates whether the statistical object has received any datapoints.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of datapoints used in the statistical object.
    fn len(&self) -> usize;

    /// Returns the sum of all datapoints.
    fn sum(&self) -> Self::Value;

    /// Returns the squared sum of all datapoints.
    fn sqrtsum(&self) -> Self::Value;

    /// Returns the smalles datapoint.
    fn min(&self) -> Self::Value;

    /// Returns the biggest datapoint.
    fn max(&self) -> Self::Value;

    /// Retuns the mean of all datapoints.
    fn mean(&self) -> Self::Value;

    /// Returns the standard derivation.
    fn std_derivation(&self) -> Self::Value;

    /// Returns the variance of all datapoints.
    fn variance(&self) -> Self::Value;
}

///
/// A statistical metric that can be represented via a timeless
/// histogramm.
///
#[derive(Debug)]
pub struct Histogramm {
    lower_bound: f64,
    upper_bound: f64,
    interval: f64,

    min: f64,
    max: f64,

    bins: Vec<f64>,
}

impl Histogramm {
    ///
    /// Creates a new historgramm with equidistant bins.
    ///
    #[allow(unused)]
    #[must_use]
    pub fn new(range: RangeInclusive<f64>, bins: usize) -> Self {
        Self {
            lower_bound: *range.start(),
            upper_bound: *range.end(),
            interval: range.end() - range.start(),

            min: f64::INFINITY,
            max: f64::NEG_INFINITY,

            bins: vec![0.0; bins],
        }
    }
}

impl Statistic for Histogramm {
    type Value = f64;

    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, _sim_time: SimTime) {
        assert!(value <= self.upper_bound && value >= self.lower_bound);

        let rel = (value - self.lower_bound) / self.interval;
        let idx = (rel * (self.bins.len() as f64)).floor() as usize;

        self.bins[idx] += weight;
    }

    fn len(&self) -> usize {
        self.bins.iter().fold(0.0, |acc, &e| acc + e) as usize
    }

    fn sum(&self) -> Self::Value {
        let mut sum_value = 0.0;

        for b in &self.bins {
            let b = *b as f64;
            let rel = b / self.bins.len() as f64;
            let pow = self.lower_bound + rel * self.interval;

            sum_value += b * pow;
        }

        sum_value
    }

    fn sqrtsum(&self) -> Self::Value {
        let mut sum_value = 0.0;

        for b in &self.bins {
            let b = *b as f64;
            let rel = b / self.bins.len() as f64;
            let pow = self.lower_bound + rel * self.interval;

            sum_value += b * pow * pow;
        }

        sum_value
    }

    fn min(&self) -> Self::Value {
        self.min
    }

    fn max(&self) -> Self::Value {
        self.max
    }

    fn mean(&self) -> Self::Value {
        self.sum() / self.len() as f64
    }

    fn std_derivation(&self) -> Self::Value {
        todo!()
    }

    fn variance(&self) -> Self::Value {
        todo!()
    }
}
