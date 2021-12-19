use std::ops::RangeInclusive;

use crate::SimTime;

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

    fn datapoints_len(&self) -> usize;
    fn datapoints_sum(&self) -> Self::Value;
    fn datapoints_sqrtsum(&self) -> Self::Value;
    fn datapoints_min(&self) -> Self::Value;
    fn datapoints_max(&self) -> Self::Value;
    fn datapoints_mean(&self) -> Self::Value;
    fn datapoints_std_derivation(&self) -> Self::Value;
    fn datapoints_variance(&self) -> Self::Value;
}

///
/// The type to collect a accumulated value, provinding
/// standartised metrics like e.g. standart derivation.
///
#[derive(Debug, Clone, PartialEq)]
pub struct StdDev {
    min: f64,
    max: f64,

    num_values: usize,
    sum: f64,
    sum_weights: f64,
    sqrtsum: f64,
    sqrtsum_weights: f64,
}

impl StdDev {
    ///
    /// Creates  a new instance of StdDev.
    ///
    pub fn new() -> Self {
        Self {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,

            num_values: 0,
            sum: 0.0,
            sum_weights: 0.0,
            sqrtsum: 0.0,
            sqrtsum_weights: 0.0,
        }
    }
}

impl Default for StdDev {
    fn default() -> Self {
        Self::new()
    }
}

impl Statistic for StdDev {
    type Value = f64;

    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, _sim_time: SimTime) {
        self.num_values += 1;

        if self.min > value {
            self.min = value;
        }
        if self.max < value {
            self.max = value;
        }

        self.sum += weight * value;
        self.sum_weights += weight;

        self.sqrtsum += weight * value * value;
        self.sqrtsum_weights += weight * weight;
    }

    fn datapoints_len(&self) -> usize {
        self.num_values
    }

    fn datapoints_sum(&self) -> Self::Value {
        self.sum
    }

    fn datapoints_sqrtsum(&self) -> Self::Value {
        self.sqrtsum
    }

    fn datapoints_min(&self) -> Self::Value {
        self.min
    }

    fn datapoints_max(&self) -> Self::Value {
        self.max
    }

    fn datapoints_mean(&self) -> Self::Value {
        self.sum / (self.num_values as f64)
    }

    fn datapoints_std_derivation(&self) -> Self::Value {
        self.datapoints_variance().sqrt()
    }

    fn datapoints_variance(&self) -> Self::Value {
        if self.num_values == 0 {
            f64::NAN
        } else {
            let var = (self.sum_weights * self.sqrtsum - self.sum * self.sum)
                / (self.sum_weights * self.sum_weights - self.sqrtsum_weights);
            if var < 0.0 {
                0.0
            } else {
                var
            }
        }
    }
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

    fn datapoints_len(&self) -> usize {
        self.bins.iter().fold(0.0, |acc, &e| acc + e) as usize
    }

    fn datapoints_sum(&self) -> Self::Value {
        let mut sum_value = 0.0;

        for b in &self.bins {
            let b = *b as f64;
            let rel = b / self.bins.len() as f64;
            let pow = self.lower_bound + rel * self.interval;

            sum_value += b * pow;
        }

        sum_value
    }

    fn datapoints_sqrtsum(&self) -> Self::Value {
        let mut sum_value = 0.0;

        for b in &self.bins {
            let b = *b as f64;
            let rel = b / self.bins.len() as f64;
            let pow = self.lower_bound + rel * self.interval;

            sum_value += b * pow * pow;
        }

        sum_value
    }

    fn datapoints_min(&self) -> Self::Value {
        self.min
    }

    fn datapoints_max(&self) -> Self::Value {
        self.max
    }

    fn datapoints_mean(&self) -> Self::Value {
        self.datapoints_sum() / self.datapoints_len() as f64
    }

    fn datapoints_std_derivation(&self) -> Self::Value {
        todo!()
    }

    fn datapoints_variance(&self) -> Self::Value {
        todo!()
    }
}
