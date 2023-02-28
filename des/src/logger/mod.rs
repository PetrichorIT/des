//! A simulation specific logger.
//!
//! # Why ?
//!
//! The reason 'des' provides a simulation specific logger is simple.
//! When creating logs from specific network-nodes in the simulation, its important
//! that each node has its own stdout/stderr for logging. Obviously thats not
//! possible with a normal logging implementation, since all simulated nodes still run in the
//! same process, thus share a stdout/stderr. The logger provided in this submodule
//! automatically recognizes which module / network-node is active and accordingly
//! annotates the log entry.
//!
//! # How can i set it up ?
//!
//! This is a logger implementation based on the default loggin api provided by [`log`].
//! To use it, create a new Logger at the start of your programm, configure the logger and then
//! set the logger. When another logger is allready set, this operation will fail, except if the
//! allready existing logger is also a instance of [`Logger`].
//!
//! ```
//! # use des::prelude::*;
//! # use des::logger::*;
//! fn main() {
//!     Logger::new()
//!         .add_filters("*.router=warn")
//!         .try_set_logger()
//!         .expect("Failed to set logger");
//!     /* ... */
//! }
//! ```
//!
//! # What can be configured ?
//!
//! This logger works by creating scopes. A scope describes a custom output target for
//! your log messages. When a logger is created the user can provide some policies and filters
//! how scopes are created.
//!
//! A 'LogScopeConfigurationPolicy' is a policy object, that can configure abitrary scopes.
//! When a new scope is created this policy receives the scope name as a parameter,
//! and returns a [`LogOutput`] object and a [`LogFormat`] identifier. The
//! output object is used to write log messages to some external target. The default target
//! would be an stdout/stderr pair, but other targets are also valid. For example
//! logs could be written to a file, a vector of strings, or a TCP stream (not implemented by default).
//! The format argument is self-explanatory: It defines whether the write expects a output with, or without
//! color support (usefull if the output is stdout/stderr).
//!
//! ```
//! # use des::prelude::*;
//! # use des::logger::*;
//! # use std::io::{self, stdout, stderr};
//! # use log::Level;
//! # use std::fs::File;
//! struct MixedLoggingOutput {
//!     file: File,
//! }
//!
//! impl LogOutput for MixedLoggingOutput {
//!     fn write(&mut self, record: &LogRecord, fmt: LogFormat) -> io::Result<()> {
//!         self.file.write(record, fmt)?;
//!         if record.level == Level::Error {
//!             let mut default_output = (stdout(), stderr());
//!             default_output.write(record, fmt)?;
//!         }
//!         Ok(())
//!     }    
//! }
//!
//! fn main() {
//!     let p: Box<LogScopeConfigurationPolicyFn> = Box::new(|s| {
//!         (
//!             Box::new(MixedLoggingOutput {
//!                 file: std::fs::File::create(format!("{s}.log")).unwrap(),
//!             }),
//!             LogFormat::Color,
//!         )  
//!     });
//!
//!     Logger::new()
//!         .policy(p)
//!         .try_set_logger()
//!         .expect("Failed to set logger");
//!     /* ... */
//! }
//! ```
//!
//! Additionally the user can define filters. Filter restrict which log levels are logged per scope
//! at runtime, simuilar to the 'max_level_*' features of [`log`], but with much greater control.
//! Filters can be defined by text, and thus be read from env-vars for custom runtime behaviour.
//!
//! Finally an internal log level can be set. By default [`des`] does not log any internal event-handling below
//! the WARN level. By setting the internal log level, user can expose internal logs.
//!
//! # When do i create scopes ?
//!
//! In 99% of cases, never. Scopes are automatically created when using the feature 'net'.
//! Additionally which scope is active is automatically determined by [`des`]. If you want
//! to create 'subscopes' within one network node, provide the logger with a custom target.
//! This target will be added to the log entry as an extra parameter.
//!
//! # How do i use the logger ?
//!
//! Just as you would use every other logger using the [`log`] crate.
//!
//! ```
//! # use des::prelude::*;
//! struct MyModule;
//! impl Module for MyModule {
//!     fn new() -> Self {
//!         Self
//!     }
//!
//!     fn handle_message(&mut self, m: Message) {
//!         log::info!("recv: id = {}", m.header().id);
//!     }
//! }
//! ```

use log::{Level, LevelFilter, Log, SetLoggerError};
use spin::RwLock;
use std::{
    fmt::Debug,
    io::{stderr, stdout},
    sync::{atomic::Ordering, Arc},
};
use termcolor::{BufferWriter, ColorChoice};

mod filter;
mod fmt;
mod output;
mod record;

pub use fmt::LogFormat;
pub use output::LogOutput;
pub use record::LogRecord;

use self::filter::FilterPolicy;
use crate::sync::AtomicUsize;
use crate::time::SimTime;

mod wrap {
    use super::Logger;
    use log::*;
    use spin::RwLock;

    // is truly accesed from multiple threads.
    pub(super) static SCOPED_LOGGER: LoggerWrap = LoggerWrap::uninitalized();

    pub(super) struct LoggerWrap {
        pub(super) inner: RwLock<Option<Logger>>, // inner: RwLock<Option<Logger>>,
    }

    impl LoggerWrap {
        pub(super) const fn uninitalized() -> Self {
            Self {
                inner: RwLock::new(None),
            }
        }

        pub(super) fn reset(&self) {
            self.inner.write().take();
            // *lock = None;
        }

        pub(super) fn reset_contents(&self, new: Logger) {
            // Check activ;
            let active = { self.inner.read().as_ref().unwrap().active };
            if active {
                self.inner.write().replace(new);
            }
        }

        pub(super) fn set(&self, other: Logger) -> Option<Logger> {
            self.inner.write().replace(other)
        }
    }

    impl Log for LoggerWrap {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            let lock = self.inner.read();
            lock.as_ref().unwrap().enabled(metadata)
        }

        fn log(&self, record: &log::Record) {
            let lock = self.inner.read();
            lock.as_ref().unwrap().log(record);
        }

        fn flush(&self) {
            let lock = self.inner.read();
            lock.as_ref().unwrap().flush();
        }
    }
}

/// A token describing a logger scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeToken(usize);

// is only set on the simulation thread, but read by all
// use static mut with a file-local saftey contract.
static CURRENT_SCOPE: AtomicUsize = AtomicUsize::new(usize::MAX);

/// A logger that collects scope specific messages.
pub struct Logger {
    active: bool,
    scopes: RwLock<Vec<LoggerScope>>,
    // use std lock to allways deal with multi-threaded access,
    // (e.g.) from other crates in the test chain
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
            scopes: RwLock::new(Vec::new()),
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
            scopes: RwLock::new(Vec::new()),
            active: true,
            policy: Box::new(DefaultPolicy),
            interal_max_level: LevelFilter::Warn,
            filter: FilterPolicy::new(false),
        }
    }

    ///
    /// Creates a new scope, reference by the given token.
    ///
    pub fn register_scope(scope: &str) -> ScopeToken {
        wrap::SCOPED_LOGGER
            .inner
            .read()
            .as_ref()
            .map(|v| v._register_scope(scope))
            .unwrap_or(ScopeToken(usize::MAX))
    }

    fn _register_scope(&self, scope: &str) -> ScopeToken {
        let (output, fmt) = self.policy.configure(scope);

        let new_scope = LoggerScope {
            scope: Arc::new(scope.to_string()),
            output,
            fmt,
            filter: self.filter.filter_for(scope, LevelFilter::max()),
        };

        let mut scopes = self.scopes.write();
        let token = ScopeToken(scopes.len());
        scopes.push(new_scope);

        token
    }

    /// Enters into a scope.
    pub fn enter_scope(scope: ScopeToken) {
        CURRENT_SCOPE.store(scope.0, Ordering::SeqCst);
    }

    /// Leaves a scope.
    pub fn leave_scope() {
        CURRENT_SCOPE.store(usize::MAX, Ordering::SeqCst);
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
        self.try_set_logger().expect("Failed to set logger");
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
        let old = wrap::SCOPED_LOGGER.set(self);
        match log::set_logger(&wrap::SCOPED_LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
        {
            Ok(()) => Ok(()),
            Err(e) => {
                // Since a logger was allready set it might either be a
                // ScopedLogger or somthing elsewhere
                // If SCOPED_LOGGER is Some that this logger is the set logger.

                if let Some(v) = old {
                    // Old was Scoped logger so keep the old logger and reset it.
                    let recently_created = wrap::SCOPED_LOGGER.set(v);
                    wrap::SCOPED_LOGGER.reset_contents(recently_created.unwrap());
                    Ok(())
                } else {
                    wrap::SCOPED_LOGGER.reset();
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

        // (2) Get scopes
        let mut scopes = self.scopes.write();

        // (3) Get target pointer
        let target_is_module_path = Some(record.metadata().target()) == record.module_path();
        let target_label = if target_is_module_path {
            String::new()
        } else {
            format!(" ({})", record.metadata().target())
        };

        // (4) Get scope or make defeault print based on the target marker.
        // SAFTEY:
        // the current scope cannot be seen in invalid states,
        // since begin_scope or end_scope only occures inbetween events
        // when no other threads are ative
        let scope_token = CURRENT_SCOPE.load(Ordering::SeqCst);
        if scope_token == usize::MAX {
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
        let scope = scopes.get_mut(scope_token);
        if let Some(scope) = scope {
            scope.log(format!("{}", record.args()), target_label, record.level());
        } else {
            todo!()
        }
    }

    fn flush(&self) {}
}

// SAFTEY:
// A logger does not contain any thread specific entries.
// so this type is sendable.
unsafe impl Send for Logger {}

// SAFTEY:
// Since all internal datapoints are either acessed by ownership
// or fields that are themself Sync.
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
            eprintln!("failed to write to logging output: {e}");
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
