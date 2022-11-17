//! Physical link abstractions.
#![allow(clippy::cast_precision_loss)]

use rand::distributions::Uniform;
use rand::prelude::StdRng;
use rand::Rng;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::Arc;

use crate::net::runtime::ChannelUnbusyNotif;
use crate::net::{message::Message, MessageAtGateEvent, NetEvents, ObjectPath};
use crate::runtime::{rng, Runtime};
use crate::time::{Duration, SimTime};

use super::{gate::GateRef, NetworkRuntime};

///
/// A readonly reference to a channel.
///
pub type ChannelRef = Arc<Channel>;

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
    /// The size of the channels queue in bytes.
    pub queuesize: usize,
}

impl ChannelMetrics {
    ///
    /// Creates a new instance of channel metrics.
    ///
    #[must_use]
    pub const fn new(bitrate: usize, latency: Duration, jitter: Duration) -> Self {
        Self::new_with_cost(bitrate, latency, jitter, 1.0, 0)
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
        queuesize: usize,
    ) -> Self {
        Self {
            bitrate,
            latency,
            jitter,
            cost,
            queuesize,
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

        let len = msg.length() * 8;
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
            let len = msg.length() * 8;
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
#[derive(Debug)]
pub struct Channel {
    inner: RefCell<ChannelInner>,
}

#[derive(Debug)]
struct ChannelInner {
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

    buffer: VecDeque<(Box<Message>, GateRef)>,
    buffer_len: usize,
}

impl Channel {
    ///
    /// The object path of the channel.
    ///
    #[must_use]
    pub fn path(&self) -> ObjectPath {
        self.inner.borrow().path.clone()
    }

    ///
    /// A description of the channels capabilities,
    /// independent from its current state.
    ///
    #[must_use]
    pub fn metrics(&self) -> ChannelMetrics {
        self.inner.borrow().metrics
    }

    ///
    /// A indicator whether a channel is currently busy transmissting a
    /// packet onto the medium.
    ///
    /// Note that being non-busy does not mean that no packet is currently on the medium
    /// it just means that all bits have been put onto the medium.
    ///
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.inner.borrow().busy
    }

    ///
    /// Sets the channel busy, announcing that the message will be trabńsmitted
    /// in '`sim_time`' time units.
    ///
    pub(crate) fn set_busy_until(&self, sim_time: SimTime) {
        let mut chan = self.inner.borrow_mut();
        chan.busy = true;
        chan.transmission_finish_time = sim_time;
    }

    ///
    /// Returns the time when the packet currently being transmitted onto the medium
    /// has been fully transmitted, or [`SimTime::ZERO`] if no packet is currently being transmitted.
    ///
    #[must_use]
    pub fn transmission_finish_time(&self) -> SimTime {
        self.inner.borrow().transmission_finish_time
    }

    ///
    /// Creates a new channel using the given metrics,
    /// with an initially unbusy state.
    ///
    #[must_use]
    pub fn new(path: ObjectPath, metrics: ChannelMetrics) -> ChannelRef {
        ChannelRef::new(Channel {
            inner: RefCell::new(ChannelInner {
                path,
                metrics,
                busy: false,
                transmission_finish_time: SimTime::ZERO,
                buffer: VecDeque::new(),
                buffer_len: 0,

                #[cfg(feature = "metrics")]
                stats: crate::stats::InProgressChannelStats::new(
                    ObjectPath::root_module("chan"),
                    metrics,
                ),
            }),
        })
    }

    ///
    /// Calculates the stats for a given channel
    ///
    #[cfg(feature = "metrics")]
    pub fn calculate_stats(&self) -> crate::stats::ChannelStats {
        self.inner
            .borrow()
            .stats
            .evaluate(SimTime::now().duration_since(SimTime::MIN))
    }

    // #[cfg(feature = "metrics")]
    // pub(crate) fn register_message_passed(&self, msg: &Message) {
    //     self.inner.borrow_mut().stats.register_message_passed(msg)
    // }

    // #[cfg(feature = "metrics")]
    // pub(crate) fn register_message_dropped(&self, msg: &Message) {
    //     self.inner.borrow_mut().stats.register_message_dropped(msg)
    // }

    ///
    /// Calcualtes the packet travel duration using the
    /// underlying metric.
    ///
    pub fn calculate_duration(&self, msg: &Message, rng: &mut StdRng) -> Duration {
        self.inner.borrow().metrics.calculate_duration(msg, rng)
    }

    ///
    /// Calcualtes the busy time of the channel using
    /// the underlying metric.
    ///
    #[must_use]
    pub fn calculate_busy(&self, msg: &Message) -> Duration {
        self.inner.borrow().metrics.calculate_busy(msg)
    }

    pub(super) fn send_message<A>(
        self: Arc<Self>,
        msg: Box<Message>,
        next_gate: &GateRef,
        rt: &mut Runtime<NetworkRuntime<A>>,
    ) {
        let rng_ref = rng();
        let mut chan = self.inner.borrow_mut();

        if chan.busy {
            let msg_size = msg.length();
            if chan.buffer_len + msg_size > chan.metrics.queuesize {
                log::warn!(
                    "Gate '{}' dropping message [{}] pushed onto busy channel {}",
                    next_gate.previous_gate().unwrap().name(),
                    msg.str(),
                    chan.path
                );

                // Register message progress (DROP)
                #[cfg(feature = "metrics")]
                {
                    chan.stats.register_message_dropped(&msg);
                }

                drop(msg);
                log_scope!();
            } else {
                log::trace!(
                    "Gate '{}' added message [{}] to queue",
                    next_gate.previous_gate().unwrap().name(),
                    msg.str()
                );
                chan.buffer_len += msg.length();
                chan.buffer.push_back((msg, Arc::clone(next_gate)));
            }
        } else {
            // Register message progress (SUCC)
            #[cfg(feature = "metrics")]
            {
                chan.stats.register_message_passed(&msg);
            }

            let dur = chan.metrics.calculate_duration(&msg, rng_ref);
            let busy = chan.metrics.calculate_busy(&msg);

            let transmissin_finish = SimTime::now() + busy;

            drop(chan);
            self.set_busy_until(transmissin_finish);

            rt.add_event(
                NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                    channel: self.clone(),
                }),
                transmissin_finish,
            );

            let next_event_time = SimTime::now() + dur;

            rt.add_event(
                NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                    gate: Arc::clone(next_gate),
                    message: msg,
                }),
                next_event_time,
            );

            // must break iteration,
            // but not perform on-module handling
            log_scope!();
        }
    }

    ///
    /// Resets the busy state of a channel.
    ///
    pub(crate) fn unbusy<A>(self: Arc<Self>, rt: &mut Runtime<NetworkRuntime<A>>) {
        let mut chan = self.inner.borrow_mut();

        chan.busy = false;
        chan.transmission_finish_time = SimTime::ZERO;

        if let Some((msg, next_gate)) = chan.buffer.pop_front() {
            chan.buffer_len -= msg.length();
            drop(chan);
            self.send_message(msg, &next_gate, rt);
        }
    }
}
