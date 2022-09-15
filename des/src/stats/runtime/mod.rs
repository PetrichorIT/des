// Runtime metrics
// ===
//
// The following features apply:
// - metrics (base feature that activates all constant space metrics)
// - metrics-rt-full (Stores a list of event counts, memory inefficents)
// - metrics-net-full
use std::{
    fs::OpenOptions,
    io::BufWriter,
    path::{Path, PathBuf},
};

cfg_net! {
    mod channel;
    pub use channel::*;
}

cfg_not_cqueue! {
    mod heap;
    pub use heap::*;
}

cfg_cqueue! {
    mod cqueue;
    pub use cqueue::*;
}

cfg_metrics_rt_full! {
    mod rt_full;
    pub(crate) use rt_full::*;
}

/// Defines where the output of a profiler should be written to.
#[derive(Debug, PartialEq, Eq)]
pub struct ProfilerOutputTarget {
    log_output: Option<PathBuf>,
    log_create: bool,
    log_append: bool,

    #[cfg(feature = "metrics-rt-full")]
    event_count_output: Option<PathBuf>,
}

impl ProfilerOutputTarget {
    /// Creates a new instance of Self.s
    #[must_use]
    pub fn new() -> Self {
        Self {
            log_output: None,
            log_create: true,
            log_append: false,

            #[cfg(feature = "metrics-rt-full")]
            event_count_output: None,
        }
    }

    /// Sets the output file.
    #[must_use]
    pub fn write_into(mut self, f: impl AsRef<Path>) -> Self {
        let path = f.as_ref().to_owned();
        self.log_output = Some(path);
        self
    }

    /// Sets the option to create files if nessecary
    #[must_use]
    pub fn opt_create(mut self, b: bool) -> Self {
        self.log_create = b;
        self
    }

    /// Set the option to append to files.
    #[must_use]
    pub fn opt_append(mut self, b: bool) -> Self {
        self.log_append = b;
        self
    }

    /// Sets the output file for the `event_count` (as json).
    #[cfg(feature = "metrics-rt-full")]
    #[must_use]
    pub fn write_event_count_into(mut self, f: impl AsRef<Path>) -> Self {
        let path = f.as_ref().to_owned();
        self.event_count_output = Some(path);
        self
    }

    pub(crate) fn run(self, metrics: &RuntimeMetrics) -> std::io::Result<()> {
        if let Some(path) = self.log_output {
            let f = OpenOptions::new()
                .append(self.log_append)
                .create(self.log_create)
                .write(true)
                .open(path)
                .unwrap();
            let mut f = BufWriter::new(f);
            metrics.write_to(&mut f)?;
        } else {
            eprintln!("Didn't set output path at ProfilerOutputTarget");
        }

        #[cfg(feature = "metrics-rt-full")]
        if let Some(path) = self.event_count_output {
            let f = OpenOptions::new().create(true).write(true).open(path)?;
            let mut f = BufWriter::new(f);
            metrics.write_event_count_to(&mut f)?;
        } else {
            eprintln!("Didn't set event_count_output path at ProfilerOutputTarget");
        }

        Ok(())
    }
}

impl From<&str> for ProfilerOutputTarget {
    fn from(string: &str) -> Self {
        ProfilerOutputTarget::new()
            .write_into(string)
            .opt_create(true)
    }
}

impl Default for ProfilerOutputTarget {
    fn default() -> Self {
        Self::new()
    }
}
