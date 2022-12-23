use log::Level;
use std::io::Write;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use super::LogRecord;

/// Defines the output type of a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogFormat {
    /// Outputs records using ANSI color code for a terminal.
    ColorFull,
    /// Outputs records in a only ASCII format for storage in files.
    FileOutput,
}

const PARENS_COLOR: Color = Color::Rgb(0x7f, 0x8c, 0x8d);

impl LogFormat {
    pub(super) fn fmt(self, record: &LogRecord, mut out: StandardStream) {
        match self {
            Self::ColorFull => {
                out.set_color(ColorSpec::new().set_fg(Some(PARENS_COLOR)))
                    .expect("Failed to set color on output stream");

                write!(&mut out, "[ ").expect("Failed to write to output stream");

                // [ time ... target ] max 10 max 14
                let time = format!("{}", record.time);
                write!(&mut out, "{time:^5}").expect("Failed to write to output stream");
                write!(&mut out, " ] ").expect("Failed to write to output stream");

                out.set_color(ColorSpec::new().set_fg(Some(get_level_color(record.level))))
                    .expect("Failed to set color on output stream");

                write!(&mut out, "{}: ", record.target).expect("Failed to write to output stream");

                out.reset().expect("Failed to reset output stream");

                writeln!(&mut out, "{}", record.msg).expect("Failed to write to output stream");
            }
            Self::FileOutput => {
                write!(&mut out, "[ ").expect("Failed to write to output stream");
                // [ time ... target ] max 10 max 14
                let time = format!("{}", record.time);
                write!(&mut out, "{time:^5}").expect("Failed to write to output stream");
                write!(&mut out, " ] ").expect("Failed to write to output stream");
                write!(&mut out, "{}: ", record.target).expect("Failed to write to output stream");
                writeln!(&mut out, "{}", record.msg).expect("Failed to write to output stream");
            }
        }
    }
}

fn get_level_color(level: Level) -> Color {
    match level {
        Level::Debug => Color::Magenta,
        Level::Trace => Color::Cyan,
        Level::Info => Color::Green,
        Level::Warn => Color::Yellow,
        Level::Error => Color::Red,
    }
}
