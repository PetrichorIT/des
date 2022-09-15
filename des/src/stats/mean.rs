use crate::time::{Duration, SimTime};

use super::{Statistic, StdDev};

/// Collects vallues in a vectors, combining all elements in
/// a given time slot into a mean value, returning a vectors of means.
#[derive(Debug, Clone, PartialEq)]
pub struct MeanVec {
    slot_size: Duration,

    current_slot_end: SimTime,
    current_slot_buffer: StdDev,

    /// (END: MEAN, MIN, MAX; STDDEV);
    pub(super) results: Vec<(SimTime, f64, f64, f64, f64)>,
}

impl MeanVec {
    /// Creates a new instance with the given slot size.
    #[must_use]
    pub fn new(slot_size: Duration) -> Self {
        Self {
            slot_size,

            current_slot_end: SimTime::ZERO + slot_size,
            current_slot_buffer: StdDev::new(),

            results: Vec::new(),
        }
    }

    fn mean_step(&mut self) {
        if self.current_slot_buffer.is_empty() {
            self.results.push(self.results.last().copied().unwrap_or((
                self.current_slot_end,
                0.0,
                0.0,
                0.0,
                0.0,
            )));
        } else {
            self.results.push((
                self.current_slot_end,
                self.current_slot_buffer.mean(),
                self.current_slot_buffer.min(),
                self.current_slot_buffer.max(),
                self.current_slot_buffer.std_derivation(),
            ));
        }

        self.current_slot_end += self.slot_size;
        self.current_slot_buffer.clear();
    }

    /// Finishes th last computation
    pub fn finish(&mut self) {
        self.mean_step();
    }
}

impl Statistic for MeanVec {
    type Value = f64;

    fn len(&self) -> usize {
        self.results.len()
    }

    fn sum(&self) -> Self::Value {
        todo!()
    }

    fn sqrtsum(&self) -> Self::Value {
        todo!()
    }

    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, sim_time: SimTime) {
        while sim_time > self.current_slot_end {
            self.mean_step();
        }

        self.current_slot_buffer
            .collect_weighted_at(value, weight, sim_time);
    }

    fn min(&self) -> Self::Value {
        todo!()
    }

    fn max(&self) -> Self::Value {
        todo!()
    }

    fn mean(&self) -> Self::Value {
        todo!()
    }

    fn std_derivation(&self) -> Self::Value {
        todo!()
    }

    fn variance(&self) -> Self::Value {
        todo!()
    }
}

impl Eq for MeanVec {}
