use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    fmt::Debug,
    io::{stderr, stdout},
    sync::Arc,
};

mod env;
mod fmt;
mod output;
mod record;

pub use fmt::LogFormat;
pub use record::LogRecord;
pub use output::LogOutput;

use crate::time::SimTime;
use log::{Level, LevelFilter, Log, SetLoggerError};
use termcolor::{ColorChoice, BufferWriter};

use self::env::LogEnvOptions;

static SCOPED_LOGGER: LoggerWrap = LoggerWrap::uninitalized();

thread_local! {
    pub(crate) static CURRENT_SCOPE: RefCell<Option<&'static str>> = const { RefCell::new(None) }
}

struct LoggerWrap {
    inner: UnsafeCell<Option<Logger>>,
}

impl LoggerWrap {
    const fn uninitalized() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    fn reset(&self) {
        let old_inner = unsafe { &mut *self.inner.get() };
        *old_inner = None;
    }

    fn reset_contents(&self, new: Logger) {
        let old_inner = unsafe { &mut *self.inner.get() };
        old_inner.as_mut().unwrap().reset_contents(new);
    }

    fn set(&self, inner: Logger) -> Option<Logger> {
        let old_inner = unsafe { &mut *self.inner.get() };
        old_inner.replace(inner)
    }
}

impl Log for LoggerWrap {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        unsafe {
            let inner = &*self.inner.get();
            inner.as_ref().unwrap_unchecked().enabled(metadata)
        }
    }

    fn log(&self, record: &log::Record) {
        unsafe {
            let inner = &*self.inner.get();
            inner.as_ref().unwrap_unchecked().log(record);
        }
    }

    fn flush(&self) {
        unsafe {
            let inner = &*self.inner.get();
            inner.as_ref().unwrap_unchecked().flush();
        }
    }
}

unsafe impl Send for LoggerWrap {}
unsafe impl Sync for LoggerWrap {}


/// A logger that collects scope specific messages.
pub struct Logger {
    active: bool,
    scopes: RefCell<HashMap<String, LoggerScope>>,
    policy: Box<dyn LogScopeConfigurationPolicy>,
    interal_max_level: LevelFilter,
    env: LogEnvOptions,
}


/// A policy object.

#[cfg(debug_assertions)]
#[doc(hidden)]
pub trait LogScopeConfigurationPolicy {
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
impl LogScopeConfigurationPolicy for DefaultPolicy{
    fn configure(&self, _scope: &str) -> (Box<dyn LogOutput>, LogFormat) {
        (Box::new((stdout(), stderr())), LogFormat::ColorFull)
    }
}

struct QuietPolicy;
impl LogScopeConfigurationPolicy for QuietPolicy{
    fn configure(&self, _scope: &str) -> (Box<dyn LogOutput>, LogFormat) {
        (Box::new(()), LogFormat::ColorFull)
    }
}


impl Logger {
    /// Creates a new Logger (builder).
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: RefCell::new(HashMap::new()),
            active: true,
            policy: Box::new(DefaultPolicy),
            interal_max_level: LevelFilter::Warn,
            env: LogEnvOptions::new(),
        }
    }

    /// A logger that does not forward logs to stdout or stderr
    #[must_use]
    pub fn quiet() -> Self {
        Self {
            scopes: RefCell::new(HashMap::new()),
            active: true,
            policy: Box::new(QuietPolicy),
            interal_max_level: LevelFilter::Warn,
            env: LogEnvOptions::new(),
        }
    }

    /// Begins a new scope, returning the currently active scope.
    #[doc(hidden)]
    pub fn begin_scope(ident: impl AsRef<str>) {
        let ident: *const str = ident.as_ref();
        let ident: &'static str = unsafe { &*ident };
        CURRENT_SCOPE.with(|cell| cell.replace(Some(ident)));
    }

    /// Removes the current scope.
    #[doc(hidden)]
    pub fn end_scope() {
        CURRENT_SCOPE.with(|cell| cell.replace(None));
    }


    fn reset_contents(&mut self, new: Self) {
        if self.active {
            *self = new;
        }
    }

    /// Sets the loggers activity status.
    #[must_use]
    pub fn active(mut self, is_active: bool) -> Self {
        self.active = is_active;
        self
    }

    /// Set the policy that dicates whether to forward messages to stdout
    #[must_use]
    pub fn policy(mut self, predicate: Box<LogScopeConfigurationPolicyFn>) -> Self {
        self.policy= Box::new(predicate);
        self
    }

    /// Set the policy that dicates whether to forward messages to stdout
    #[must_use]
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn policy_object(mut self, object: impl LogScopeConfigurationPolicy + 'static) -> Self {
        self.policy = Box::new(object);
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
            Ok(()) => {
                Ok(())
            }
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

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

impl Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScopedLogger {{ ... }}")
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
        let Ok(mut scopes) = self.scopes.try_borrow_mut() else {
            // TODO:
            // If multiple accesses are happening, this is due to third party tools.
            // For now this remains unhandled, since a mutex would be an overcommitment 
            // for single threaded cases.
            // Once rt-multi-thread is active, use a std::sync::mutex to solve this problem.
            return
        };

        // (3) Get target pointer
        let target_is_module_path = Some(record.metadata().target()) == record.module_path();
        let target_label = if target_is_module_path {
            String::new()
        } else {
            format!(" ({})", record.metadata().target())
        };

        // (4) Get scope or make defeault print based on the target marker.
        let Some(scope_label) = CURRENT_SCOPE.with(|c| *c.borrow()) else {
            

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

                LogFormat::fmt(LogFormat::ColorFull, &record, &mut buffer)
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
                filter: self.env.level_filter_for(scope_label),
            };

            new_scope.log(format!("{}", record.args()), target_label, record.level());

            scopes.insert(scope_label.to_string(), new_scope);
        }
    }

    fn flush(&self) {}
}

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
        f.debug_struct("LoggerScope").field("scope", &self.scope).field("fmt", &self.fmt).field("filter", &self.filter).finish_non_exhaustive()
    }
}