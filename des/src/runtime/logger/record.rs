use std::{fmt::Arguments, sync::Arc};

use log::Level;

use crate::time::SimTime;

/// A logging record.
#[derive(Debug)]
pub(super) struct LogRecord<'a> {
    /// The custom target if exisitent
    pub(super) target: String,
    /// The target of the log message.
    pub(super) scope: Arc<String>,
    /// The temporal origin point.
    pub(super) time: SimTime,
    /// The message formated with the std formater
    pub(super) msg: &'a Arguments<'a>,
    /// The original log level.
    pub(super) level: Level,
}
