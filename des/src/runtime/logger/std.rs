#![allow(dead_code)]

use log::*;
use std::cell::{Cell, RefCell};
use std::io::Write;
use termcolor::WriteColor;
use termcolor::*;

thread_local! {
    static CURRENT_OBJECT: RefCell<Option<String>> = const { RefCell::new(None) };

    static LOGGER_ACTIVE: Cell<bool> = const { Cell::new(true) };
    static LOGGER_PRIO_SCOPE: Cell<bool> = const { Cell::new(false) };
    static LOGGER_SHOW_NONPRIO_TARGET: Cell<bool> = const { Cell::new(true) };
}

/// A logger instance for internal logs.
pub(crate) static LOGGER: StandardLogger = StandardLogger();

/// The logging implementation used internally.
#[derive(Debug)]
pub struct StandardLogger();

impl StandardLogger {
    ///
    /// Creates a logger and registers it to the logging interface
    ///
    pub fn setup() -> Result<(), SetLoggerError> {
        match set_logger(&LOGGER).map(|()| set_max_level(LevelFilter::Trace)) {
            Ok(v) => Ok(v),
            Err(_e) => Ok(()),
        }
    }

    ///
    /// Configures the logger to prioritize either interal scopes
    /// or manually provided targets
    ///
    pub fn prioritise_internal_scopes_as_target(value: bool) {
        LOGGER_PRIO_SCOPE.with(|v| v.set(value))
    }

    ///
    /// Configures the logger to also show non-prioritesed targets
    /// in a appendix.
    ///
    pub fn show_nonpriority_target(value: bool) {
        LOGGER_SHOW_NONPRIO_TARGET.with(|v| v.set(value))
    }

    ///
    /// Manually overwrites the logger
    ///
    pub fn active(value: bool) {
        LOGGER_ACTIVE.with(|v| v.set(value))
    }

    pub(crate) fn begin_scope(ident: &str) {
        CURRENT_OBJECT.with(|c| *c.borrow_mut() = Some(ident.to_string()))
    }

    pub(crate) fn begin_scope_with_suffix(ident: &str, suffix: &str) {
        CURRENT_OBJECT.with(|c| *c.borrow_mut() = Some(format!("{}: {}", ident, suffix)))
    }

    pub(crate) fn end_scope() {
        CURRENT_OBJECT.with(|c| *c.borrow_mut() = None)
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
}

impl Log for StandardLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Trace && LOGGER_ACTIVE.with(|v| v.get())
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let (target, appx) = {
                if let Some(v) = CURRENT_OBJECT.with(|v| v.borrow().clone()) {
                    if v == record.target() || record.target().starts_with("des::") {
                        (v, None)
                    } else {
                        if LOGGER_PRIO_SCOPE.with(|v| v.get()) {
                            (v, Some(record.target().to_string()))
                        } else {
                            (record.target().to_string(), Some(v))
                        }
                    }
                } else {
                    (record.target().to_string(), None)
                }
            };

            let mut stream = match record.level() {
                Level::Error => StandardStream::stderr(termcolor::ColorChoice::Always),
                _ => StandardStream::stdout(termcolor::ColorChoice::Always),
            };

            stream
                .set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0x7f, 0x8c, 0x8d))))
                .expect("Failed to set termcolor");

            write!(&mut stream, "[ ").expect("Failed to write to stdout");

            stream
                .set_color(
                    ColorSpec::new().set_fg(Some(StandardLogger::get_level_color(record.level()))),
                )
                .expect("Failed to set termcolor");

            write!(&mut stream, "{:>25}", target).expect("Failed to write to stdout");

            stream
                .set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0x7f, 0x8c, 0x8d))))
                .expect("Failed to set termcolor");

            write!(&mut stream, " ] ").expect("Failed to write to stdout");

            stream.reset().expect("Failed to reset termcolor");

            write!(&mut stream, "{}", record.args()).expect("Failed to write to stdout");

            if LOGGER_SHOW_NONPRIO_TARGET.with(|v| v.get()) {
                if let Some(appx) = appx {
                    write!(&mut stream, " ({})", appx).expect("Failed to write to stdout");
                }
            }

            writeln!(&mut stream).expect("Failed to write")
        }
    }

    fn flush(&self) {}
}
