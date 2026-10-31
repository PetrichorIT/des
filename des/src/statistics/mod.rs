//! Statistical results collection.

use std::any::Any;

pub mod histogram;
pub mod std_dev;
pub mod time_series;

/// Trait for statistical results.
pub trait Statistic: Any {
    /// The result type emitted by the statistic.
    type Result: Any;
}
