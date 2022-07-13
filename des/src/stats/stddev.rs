use crate::stats::Statistic;
use crate::time::SimTime;
use std::fmt::Display;

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

    ///
    /// Resets all values to their inital state.
    ///
    pub fn clear(&mut self) {
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;

        self.num_values = 0;
        self.sum = 0.0;
        self.sum_weights = 0.0;
        self.sqrtsum = 0.0;
        self.sqrtsum_weights = 0.0;
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

    fn len(&self) -> usize {
        self.num_values
    }

    fn sum(&self) -> Self::Value {
        self.sum
    }

    fn sqrtsum(&self) -> Self::Value {
        self.sqrtsum
    }

    fn min(&self) -> Self::Value {
        self.min
    }

    fn max(&self) -> Self::Value {
        self.max
    }

    fn mean(&self) -> Self::Value {
        self.sum / (self.num_values as f64)
    }

    fn std_derivation(&self) -> Self::Value {
        self.variance().sqrt()
    }

    fn variance(&self) -> Self::Value {
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

impl Display for StdDev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mean: {:>7.3} with derivation {:>7.3} (min: {:>7.3} max: {:>7.3} len: {})",
            self.mean(),
            self.std_derivation(),
            self.min(),
            self.max(),
            self.num_values
        )
    }
}

///
/// The type to collect a accumulated value, provinding
/// standartised metrics like e.g. standart derivation.
/// Compresses a normal StdDev in a two-dimensional structure.
///
#[derive(Debug, Clone, PartialEq)]
pub struct CompressedStdDev {
    current_collector: StdDev,
    bound: usize,

    min: f64,
    max: f64,

    num_values_total: usize,
    num_values_compressed: usize,

    sum: f64,
    sqrtsum: f64,
    var_sum: f64,
}

impl CompressedStdDev {
    ///
    /// Creates  a new instance of CompressedStdDev.
    ///
    #[allow(unused)]
    pub fn new(bound: usize) -> Self {
        Self {
            current_collector: StdDev::new(),
            bound,

            min: 0.0,
            max: 0.0,

            num_values_total: 0,
            num_values_compressed: 0,

            sum: 0.0,
            sqrtsum: 0.0,
            var_sum: 0.0,
        }
    }

    ///
    /// Flushes the compression mechanisms to forward all values
    /// to the compressed state.
    ///
    pub fn flush(&mut self) {
        let min = self.current_collector.min();
        let max = self.current_collector.max();

        let mean = self.current_collector.mean();
        let variance = self.current_collector.variance();

        self.min = self.min.min(min);
        self.max = self.max.max(max);

        self.num_values_total += self.current_collector.len();
        self.num_values_compressed += 1;

        self.sum += mean;
        self.sqrtsum += mean * mean;

        self.var_sum += variance;

        self.current_collector.clear();
    }
}

impl Statistic for CompressedStdDev {
    type Value = f64;

    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, sim_time: SimTime) {
        self.current_collector
            .collect_weighted_at(value, weight, sim_time);
        // check for flush
        if self.current_collector.len() > self.bound {
            self.flush()
        }
    }

    fn len(&self) -> usize {
        self.num_values_total
    }

    fn sum(&self) -> Self::Value {
        self.sum
    }

    fn sqrtsum(&self) -> Self::Value {
        self.sqrtsum
    }

    fn min(&self) -> Self::Value {
        self.min
    }

    fn max(&self) -> Self::Value {
        self.max
    }

    fn mean(&self) -> Self::Value {
        self.sum() / (self.num_values_compressed as f64)
    }

    fn std_derivation(&self) -> Self::Value {
        self.variance().sqrt()
    }

    fn variance(&self) -> Self::Value {
        self.var_sum / (self.num_values_compressed as f64)
    }
}

impl Display for CompressedStdDev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mean: {:>7.3} with derivation {:>7.3} (min: {:>7.3} max: {:>7.3} len: {}/{})",
            self.mean(),
            self.std_derivation(),
            self.min(),
            self.max(),
            self.num_values_compressed,
            self.num_values_total,
        )
    }
}
