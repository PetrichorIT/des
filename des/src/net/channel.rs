use std::fmt::Display;

use rand::distributions::Uniform;
use rand::prelude::StdRng;
use rand::Rng;

use crate::net::{Message, ObjectPath};
use crate::time::{Duration, SimTime};
use crate::util::{Ptr, PtrConst, PtrMut};

///
/// A readonly reference to a channel.
///
pub type ChannelRef = PtrConst<Channel>;

///
/// A mutable reference to a channel.
///
pub type ChannelRefMut = PtrMut<Channel>;

///
/// Metrics that define a channels capabilitites.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelMetrics {
    /// The maximum throughput of the channel in bit/s
    pub bitrate: usize,
    /// The latency a message endures while transversing a channel.
    pub latency: Duration,
    /// The variance in latency.
    pub jitter: Duration,

    /// A userdefined cost for the channel.
    pub cost: f64,
}

impl ChannelMetrics {
    ///
    /// Creates a new instance of channel metrics.
    ///
    #[must_use]
    pub const fn new(bitrate: usize, latency: Duration, jitter: Duration) -> Self {
        Self::new_with_cost(bitrate, latency, jitter, 1.0)
    }

    ///
    /// Creates a new instance of channel metrics.
    ///
    #[must_use]
    pub const fn new_with_cost(
        bitrate: usize,
        latency: Duration,
        jitter: Duration,
        cost: f64,
    ) -> Self {
        Self {
            bitrate,
            latency,
            jitter,
            cost,
        }
    }

    ///
    /// Calcualtes the duration a message travels on a link.
    ///
    #[allow(clippy::if_same_then_else)]
    pub fn calculate_duration(&self, msg: &Message, rng: &mut StdRng) -> Duration {
        if self.bitrate == 0 {
            return Duration::ZERO;
        }

        let len = msg.bit_len;
        let transmission_time = Duration::from_secs_f64(len as f64 / self.bitrate as f64);
        if self.jitter == Duration::ZERO {
            self.latency + transmission_time
        } else {
            let perc = rng.sample(Uniform::new(0.0f64, self.jitter.as_secs_f64()));
            self.latency + transmission_time + Duration::from_secs_f64(perc)
        }
    }

    ///
    /// Calculate the duration the channel is busy transmitting the
    /// message onto the channel.
    ///
    #[must_use]
    pub fn calculate_busy(&self, msg: &Message) -> Duration {
        if self.bitrate == 0 {
            Duration::ZERO
        } else {
            let len = msg.bit_len;
            Duration::from_secs_f64(len as f64 / self.bitrate as f64)
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

impl Eq for ChannelMetrics {}

///
/// A representation of a one directional delayed link,.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    /// The path.
    path: ObjectPath,

    /// The capabilities of the channel.
    metrics: ChannelMetrics,

    /// The stats to track the channels activity.
    #[cfg(feature = "metrics")]
    stats: crate::stats::InProgressChannelStats,

    /// A indicator whether a channel is busy transmitting a packet.
    busy: bool,

    /// The time the current packet is fully transmitted onto the channel.
    transmission_finish_time: SimTime,
}

impl Channel {
    ///
    /// The object path of the channel.
    ///
    #[must_use]
    pub fn path(&self) -> &ObjectPath {
        &self.path
    }

    ///
    /// A description of the channels capabilities,
    /// independent from its current state.
    ///
    #[inline(always)]
    #[must_use]
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
    #[inline(always)]
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    ///
    /// Sets the channel busy, announcing that the message will be trabńsmitted
    /// in '`sim_time`' time units.
    ///
    pub(crate) fn set_busy_until(&mut self, sim_time: SimTime) {
        self.busy = true;
        self.transmission_finish_time = sim_time;
    }

    ///
    /// Resets the busy state of a channel.
    ///
    pub(crate) fn unbusy(&mut self) {
        self.busy = false;
        self.transmission_finish_time = SimTime::ZERO;
    }

    ///
    /// Returns the time when the packet currently being transmitted onto the medium
    /// has been fully transmitted, or [`SimTime::ZERO`] if no packet is currently being transmitted.
    ///
    #[must_use]
    pub fn transmission_finish_time(&self) -> SimTime {
        self.transmission_finish_time
    }

    ///
    /// Creates a new channel using the given metrics,
    /// with an initially unbusy state.
    ///
    #[must_use]
    pub fn new(path: ObjectPath, metrics: ChannelMetrics) -> ChannelRefMut {
        Ptr::new(Self {
            path,
            metrics,
            busy: false,
            transmission_finish_time: SimTime::ZERO,

            #[cfg(feature = "metrics")]
            stats: crate::stats::InProgressChannelStats::new(
                ObjectPath::root_module("chan".to_string()),
                metrics,
            ),
        })
    }

    ///
    /// Calculates the stats for a given channel
    ///
    #[cfg(feature = "metrics")]
    pub fn calculate_stats(&self) -> crate::stats::ChannelStats {
        self.stats
            .evaluate(SimTime::now().duration_since(SimTime::MIN))
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn register_message_passed(&mut self, msg: &Message) {
        self.stats.register_message_passed(msg)
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn register_message_dropped(&mut self, msg: &Message) {
        self.stats.register_message_dropped(msg)
    }

    ///
    /// Calcualtes the packet travel duration using the
    /// underlying metric.
    ///
    #[inline(always)]
    pub fn calculate_duration(&self, msg: &Message, rng: &mut StdRng) -> Duration {
        self.metrics.calculate_duration(msg, rng)
    }

    ///
    /// Calcualtes the busy time of the channel using
    /// the underlying metric.
    ///
    #[inline(always)]
    #[must_use]
    pub fn calculate_busy(&self, msg: &Message) -> Duration {
        self.metrics.calculate_busy(msg)
    }
}
