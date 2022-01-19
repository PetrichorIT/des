use std::fmt::Display;

use crate::create_global_uid;

use crate::core::*;
use crate::net::*;
use crate::util::Indexable;

create_global_uid!(
    /// A runtime-unique identifier for a one directional channel.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ChannelId(usize) = CHANNEL_ID;
);

/// A not defined channel aka a missing link.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub const CHANNEL_NULL: ChannelId = ChannelId(0);

/// A reference to other channel in a two directional configuration.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub const CHANNEL_SELF: ChannelId = ChannelId(1);

/// The id of a general purpose non-delay channel
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub const CHANNEL_INSTANTANEOUS: ChannelId = ChannelId(2);

///
/// Metrics that define a channels capabilitites.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
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
    #[allow(clippy::if_same_then_else)]
    pub fn calculate_duration(&self, msg: &Message) -> SimTime {
        if self.bitrate == 0 {
            return SimTime::ZERO;
        }

        let len = msg.bit_len();
        let transmission_time = SimTime::from(len as f64 / self.bitrate as f64);
        if self.jitter == SimTime::ZERO {
            self.latency + transmission_time
        } else {
            // TODO: handle jitter this is just a yiuck fix to use NDL defaults
            self.latency + transmission_time
        }
    }

    ///
    /// Calculate the duration the channel is busy transmitting the
    /// message onto the channel.
    ///
    pub fn calculate_busy(&self, msg: &Message) -> SimTime {
        if self.bitrate == 0 {
            SimTime::ZERO
        } else {
            let len = msg.bit_len();
            SimTime::from(len as f64 / self.bitrate as f64)
        }
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
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub struct Channel {
    /// A unique identifier for a channel.
    id: ChannelId,

    /// The capabilities of the channel.
    metrics: ChannelMetrics,

    /// A indicator whether a channel is busy transmitting a packet.
    busy: bool,
}

impl Indexable for Channel {
    type Id = ChannelId;

    fn id(&self) -> Self::Id {
        self.id
    }
}

impl Channel {
    ///
    /// The capabilities of the channel.
    ///
    pub fn metrics(&self) -> &ChannelMetrics {
        &self.metrics
    }

    ///
    /// A indicator whether a channel is currently busy transmissting a
    /// packet onto the medium.
    ///
    /// Note that being non-busy does not mean that no packet is currently on the medium
    /// it just means that all bits have been put onto the medium.
    ///
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    ///
    /// Sets the busy state of an medium.
    ///
    pub fn set_busy(&mut self, busy_state: bool) {
        self.busy = busy_state
    }

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
