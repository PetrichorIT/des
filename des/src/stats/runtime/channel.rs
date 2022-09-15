use std::time::Duration;

use crate::net::{ChannelMetrics, Message, ObjectPath};
use crate::stats::SlottedActivityTimeline;

/// Defines the slot size used for activity timelines of
/// channels.
pub const CHANNEL_HIST_SLOTTING: Duration = Duration::new(5, 0);

///
/// A statistical item attached to a channel.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InProgressChannelStats {
    num_messages_passed: usize,
    num_messages_dropped: usize,

    num_bytes_passed: usize,
    num_bytes_dropped: usize,

    busy_time: Duration,
    busy_hist: SlottedActivityTimeline,

    channel: ObjectPath,
    metrics: ChannelMetrics,
}

impl InProgressChannelStats {
    ///
    /// Creates a new instance of [`ChannelStats`].
    ///
    pub(crate) fn new(channel: ObjectPath, metrics: ChannelMetrics) -> Self {
        Self {
            channel,
            metrics,

            num_messages_passed: 0,
            num_messages_dropped: 0,

            num_bytes_passed: 0,
            num_bytes_dropped: 0,

            busy_time: Duration::ZERO,
            busy_hist: SlottedActivityTimeline::new(CHANNEL_HIST_SLOTTING),
        }
    }

    pub(crate) fn evaluate(&self, duration: Duration) -> ChannelStats {
        let mut result = ChannelStats {
            num_messages_passed: self.num_messages_passed,
            num_messages_droppped: self.num_messages_dropped,

            num_bytes_passed: self.num_bytes_passed,
            num_bytes_dropped: self.num_bytes_dropped,

            busy_perc: self.busy_time.as_secs_f64() / duration.as_secs_f64(),
            busy_time: self.busy_time,
            busy_hist: self.busy_hist.clone(),

            channel: self.channel.clone(),
        };

        result.busy_hist.finish();

        result
    }

    pub(crate) fn register_message_passed(&mut self, msg: &Message) {
        self.num_messages_passed += 1;
        self.num_bytes_passed += msg.length() as usize;

        let busy = self.metrics.calculate_busy(msg);
        self.busy_time += busy;

        self.busy_hist.record_activity(busy, 1.0);
    }

    pub(crate) fn register_message_dropped(&mut self, msg: &Message) {
        self.num_messages_dropped += 1;
        self.num_bytes_dropped += msg.length() as usize;
    }
}

///
/// Statistics for a specific channel.
///
#[derive(Debug)]
#[allow(missing_docs)]
pub struct ChannelStats {
    pub num_messages_passed: usize,
    pub num_messages_droppped: usize,

    pub num_bytes_passed: usize,
    pub num_bytes_dropped: usize,

    pub busy_time: Duration,
    pub busy_perc: f64,
    pub busy_hist: SlottedActivityTimeline,

    pub channel: ObjectPath,
}

#[allow(missing_docs)]
impl ChannelStats {
    #[must_use]
    pub fn total_messages(&self) -> usize {
        self.num_messages_passed + self.num_messages_droppped
    }

    #[must_use]
    pub fn message_pass_probability(&self) -> f64 {
        self.num_messages_passed as f64 / self.total_messages() as f64
    }

    #[must_use]
    pub fn total_bytes(&self) -> usize {
        self.num_bytes_passed + self.num_bytes_dropped
    }

    #[must_use]
    pub fn byte_pass_probability(&self) -> f64 {
        self.num_bytes_passed as f64 / self.total_bytes() as f64
    }
}
