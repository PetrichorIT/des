use log::*;
use std::io::Write;
use termcolor::WriteColor;
use termcolor::*;

/// A logger instance for internal logs.
pub static LOGGER: StandardLogger = StandardLogger();

/// The logging implementation used internally.
#[derive(Debug)]
pub struct StandardLogger();

impl StandardLogger {
    ///
    /// Creates a logger and registers it to the logging interface
    ///
    pub fn setup() -> Result<(), SetLoggerError> {
        set_logger(&LOGGER).map(|()| set_max_level(LevelFilter::Trace))
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
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
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

            write!(&mut stream, "{:>25}", record.target()).expect("Failed to write to stdout");

            stream
                .set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0x7f, 0x8c, 0x8d))))
                .expect("Failed to set termcolor");

            write!(&mut stream, " ] ").expect("Failed to write to stdout");

            stream.reset().expect("Failed to reset termcolor");

            writeln!(&mut stream, "{}", record.args()).expect("Failed to write to stdout");
        }
    }

    fn flush(&self) {}
}
