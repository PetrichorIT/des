use std::sync::Arc;

use log::Level;

use crate::time::SimTime;

/// A logging record.
#[derive(Debug)]
pub struct LogRecord {
    /// The target of the log message.
    pub target: Arc<String>,
    /// The temporal origin point.
    pub time: SimTime,
    /// The message formated with the std formater
    pub msg: String,
    /// The original log level.
    pub level: Level,
}
