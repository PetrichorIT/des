use log::*;
use std::io::Write;
use termcolor::WriteColor;
use termcolor::*;

/// A logger instance for internal logs from [des].
pub static LOGGER: StandardLogger = StandardLogger();

// const MODULE_COLORS: [Color; 8] = [
//     Color::Rgb(0x2e, 0xcc, 0x71),
//     Color::Rgb(0x29, 0x80, 0xb9),
//     Color::Rgb(0x8e, 0x44, 0xad),
//     Color::Rgb(0xf1, 0xc4, 0x0f),
//     Color::Rgb(0xd3, 0x54, 0x00),
//     Color::Rgb(0xe7, 0x4c, 0x3c),
//     Color::Rgb(0x1a, 0xbc, 0x9c),
//     Color::Rgb(0x34, 0x49, 0x5e),
// ];

// fn get_random_color(str: &str) -> Color {
//     let mut sum = 0;
//     for byte in str.chars() {
//         sum += byte as usize;
//     }

//     MODULE_COLORS[sum % 8]
// }

/// The logging implementation used by [des] internally.
pub struct StandardLogger();

impl StandardLogger {
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
