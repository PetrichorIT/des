use std::fmt::Display;

use super::{GateId, MessageBody};
use crate::SimTime;

/// A runtime-unqiue identifier for a end-to-end connection.
pub type ConnectionId = usize;

/// A runtime-unique identifier for a one directional channel.
pub type ChannelId = usize;

/// A not defined channel aka a missing link.
pub const CHANNEL_NULL: ChannelId = 0;

/// A reference to other channel in a two directional configuration.
pub const CHANNEL_SELF: ChannelId = 1;

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
    pub fn calculate_duration<T: MessageBody>(&self, msg: &T) -> SimTime {
        let len = msg.bit_len();
        let transmission_time = len as f64 / self.bitrate as f64;
        if self.jitter == SimTime::ZERO {
            self.latency + transmission_time
        } else {
            todo!()
        }
    }

    pub fn calculate_busy<T: MessageBody>(&self, msg: &T) -> SimTime {
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

static mut CHANNEL_ID: ChannelId = 0xff;
fn register_channel() -> ChannelId {
    unsafe {
        let r = CHANNEL_ID;
        CHANNEL_ID += 1;
        r
    }
}

///
/// A representation of a one directional link.
///
#[derive(Debug)]
pub struct Channel {
    /// A unique identifier for a channel.
    pub id: ChannelId,

    /// The source of the channel.
    pub src: GateId,

    /// The connection bound to the channel.
    pub trg: GateId,

    /// The capabilities of the channel.
    pub metrics: ChannelMetrics,

    pub busy: bool,
}

impl Channel {
    pub fn new(src: GateId, trg: GateId, metrics: ChannelMetrics) -> Self {
        Self {
            id: register_channel(),
            src,
            trg,
            metrics,
            busy: false,
        }
    }

    pub fn calculate_duration<T: MessageBody>(&self, msg: &T) -> SimTime {
        self.metrics.calculate_duration(msg)
    }

    pub fn calculate_busy<T: MessageBody>(&self, msg: &T) -> SimTime {
        self.metrics.calculate_busy(msg)
    }
}
