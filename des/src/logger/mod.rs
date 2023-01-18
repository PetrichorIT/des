//! A simulation specific logger.

use std::{
    collections::HashMap,
    fmt::Debug,
    io::{stderr, stdout},
    sync::Arc,
};

mod filter;
mod fmt;
mod output;
mod record;

pub use fmt::LogFormat;
pub use output::LogOutput;
pub use record::LogRecord;

use crate::time::SimTime;
use log::{Level, LevelFilter, Log, SetLoggerError};
use termcolor::{BufferWriter, ColorChoice};

use self::filter::FilterPolicy;

static SCOPED_LOGGER: LoggerWrap = LoggerWrap::uninitalized();
static CURRENT_SCOPE: spin::Mutex<&'static str> = spin::Mutex::new("");

struct LoggerWrap {
    inner: spin::RwLock<Option<Logger>>,
}

impl LoggerWrap {
    const fn uninitalized() -> Self {
        Self {
            inner: spin::RwLock::new(None),
        }
    }

    fn reset(&self) {
        let mut lock = self.inner.write();
        *lock = None;
    }

    fn reset_contents(&self, new: Logger) {
        let mut lock = self.inner.write();
        lock.as_mut().unwrap().reset_contents(new);
    }

    fn set(&self, inner: Logger) -> Option<Logger> {
        let mut lock = self.inner.write();
        lock.replace(inner)
    }
}

impl Log for LoggerWrap {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        unsafe {
            let lock = self.inner.read();
            lock.as_ref().unwrap_unchecked().enabled(metadata)
        }
    }

    fn log(&self, record: &log::Record) {
        unsafe {
            let lock = self.inner.read();
            lock.as_ref().unwrap_unchecked().log(record);
        }
    }

    fn flush(&self) {
        unsafe {
            let lock = self.inner.read();
            lock.as_ref().unwrap_unchecked().flush();
        }
    }
}

/// A logger that collects scope specific messages.
pub struct Logger {
    active: bool,
    scopes: spin::Mutex<HashMap<String, LoggerScope>>,
    policy: Box<dyn LogScopeConfigurationPolicy>,
    interal_max_level: LevelFilter,
    filter: FilterPolicy,
}

/// An object to define a logger configuration policy.
pub trait LogScopeConfigurationPolicy {
    /// Configures a new logging scope with an output target and a
    /// base format.
    fn configure(&self, scope: &str) -> (Box<dyn LogOutput>, LogFormat);
}

impl LogScopeConfigurationPolicy for Box<LogScopeConfigurationPolicyFn> {
    fn configure(&self, scope: &str) -> (Box<dyn LogOutput>, LogFormat) {
        self(scope)
    }
}

/// A configuration function to create the log policy for a given scope.
pub type LogScopeConfigurationPolicyFn = dyn Fn(&str) -> (Box<dyn LogOutput>, LogFormat);

struct DefaultPolicy;
impl LogScopeConfigurationPolicy for DefaultPolicy {
    fn configure(&self, _scope: &str) -> (Box<dyn LogOutput>, LogFormat) {
        (Box::new((stdout(), stderr())), LogFormat::Color)
    }
}

impl Logger {
    /// Creates a new Logger (builder).
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: spin::Mutex::new(HashMap::new()),
            active: true,
            policy: Box::new(DefaultPolicy),
            interal_max_level: LevelFilter::Warn,
            filter: FilterPolicy::new(true),
        }
    }

    /// Creates a new Logger (builder) debug.
    #[must_use]
    #[doc(hidden)]
    pub fn debug() -> Self {
        Self {
            scopes: spin::Mutex::new(HashMap::new()),
            active: true,
            policy: Box::new(DefaultPolicy),
            interal_max_level: LevelFilter::Warn,
            filter: FilterPolicy::new(false),
        }
    }

    /// Begins a new scope, returning the currently active scope.
    #[doc(hidden)]
    pub fn begin_scope(ident: impl AsRef<str>) {
        let ident: *const str = ident.as_ref();
        let ident: &'static str = unsafe { &*ident };
        *CURRENT_SCOPE.lock() = ident;
    }

    /// Removes the current scope.
    #[doc(hidden)]
    pub fn end_scope() {
        *CURRENT_SCOPE.lock() = "";
    }

    fn reset_contents(&mut self, new: Self) {
        if self.active {
            *self = new;
        }
    }

    /// Adds a filter to the policy.
    #[must_use]
    pub fn add_filters(mut self, s: &str) -> Self {
        self.filter.parse_str(s);
        self
    }

    /// Sets the loggers activity status.
    #[must_use]
    pub fn active(mut self, is_active: bool) -> Self {
        self.active = is_active;
        self
    }
    /// Set the policy that dicates whether to forward messages to stdout
    #[must_use]
    pub fn policy(mut self, policy: impl LogScopeConfigurationPolicy + 'static) -> Self {
        self.policy = Box::new(policy);
        self
    }

    /// Sets the internal max level for all log message coming from internals
    #[must_use]
    pub fn interal_max_log_level(mut self, level: LevelFilter) -> Self {
        self.interal_max_level = level;
        self
    }

    /// Connects the logger to the logging framework
    ///
    /// # Panics
    ///
    /// Panics if another logger as allready set, or sombody steals the
    /// logger from the static registry in a race condition.
    pub fn set_logger(self) {
        self.try_set_logger().expect("Failed to set logger")
    }

    /// Connects the logger to the logging framework.
    ///
    /// # Errors
    ///
    /// This will fail if another logger is allready set,
    /// that is not of this type. If the other logger is of this type,
    /// it will be replaced.
    ///
    /// # Panics
    ///
    /// Panics if somebody steals the logger from the static registry
    /// in a race condition.
    ///
    pub fn try_set_logger(self) -> Result<(), SetLoggerError> {
        let old = SCOPED_LOGGER.set(self);
        match log::set_logger(&SCOPED_LOGGER).map(|()| log::set_max_level(LevelFilter::Trace)) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Since a logger was allready set it might either be a
                // ScopedLogger or somthing elsewhere
                // If SCOPED_LOGGER is Some that this logger is the set logger.

                if let Some(v) = old {
                    // Old was Scoped logger so keep the old logger and reset it.
                    let recently_created = SCOPED_LOGGER.set(v);
                    SCOPED_LOGGER.reset_contents(recently_created.unwrap());
                    Ok(())
                } else {
                    SCOPED_LOGGER.reset();
                    Err(e)
                }
            }
        }
    }
}

impl Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logger")
            .field("filters", &self.filter)
            .field("internal", &self.interal_max_level)
            .finish()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.active && metadata.level() <= log::STATIC_MAX_LEVEL
    }

    fn log(&self, record: &log::Record) {
        // (0) Check general activity metadata
        if !self.enabled(record.metadata()) {
            return;
        }

        // (1) If the source is internal (but still marked) add this filter
        let source_is_internal = record.module_path().map_or(false, |s| s.starts_with("des"));
        if source_is_internal && record.level() > self.interal_max_level {
            return;
        }

        // (2) Get scopes by ptr
        let mut scopes = self.scopes.lock();

        // (3) Get target pointer
        let target_is_module_path = Some(record.metadata().target()) == record.module_path();
        let target_label = if target_is_module_path {
            String::new()
        } else {
            format!(" ({})", record.metadata().target())
        };

        // (4) Get scope or make defeault print based on the target marker.
        let scope_label = CURRENT_SCOPE.lock().clone();
        if scope_label.is_empty() {
            // if policy(record.target()) {
            // No target scope was given --- not scoped println.
            let out = match record.level() {
                Level::Error | Level::Warn => BufferWriter::stderr(ColorChoice::Always),
                _ => BufferWriter::stdout(ColorChoice::Always),
            };
            let mut buffer = out.buffer();

            let record = LogRecord {
                scope: Arc::new(record.target().to_string()),
                target: target_label,
                level: record.level(),
                time: SimTime::now(),
                msg: format!("{}", record.args()),
            };

            LogFormat::fmt(LogFormat::Color, &record, &mut buffer)
                .expect("Failed to write record to output stream");
            out.print(&buffer)
                .expect("Failed to write to output buffer");
            // }
            return;
        };

        // (5) Fetch scope information
        let scope = scopes.get_mut(scope_label);
        if let Some(scope) = scope {
            scope.log(format!("{}", record.args()), target_label, record.level());
        } else {
            // TODO: Check target validity
            let (output, fmt) = self.policy.configure(scope_label);

            let mut new_scope = LoggerScope {
                scope: Arc::new(scope_label.to_string()),
                output,
                fmt,
                filter: dbg!(self.filter.filter_for(scope_label, LevelFilter::max())),
            };

            new_scope.log(format!("{}", record.args()), target_label, record.level());

            scopes.insert(scope_label.to_string(), new_scope);
        }
    }

    fn flush(&self) {}
}

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

/// A collection of all logging activity in one scope.
pub(self) struct LoggerScope {
    /// The target identifier for the current logger.
    scope: Arc<String>,
    output: Box<dyn LogOutput>,
    fmt: LogFormat,
    filter: LevelFilter,
}

impl LoggerScope {
    fn log(&mut self, msg: String, target: String, level: Level) {
        if level > self.filter {
            return;
        }

        let record = LogRecord {
            msg,
            level,
            time: SimTime::now(),
            scope: self.scope.clone(),
            target,
        };

        if let Err(e) = self.output.write(&record, self.fmt) {
            eprintln!("failed to write to logging output: {e}")
        }
    }
}

impl Debug for LoggerScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggerScope")
            .field("scope", &self.scope)
            .field("fmt", &self.fmt)
            .field("filter", &self.filter)
            .finish_non_exhaustive()
    }
}
