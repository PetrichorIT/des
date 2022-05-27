//!
//! A set of structs that help with the collection of statistical data.
//!

use crate::time::SimTime;
use std::ops::RangeInclusive;

#[cfg(feature = "internal-metrics")]
mod internal;
#[cfg(feature = "internal-metrics")]
pub use internal::*;

mod stddev;
pub use stddev::*;

mod outvec;
pub use outvec::*;

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
        self.collect_weighted_at(value, weight, SimTime::now())
    }

    ///
    /// Collects a non-weighted (w=1) datapoint at a given time.
    ///
    fn collect_at(&mut self, value: Self::Value, sim_time: SimTime) {
        self.collect_weighted_at(value, 1.0, sim_time)
    }

    ///
    /// Collects a non.weighted datapoint at the current time.
    ///
    fn collect(&mut self, value: Self::Value) {
        self.collect_weighted_at(value, 1.0, SimTime::now())
    }

    /// # Collections statisitcs

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize;
    fn sum(&self) -> Self::Value;
    fn sqrtsum(&self) -> Self::Value;
    fn min(&self) -> Self::Value;
    fn max(&self) -> Self::Value;
    fn mean(&self) -> Self::Value;
    fn std_derivation(&self) -> Self::Value;
    fn variance(&self) -> Self::Value;
}

///
/// A statistical metric that can be represented via a timeless
/// histogramm.
///
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
