use crate::{
    stats::{Statistic, StdDev},
    time::SimTime,
};
use std::{
    fmt::Debug,
    fs::{remove_file, File, OpenOptions},
    io::{BufWriter, Write},
};

#[cfg(feature = "net")]
use crate::net::ObjectPath;

///
/// A vector of values that will be written to a file.
///
#[derive(Clone)]
pub struct OutVec {
    name: String,
    #[cfg(feature = "net")]
    owner: Option<ObjectPath>,
    results_dir: String,

    buffered_values: Vec<(f64, f64)>,
    stddev: StdDev,
    max_buffered_values: usize,

    output_allready_written: bool,
}

impl OutVec {
    ///
    /// Returns the path to the owner of this `OutVec`.
    ///
    #[must_use]
    pub fn path(&self) -> String {
        #[cfg(feature = "net")]
        match &self.owner {
            Some(owner) => format!("{}_{}", owner, self.name),
            None => self.name.clone(),
        }

        #[cfg(not(feature = "net"))]
        self.name.clone()
    }

    ///
    /// Creates a new `OutVec` bound to a onwer.
    ///
    #[must_use]
    pub fn new(name: String, #[cfg(feature = "net")] owner: Option<ObjectPath>) -> Self {
        Self {
            #[cfg(feature = "net")]
            owner,

            name,
            results_dir: String::from("results"),

            buffered_values: Vec::new(),
            stddev: StdDev::new(),
            max_buffered_values: !0,

            output_allready_written: false,
        }
    }

    ///
    /// Configures the max amount of buffered values.
    ///
    #[must_use]
    pub fn buffer_max(mut self, max_buffered_values: usize) -> Self {
        self.max_buffered_values = max_buffered_values;
        self
    }

    ///
    /// Configures the directory the results will be written to
    ///
    #[must_use]
    pub fn result_dir(mut self, dir: String) -> Self {
        self.results_dir = dir;
        self
    }

    ///
    /// Resets all values to their inital state.
    ///
    pub fn clear(&mut self) {
        if self.output_allready_written {
            let path = format!("results/{}.out", self.path());
            match remove_file(&path) {
                Ok(_) => (),

                #[cfg(feature = "tracing")]
                Err(e) => {
                    tracing::error!(target: "metrics (OutVec)", "Failed to remove metrics file '{}' after clear: {}", path, e);
                }

                #[cfg(not(feature = "tracing"))]
                Err(_) => {}
            }
        }

        self.buffered_values.clear();
        self.output_allready_written = false;
    }

    ///
    /// Finishes the `OutVec`, writing the data to the result file.
    ///
    pub fn finish(&mut self) {
        self.try_write_to_file();
    }

    fn try_write_to_file(&mut self) {
        // Write to output file.
        let path = format!("{}/{}.out", self.results_dir, self.path());

        let file = if self.output_allready_written {
            OpenOptions::new().write(true).append(true).open(&path)
        } else {
            File::create(&path)
        };

        let file = match file {
            Ok(file) => file,

            #[cfg(feature = "tracing")]
            Err(e) => {
                tracing::error!(target: "metrics (OutVec)", "Failed to write metrics to file '{}': {}", path, e);
                return;
            }

            #[cfg(not(feature = "tracing"))]
            Err(_) => return,
        };

        let mut writer = BufWriter::new(file);

        writeln!(writer, "# Slice written at {}", SimTime::now()).unwrap();
        for (time, value) in self.buffered_values.drain(..) {
            writeln!(writer, "{time} = {value}").unwrap();
        }

        // Done
        self.output_allready_written = true;
    }
}

impl Statistic for OutVec {
    type Value = f64;

    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, sim_time: SimTime) {
        assert!(
            (weight - 1.0).abs() < f64::EPSILON,
            "OutVec cannot function using specific weights"
        );
        self.stddev.collect_weighted_at(value, weight, sim_time);
        self.buffered_values.push((sim_time.into(), value));

        // check for output
        if self.buffered_values.len() > self.max_buffered_values {
            self.try_write_to_file();
        }
    }

    fn len(&self) -> usize {
        self.stddev.len()
    }

    fn sum(&self) -> Self::Value {
        self.stddev.sum()
    }

    fn sqrtsum(&self) -> Self::Value {
        self.stddev.sqrtsum()
    }

    fn min(&self) -> Self::Value {
        self.stddev.min()
    }

    fn max(&self) -> Self::Value {
        self.stddev.max()
    }

    fn mean(&self) -> Self::Value {
        self.stddev.mean()
    }

    fn std_derivation(&self) -> Self::Value {
        self.stddev.std_derivation()
    }

    fn variance(&self) -> Self::Value {
        self.stddev.variance()
    }

    fn collect_weighted(&mut self, value: Self::Value, weight: f64) {
        self.collect_weighted_at(value, weight, SimTime::now());
    }

    fn collect_at(&mut self, value: Self::Value, sim_time: SimTime) {
        self.collect_weighted_at(value, 1.0, sim_time);
    }

    fn collect(&mut self, value: Self::Value) {
        self.collect_weighted_at(value, 1.0, SimTime::now());
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Debug for OutVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutVec")
            .field("path", &self.path())
            .field("value", &self.buffered_values)
            .field("stddev", &self.stddev)
            .field("max_buffered_values", &self.max_buffered_values)
            .field("written", &self.output_allready_written)
            .finish()
    }
}
