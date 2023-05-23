use std::io::{Result, Stdout};

use termcolor::{BufferWriter, ColorChoice};
use tracing::Event;

use crate::prelude::SimTime;

use super::format::TracingFormatter;

pub struct TracingRecord<'a> {
    pub time: SimTime,
    pub scope: Option<&'a str>,
    pub target: Option<&'a str>,
    pub spans: &'a [&'a str],
    pub event: &'a Event<'a>,
}

/// Describes an object that can be used as a output medium for the logger.
pub trait TracingOutput {
    /// Writes a log record to the output target using the
    /// provided format if possible.
    ///
    /// This function may fail if the underlying target cannot be
    /// written.
    ///
    /// # Errors
    ///
    /// Returns the raw error that occured on the underlying target.
    ///
    fn write(&mut self, fmt: &mut dyn TracingFormatter, record: TracingRecord<'_>) -> Result<()>;
}

impl TracingOutput for Stdout {
    fn write(&mut self, fmt: &mut dyn TracingFormatter, record: TracingRecord<'_>) -> Result<()> {
        let writer = BufferWriter::stdout(ColorChoice::Always);
        let mut buffer = writer.buffer();
        fmt.fmt(&mut buffer, record)?;
        writer.print(&buffer)
    }
}
