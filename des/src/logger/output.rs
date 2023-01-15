use std::{
    fs::File,
    io::{BufWriter, Result, Stderr, Stdout, Write},
};

use log::Level::*;
use termcolor::*;

use super::{record::LogRecord, LogFormat};

/// Describes an object that can be used as a output medium for the logger.
pub trait LogOutput {
    /// Writes a log record to the output target using the
    /// provided format if possible.
    ///
    /// This function may fail if the underlying target cannot be
    /// written.
    fn write(&mut self, record: &LogRecord, fmt: LogFormat) -> Result<()>;
}

impl LogOutput for [Box<dyn LogOutput>] {
    fn write(&mut self, record: &LogRecord, fmt: LogFormat) -> Result<()> {
        for element in self {
            element.write(record, fmt)?;
        }
        Ok(())
    }
}

impl LogOutput for File {
    fn write(&mut self, record: &LogRecord, _fmt: LogFormat) -> Result<()> {
        let bufwrt = BufferWriter::stdout(ColorChoice::Never);
        let mut buffer = bufwrt.buffer();
        LogFormat::NoColor.fmt(record, &mut buffer)?;
        self.write_all(&buffer.as_slice())?;
        Ok(())
    }
}

impl LogOutput for BufWriter<File> {
    fn write(&mut self, record: &LogRecord, _fmt: LogFormat) -> Result<()> {
        let bufwrt = BufferWriter::stdout(ColorChoice::Never);
        let mut buffer = bufwrt.buffer();
        LogFormat::NoColor.fmt(record, &mut buffer)?;
        self.write_all(&buffer.as_slice())?;
        Ok(())
    }
}

impl LogOutput for (Stdout, Stderr) {
    fn write(&mut self, record: &LogRecord, fmt: LogFormat) -> Result<()> {
        let stream = match record.level {
            Error | Warn => BufferWriter::stderr(ColorChoice::Always),
            Info | Debug | Trace => BufferWriter::stdout(ColorChoice::Always),
        };
        let mut buffer = stream.buffer();
        fmt.fmt(record, &mut buffer)?;
        stream.print(&buffer)?;

        Ok(())
    }
}

impl LogOutput for Vec<String> {
    fn write(&mut self, record: &LogRecord, _fmt: LogFormat) -> Result<()> {
        let bufwrt = BufferWriter::stdout(ColorChoice::Never);
        let mut buffer = bufwrt.buffer();
        LogFormat::NoColor.fmt(record, &mut buffer)?;
        let string = String::from_utf8_lossy(buffer.as_slice()).into_owned();
        self.push(string);
        Ok(())
    }
}

impl LogOutput for () {
    fn write(&mut self, _record: &LogRecord, _fmt: LogFormat) -> Result<()> {
        Ok(())
    }
}
