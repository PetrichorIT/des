use std::{
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use sysinfo::{CpuExt, SystemExt};

use super::{FT_ASYNC, FT_CQUEUE, FT_INTERNAL_METRICS, FT_NET};

/// A run profiler
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profiler {
    /// The target executable.
    pub target: PathBuf,
    exec: String,

    /// Whether the target is in release mode or not.
    pub target_is_release: bool,
    /// The hardware enviroment of the execution.
    pub env: ProfilerEnv,

    /// The time point where the simulation started.
    pub simulation_start: SystemTime,

    time_start: Instant,
    /// The duration of the simulation.
    pub duration: Duration,

    /// The number of events that where executed.
    pub event_count: usize,
    /// The active features.
    pub features: Vec<String>,
}

impl Profiler {
    /// Returns the ident str for the profiler
    #[allow(unused)]
    pub(super) fn ident(&self) -> String {
        format!(
            "{}--{}-{}-{}",
            self.exec, self.env.arch, self.env.os_family, self.env.os
        )
    }

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

impl Default for Profiler {
    fn default() -> Self {
        let target = std::env::current_exe().unwrap_or_default();
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
        if FT_INTERNAL_METRICS {
            features.push("metrics".into());
        }

        Self {
            target,
            exec,
            target_is_release,

            env: ProfilerEnv::default(),

            simulation_start: SystemTime::now(),
            time_start: Instant::now(),
            duration: Duration::ZERO,

            event_count: 0,
            features,
        }
    }
}

/// A description of the runtime enviroment.
#[derive(Debug, Clone)]
pub struct ProfilerEnv {
    /// The target arch.
    pub arch: String,
    /// The target os.
    pub os: String,
    /// The target os family.
    pub os_family: String,

    #[allow(unused)]
    system: Arc<sysinfo::System>,
}

impl ProfilerEnv {
    #[allow(unused)]
    fn write_to(&self, f: &mut impl Write) -> std::io::Result<()> {
        writeln!(
            f,
            "\t{} / {}",
            self.system
                .host_name()
                .unwrap_or_else(|| "Unknown-System".into()),
            self.system
                .long_os_version()
                .unwrap_or_else(|| self.os.clone())
        )?;
        writeln!(f, "\t{}-{}-{}", self.arch, self.os_family, self.os)?;
        if let Some(cpu) = self.system.cpus().first() {
            writeln!(
                f,
                "\t{} ({} / {}) @ {}MHz",
                cpu.name(),
                cpu.brand(),
                cpu.vendor_id(),
                cpu.frequency()
            )?;
        }
        writeln!(
            f,
            "\tmem: {} total {} swap",
            self.system.total_memory(),
            self.system.total_swap()
        )?;

        Ok(())
    }
}

impl PartialEq for ProfilerEnv {
    fn eq(&self, other: &Self) -> bool {
        self.arch == other.arch && self.os == other.os && self.os_family == other.os_family
    }
}

impl Eq for ProfilerEnv {}

impl Default for ProfilerEnv {
    fn default() -> Self {
        let mut system = sysinfo::System::new();
        system.refresh_cpu();
        system.refresh_memory();

        Self {
            arch: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
            os_family: std::env::consts::FAMILY.to_string(),

            system: Arc::new(system),
        }
    }
}
