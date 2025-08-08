use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use crate::time::SimTime;

use super::{FT_ASYNC, FT_CQUEUE, FT_NET};

/// A run profiler
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profiler<E> {
    /// The target executable.
    pub target: PathBuf,
    /// The exec
    pub exec: String,

    /// Whether the target is in release mode or not.
    pub target_is_release: bool,

    /// The time point where the simulation started.
    pub simulation_start: SystemTime,

    /// Timespec
    pub time_start: Instant,
    /// The duration of the simulation.
    pub duration: Duration,

    /// The remaining events.
    pub remaining: Vec<(E, SimTime)>,

    /// The number of events that where executed.
    pub event_count: usize,
    /// The active features.
    pub features: Vec<String>,
}

impl<E> Profiler<E> {
    /// Starts the profile.
    pub(super) fn start(&mut self) {
        self.time_start = Instant::now();
    }

    /// Finishes the profile.
    pub(super) fn finish(&mut self, event_count: usize) {
        self.event_count = event_count;
        let now = Instant::now();
        self.duration = now - self.time_start;
    }
}

#[cfg(debug_assertions)]
fn is_release() -> bool {
    false
}

#[cfg(not(debug_assertions))]
fn is_release() -> bool {
    true
}

impl<E> Default for Profiler<E> {
    fn default() -> Self {
        let target = if cfg!(feature = "miri") {
            PathBuf::from("sim-run") // default for miri run, to ignore external calls
        } else {
            std::env::current_exe().unwrap_or_default()
        };

        let target_is_release = is_release();

        let mut exec = target
            .file_name()
            .expect("Failed to find binary")
            .to_string_lossy()
            .to_string();
        if target_is_release {
            exec.push_str("-release");
        }

        let mut features = Vec::with_capacity(5);
        if FT_CQUEUE {
            features.push("cqueue".into());
        }
        if FT_NET {
            features.push("net".into());
        }
        if FT_ASYNC {
            features.push("async".into());
        }

        Self {
            target,
            exec,
            target_is_release,

            simulation_start: if cfg!(feature = "miri") {
                SystemTime::UNIX_EPOCH
            } else {
                SystemTime::now()
            },
            time_start: Instant::now(),
            duration: Duration::ZERO,

            remaining: Vec::new(),

            event_count: 0,
            features,
        }
    }
}
