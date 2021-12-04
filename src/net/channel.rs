use std::fmt::Display;

use global_uid::GlobalUID;

use crate::{Message, SimTime};

/// A runtime-unqiue identifier for a end-to-end connection.
pub type ConnectionId = usize;

/// A runtime-unique identifier for a one directional channel.
#[derive(GlobalUID)]
#[repr(transparent)]
pub struct ChannelId(usize);

/// A not defined channel aka a missing link.
pub const CHANNEL_NULL: ChannelId = ChannelId(0);

/// A reference to other channel in a two directional configuration.
pub const CHANNEL_SELF: ChannelId = ChannelId(1);

/// The id of a general purpose non-delay channel
pub const CHANNEL_INSTANTANEOUS: ChannelId = ChannelId(2);

///
/// Metrics that define a channels capabilitites.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMetrics {
    /// The maximum throughput of the channel in bit/s
    pub bitrate: usize,
    /// The latency a message endures while transversing a channel.
    pub latency: SimTime,
    /// The variance in latency.
    pub jitter: SimTime,
}

impl ChannelMetrics {
    ///
    /// A channel metric that does not take up time.
    ///
    pub const INSTANTANEOUS: ChannelMetrics = ChannelMetrics {
        bitrate: 0,
        latency: SimTime::ZERO,
        jitter: SimTime::ZERO,
    };

    ///
    /// Creates a new instance of channel metrics.
    ///
    pub fn new(bitrate: usize, latency: SimTime, jitter: SimTime) -> Self {
        Self {
            bitrate,
            latency,
            jitter,
        }
    }

    ///
    /// Calcualtes the duration a message travels on a link.
    ///
    pub fn calculate_duration(&self, msg: &Message) -> SimTime {
        if self.bitrate == 0 {
            return SimTime::ZERO;
        }

        let len = msg.bit_len();
        let transmission_time = len as f64 / self.bitrate as f64;
        if self.jitter == SimTime::ZERO {
            self.latency + transmission_time
        } else {
            todo!()
        }
    }

    ///
    /// Calculate the duration the channel is busy transmitting the
    /// message onto the channel.
    ///
    pub fn calculate_busy(&self, msg: &Message) -> SimTime {
        let len = msg.bit_len();
        SimTime::new(len as f64 / self.bitrate as f64)
    }
}

impl Display for ChannelMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelMetrics")
            .field("bitrate (bit/s)", &self.bitrate)
            .field("lateny", &self.latency)
            .field("jitter", &self.jitter)
            .finish()
    }
}

///
/// A representation of a one directional link.
///
#[derive(Debug)]
pub struct Channel {
    /// A unique identifier for a channel.
    pub id: ChannelId,

    /// The capabilities of the channel.
    pub metrics: ChannelMetrics,

    /// A indicator whether a channel is busy transmitting a packet.
    pub busy: bool,
}

impl Channel {
    ///
    /// A channel metric that does not take up time.
    ///
    pub const INSTANTANEOUS: Channel = Channel {
        id: CHANNEL_INSTANTANEOUS,
        metrics: ChannelMetrics::INSTANTANEOUS,
        busy: false,
    };

    ///
    /// Creates a new channel using tthe given metrics.
    ///
    pub fn new(metrics: ChannelMetrics) -> Self {
        Self {
            id: ChannelId::gen(),
            metrics,
            busy: false,
        }
    }

    ///
    /// Calcualtes the packet travel duration using the
    /// underlying metric.
    ///
    pub fn calculate_duration(&self, msg: &Message) -> SimTime {
        self.metrics.calculate_duration(msg)
    }

    ///
    /// Calcualtes the busy time of the channel using
    /// the underlying metric.
    ///
    pub fn calculate_busy(&self, msg: &Message) -> SimTime {
        self.metrics.calculate_busy(msg)
    }
}
