use std::{fs::File, io::BufWriter, path::Path};

use crate::time::SimTime;

use super::Statistic;

/// Collects vallues in a vectors, combining all elements in
/// a given time slot into a mean value, returning a vectors of means.
#[derive(Debug, Clone)]
pub struct MeanVec {
    slot_size: SimTime,

    current_slot_end: SimTime,
    current_slot_buffer: Vec<f64>,

    /// (MEAN, MIN, MAX);
    results: Vec<(f64, f64, f64)>,
}

impl MeanVec {
    /// Creates a new instance with the given slot size.
    #[must_use]
    pub fn new(slot_size: SimTime) -> Self {
        Self {
            slot_size,

            current_slot_end: slot_size,
            current_slot_buffer: Vec::new(),

            results: Vec::new(),
        }
    }

    fn mean_step(&mut self) {
        if self.current_slot_buffer.is_empty() {
            self.results
                .push(self.results.last().map(|v| *v).unwrap_or((0.0, 0.0, 0.0)));
        } else {
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            let mut sum = 0.0;
            for v in &self.current_slot_buffer {
                sum += *v;
                if *v < min {
                    min = *v
                }
                if *v > max {
                    max = *v;
                }
            }
            self.results
                .push((sum / self.current_slot_buffer.len() as f64, min, max));
        }

        self.current_slot_end += self.slot_size;
        self.current_slot_buffer.clear();
    }

    /// Finishes th last computation
    pub fn finish(&mut self) {
        self.mean_step()
    }

    /// Writes the results as json to the output
    pub fn try_write_to(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let f = File::create(path)?;
        let mut f = BufWriter::new(f);

        use std::io::Write;
        write!(f, "{{ \"results\": [ ")?;
        for (mean, min, max) in &self.results {
            write!(f, "{{ \"command\": \"mean-vec\", \"mean\": {}, \"min\": {}, \"max\": {}, \"parameters\": {{ \"time\": {} }} }}", mean, min, max, SimTime::now().as_secs_f64().ceil())?
        }
        write!(f, "] }}")?;

        Ok(())
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

    fn collect_weighted_at(&mut self, value: Self::Value, _weight: f64, sim_time: SimTime) {
        while sim_time > self.current_slot_end {
            self.mean_step();
        }

        self.current_slot_buffer.push(value);
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
