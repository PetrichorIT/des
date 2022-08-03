use std::{
    cell::{RefCell, UnsafeCell},
    collections::{HashMap, LinkedList},
    fmt::Debug,
    io::Write,
};

use log::{Level, LevelFilter, Log, SetLoggerError};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

static SCOPED_LOGGER: ScopedLoggerWrap = ScopedLoggerWrap::uninitalized();
thread_local! {
    pub(crate) static CURRENT_SCOPE: RefCell<Option<String>> = const { RefCell::new(None) }
}

struct ScopedLoggerWrap {
    inner: UnsafeCell<Option<ScopedLogger>>,
}

impl ScopedLoggerWrap {
    const fn uninitalized() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    fn set(&self, inner: ScopedLogger) {
        let old_inner = unsafe { &mut *self.inner.get() };
        *old_inner = Some(inner);
    }
}

impl Log for ScopedLoggerWrap {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        unsafe {
            let inner = &*self.inner.get();
            inner.as_ref().unwrap_unchecked().enabled(metadata)
        }
    }

    fn log(&self, record: &log::Record) {
        unsafe {
            let inner = &*self.inner.get();
            inner.as_ref().unwrap_unchecked().log(record)
        }
    }

    fn flush(&self) {
        unsafe {
            let inner = &*self.inner.get();
            inner.as_ref().unwrap_unchecked().flush()
        }
    }
}

unsafe impl Send for ScopedLoggerWrap {}
unsafe impl Sync for ScopedLoggerWrap {}

/// A logger that collects scope specific messages.
pub struct ScopedLogger {
    active: bool,
    scopes: UnsafeCell<HashMap<String, LoggerScope>>,
    stdout_policy: Box<dyn Fn(&str) -> bool>,
    stderr_policy: Box<dyn Fn(&str) -> bool>,
    interal_max_level: LevelFilter,
}

#[allow(unused)]
impl ScopedLogger {
    /// Creates a new Logger (builder).
    pub fn new() -> Self {
        Self {
            scopes: UnsafeCell::new(HashMap::new()),
            active: true,

            stdout_policy: Box::new(|_| true),
            stderr_policy: Box::new(|_| true),

            interal_max_level: LevelFilter::Trace,
        }
    }

    /// Begins a new scope, returning the currently active scope.
    #[doc(hidden)]
    pub fn begin_scope(ident: impl AsRef<str>) -> Option<String> {
        let ident = Some(ident.as_ref().to_string());
        CURRENT_SCOPE.with(|cell| cell.replace(ident))
    }

    /// Removes the current scope.
    #[doc(hidden)]
    pub fn end_scope() {
        CURRENT_SCOPE.with(|cell| cell.replace(None));
    }

    /// Sets the loggers activity status.
    pub fn active(mut self, is_active: bool) -> Self {
        self.active = is_active;
        self
    }

    /// Set the policy that dicates whether to forward messages to stdout
    pub fn stdout_policy(mut self, predicate: &'static dyn Fn(&str) -> bool) -> Self {
        self.stdout_policy = Box::new(predicate);
        self
    }

    /// Set the policy that dicates whether to forward messages to stderr
    pub fn stderr_policy(mut self, predicate: &'static dyn Fn(&str) -> bool) -> Self {
        self.stderr_policy = Box::new(predicate);
        self
    }

    /// Sets the internal max level for all log message coming from internals
    pub fn interal_max_log_level(mut self, level: LevelFilter) -> Self {
        self.interal_max_level = level;
        self
    }

    /// Connects the logger to the logging framework.
    pub fn finish(self) -> Result<(), SetLoggerError> {
        SCOPED_LOGGER.set(self);
        log::set_logger(&SCOPED_LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
    }
}

unsafe impl Send for ScopedLogger {}
unsafe impl Sync for ScopedLogger {}

impl Debug for ScopedLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScopedLogger {{ ... }}")
    }
}

impl Log for ScopedLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.active && metadata.level() <= LevelFilter::Trace
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let source_is_internal = record
            .module_path()
            .map(|s| s.starts_with("des"))
            .unwrap_or(false);
        if source_is_internal && record.level() > self.interal_max_level {
            return;
        }

        let scopes = unsafe { &mut *self.scopes.get() };

        let target_is_module_path = Some(record.metadata().target()) == record.module_path();
        let target = if target_is_module_path {
            if let Some(v) = CURRENT_SCOPE.with(|c| c.borrow().clone()) {
                v
            } else {
                // No target scope was given --- not scoped println.
                let mut out = match record.level() {
                    Level::Error | Level::Warn => StandardStream::stderr(ColorChoice::Always),
                    _ => StandardStream::stdout(ColorChoice::Always),
                };

                out.set_color(ColorSpec::new().set_fg(Some(PARENS_COLOR.clone())))
                    .expect("Failed to set color on output stream");

                write!(&mut out, "[ ").expect("Failed to write to output stream");

                out.set_color(ColorSpec::new().set_fg(Some(get_level_color(record.level()))))
                    .expect("Failed to set color on output stream");

                write!(&mut out, "{:>25}", record.target())
                    .expect("Failed to write to output stream");

                out.set_color(ColorSpec::new().set_fg(Some(PARENS_COLOR.clone())))
                    .expect("Failed to set color on output stream");

                write!(&mut out, " ] ").expect("Failed to write to output stream");

                out.reset().expect("Failed to reset output stream");

                writeln!(&mut out, "{}", record.args()).expect("Failed to write to output stream");
                return;
            }
        } else {
            record.target().to_string()
        };

        let scope = scopes.get_mut(&target);
        if let Some(scope) = scope {
            let text = format!("{}", record.args());
            scope.log(text, record.level())
        } else {
            // TODO: Check target validity
            let stdout = &self.stdout_policy;
            let stderr = &self.stderr_policy;

            let mut new_scope = LoggerScope {
                target,
                stream: LinkedList::new(),
                fwd_stdout: stdout(record.target()),
                fwd_stderr: stderr(record.target()),
            };

            new_scope.log(format!("{}", record.args()), record.level());

            let scopes = unsafe { &mut *self.scopes.get() };
            scopes.insert(record.target().to_string(), new_scope);
        }
    }

    fn flush(&self) {}
}

/// A collection of all logging activity in one scope.
#[derive(Debug)]
pub struct LoggerScope {
    /// The target identifier for the current logger.
    pub target: String,
    /// The stream of logs.
    pub stream: LinkedList<LoggerRecord>,
    fwd_stdout: bool,
    fwd_stderr: bool,
}

const PARENS_COLOR: Color = Color::Rgb(0x7f, 0x8c, 0x8d);

impl LoggerScope {
    fn log(&mut self, msg: String, level: Level) {
        let out = match level {
            Level::Error | Level::Warn if self.fwd_stderr => {
                Some(StandardStream::stderr(ColorChoice::Always))
            }
            Level::Info | Level::Debug | Level::Trace if self.fwd_stdout => {
                Some(StandardStream::stdout(ColorChoice::Always))
            }
            _ => None,
        };
        if let Some(mut out) = out {
            out.set_color(ColorSpec::new().set_fg(Some(PARENS_COLOR.clone())))
                .expect("Failed to set color on output stream");

            write!(&mut out, "[ ").expect("Failed to write to output stream");

            out.set_color(ColorSpec::new().set_fg(Some(get_level_color(level))))
                .expect("Failed to set color on output stream");

            write!(&mut out, "{:>25}", self.target).expect("Failed to write to output stream");

            out.set_color(ColorSpec::new().set_fg(Some(PARENS_COLOR.clone())))
                .expect("Failed to set color on output stream");

            write!(&mut out, " ] ").expect("Failed to write to output stream");

            out.reset().expect("Failed to reset output stream");

            writeln!(&mut out, "{}", msg).expect("Failed to write to output stream");
        }

        self.stream.push_back(LoggerRecord { msg, level });
    }
}

/// A logging record.
#[derive(Debug)]
pub struct LoggerRecord {
    /// The message formated with the std formater
    pub msg: String,
    /// The original log level.
    pub level: Level,
}

fn get_level_color(level: Level) -> Color {
    match level {
        Level::Debug => Color::Cyan,
        Level::Trace => Color::Magenta,
        Level::Info => Color::Green,
        Level::Warn => Color::Yellow,
        Level::Error => Color::Red,
    }
}
