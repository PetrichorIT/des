use crate::{SimTime, Statistic, StdDev};
use log::error;
use std::{
    fmt::Debug,
    fs::{remove_file, File, OpenOptions},
    io::{BufWriter, Write},
};

#[cfg(feature = "net")]
use crate::ModulePath;

///
/// A vector of values that will be written to a file.
///
#[derive(Clone)]
pub struct OutVec {
    name: String,
    #[cfg(feature = "net")]
    owner: Option<ModulePath>,
    results_dir: String,

    buffered_values: Vec<(f64, f64)>,
    stddev: StdDev,
    max_buffered_values: usize,

    output_allready_written: bool,
}

impl OutVec {
    pub fn path(&self) -> String {
        #[cfg(feature = "net")]
        match &self.owner {
            Some(owner) => format!("{}_{}", owner, self.name),
            None => self.name.clone(),
        }

        #[cfg(not(feature = "net"))]
        self.name.clone()
    }

    pub fn new(name: String, #[cfg(feature = "net")] owner: Option<ModulePath>) -> Self {
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

    pub fn buffer_max(mut self, max_buffered_values: usize) -> Self {
        self.max_buffered_values = max_buffered_values;
        self
    }

    pub fn result_dir(mut self, dir: String) -> Self {
        self.results_dir = dir;
        self
    }

    pub fn clear(&mut self) {
        if self.output_allready_written {
            let path = format!("results/{}.out", self.path());
            match remove_file(&path) {
                Ok(_) => (),
                Err(e) => {
                    error!(target: "metrics (OutVec)", "Failed to remove metrics file '{}' after clear: {}", path, e);
                }
            }
        }

        self.buffered_values.clear();
        self.output_allready_written = false
    }

    pub fn finish(&mut self) {
        self.try_write_to_file()
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
            Err(e) => {
                error!(target: "metrics (OutVec)", "Failed to write metrics to file '{}': {}", path, e);
                return;
            }
        };

        let mut writer = BufWriter::new(file);

        writeln!(writer, "# Slice written at {}", SimTime::now()).unwrap();
        for (time, value) in self.buffered_values.drain(..) {
            writeln!(writer, "{} = {}", time, value).unwrap();
        }

        // Done
        self.output_allready_written = true;
    }
}

impl Statistic for OutVec {
    type Value = f64;

    fn collect_weighted_at(&mut self, value: Self::Value, weight: f64, sim_time: crate::SimTime) {
        assert_eq!(weight, 1.0, "OutVec cannot function using specific weights");
        self.stddev.collect_weighted_at(value, weight, sim_time);
        self.buffered_values.push((sim_time.into(), value));

        // check for output
        if self.buffered_values.len() > self.max_buffered_values {
            self.try_write_to_file()
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
        self.collect_weighted_at(value, weight, crate::SimTime::now())
    }

    fn collect_at(&mut self, value: Self::Value, sim_time: crate::SimTime) {
        self.collect_weighted_at(value, 1.0, sim_time)
    }

    fn collect(&mut self, value: Self::Value) {
        self.collect_weighted_at(value, 1.0, crate::SimTime::now())
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
