use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    fmt::{Arguments, Debug},
    sync::Arc,
};

mod env;
mod fmt;
mod record;

pub use fmt::LogFormat;
use record::LogRecord;

use crate::time::SimTime;
use log::{Level, LevelFilter, Log, SetLoggerError};
use termcolor::{ColorChoice, StandardStream};

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

    fn yield_scopes(&self) -> HashMap<String, LoggerScope> {
        let inner = unsafe { &mut *self.inner.get() };
        let scopes = &inner
            .as_mut()
            .expect("Failed to yield logger scopes since no logger has been set")
            .scopes;
        let scopes = unsafe { &mut *scopes.get() };
        let mut repacement = HashMap::new();
        std::mem::swap(scopes, &mut repacement);

        repacement
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
    scopes: UnsafeCell<HashMap<String, LoggerScope>>,

    stdout_policy: Box<dyn Fn(&str) -> bool>,
    stderr_policy: Box<dyn Fn(&str) -> bool>,

    interal_max_level: LevelFilter,
    env: LogEnvOptions,
}

impl Logger {
    /// Creates a new Logger (builder).
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: UnsafeCell::new(HashMap::new()),
            active: true,

            stdout_policy: Box::new(|_| true),
            stderr_policy: Box::new(|_| true),

            interal_max_level: LevelFilter::Trace,

            env: LogEnvOptions::new(),
        }
    }

    /// A logger that does not forward logs to stdout or stderr
    #[must_use]
    pub fn quiet() -> Self {
        Self {
            scopes: UnsafeCell::new(HashMap::new()),
            active: true,

            stdout_policy: Box::new(|_| false),
            stderr_policy: Box::new(|_| false),

            interal_max_level: LevelFilter::Trace,

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

    /// Yields all scopes.
    pub fn yield_scopes() -> HashMap<String, LoggerScope> {
        SCOPED_LOGGER.yield_scopes()
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
    pub fn stdout_policy(mut self, predicate: &'static dyn Fn(&str) -> bool) -> Self {
        self.stdout_policy = Box::new(predicate);
        self
    }

    /// Set the policy that dicates whether to forward messages to stderr
    #[must_use]
    pub fn stderr_policy(mut self, predicate: &'static dyn Fn(&str) -> bool) -> Self {
        self.stderr_policy = Box::new(predicate);
        self
    }

    /// Sets the internal max level for all log message coming from internals
    #[must_use]
    pub fn interal_max_log_level(mut self, level: LevelFilter) -> Self {
        self.interal_max_level = level;
        self
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
    pub fn finish(self) -> Result<(), SetLoggerError> {
        let old = SCOPED_LOGGER.set(self);
        match log::set_logger(&SCOPED_LOGGER).map(|()| log::set_max_level(LevelFilter::Trace)) {
            Ok(()) => {
                // assert!(
                //     old.is_none(),
                //     "No logger was initialized, but vacant logger was found."
                // );
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
        let scopes = unsafe { &mut *self.scopes.get() };

        // (3) Get target pointer
        let target_is_module_path = Some(record.metadata().target()) == record.module_path();
        let target_label = if target_is_module_path {
            String::new()
        } else {
            format!(" ({})", record.metadata().target())
        };

        // (4) Get scope or make defeault print based on the target marker.
        let Some(scope_label) = CURRENT_SCOPE.with(|c| *c.borrow()) else {
            let policy = match record.level() {
                Level::Error | Level::Warn => &self.stderr_policy,
                _ => &self.stdout_policy,
            };

            if policy(record.target()) {
                // No target scope was given --- not scoped println.
                let out = match record.level() {
                    Level::Error | Level::Warn => StandardStream::stderr(ColorChoice::Always),
                    _ => StandardStream::stdout(ColorChoice::Always),
                };

                let record = LogRecord {
                    scope: Arc::new(record.target().to_string()),
                    target: target_label,
                    level: record.level(),
                    time: SimTime::now(),
                    msg: record.args(),
                };

                LogFormat::fmt(LogFormat::ColorFull, &record, out).expect("Failed to write record to output stream");
                // record.log(out);
            }
            return;
        };

        // (5) Fetch scope information
        let scope = scopes.get_mut(scope_label);
        if let Some(scope) = scope {
            scope.log(record.args(), target_label, record.level());
        } else {
            // TODO: Check target validity
            let stdout = &self.stdout_policy;
            let stderr = &self.stderr_policy;

            let mut new_scope = LoggerScope {
                target: Arc::new(scope_label.to_string()),
                fwd_stdout: stdout(record.target()),
                fwd_stderr: stderr(record.target()),
                filter: self.env.level_filter_for(scope_label),
            };

            new_scope.log(record.args(), target_label, record.level());

            let scopes = unsafe { &mut *self.scopes.get() };
            scopes.insert(scope_label.to_string(), new_scope);
        }
    }

    fn flush(&self) {}
}

/// A collection of all logging activity in one scope.
#[derive(Debug)]
pub struct LoggerScope {
    /// The target identifier for the current logger.
    target: Arc<String>,
    fwd_stdout: bool,
    fwd_stderr: bool,
    filter: LevelFilter,
}

impl LoggerScope {
    fn log(&mut self, msg: &Arguments, target: String, level: Level) {
        if level > self.filter {
            return;
        }

        let out = match level {
            Level::Error | Level::Warn if self.fwd_stderr => {
                Some(StandardStream::stderr(ColorChoice::Always))
            }
            Level::Info | Level::Debug | Level::Trace if self.fwd_stdout => {
                Some(StandardStream::stdout(ColorChoice::Always))
            }
            _ => None,
        };

        let record = LogRecord {
            msg,
            level,
            time: SimTime::now(),
            scope: self.target.clone(),
            target,
        };
        if let Some(out) = out {
            LogFormat::fmt(LogFormat::ColorFull, &record, out)
                .expect("Failed to write record to output stream");
        }
    }
}
