//! Physical link abstractions.
#![allow(clippy::cast_precision_loss)]

use rand::distr::Uniform;
use rand::prelude::StdRng;
use rand::{Rng, RngCore};
use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::sync::{Arc, RwLock};

use crate::net::{
    message::Message, runtime::ChannelUnbusyNotif, MessageExitingConnection, NetEvents,
};
use crate::runtime::{rng, EventSink};
use crate::time::{Duration, SimTime};

use super::gate::Connection;

/// A readonly reference to a channel.
pub type ChannelRef = Arc<Channel>;

/// A representation of a one directional delayed link,.
pub struct Channel {
    inner: RwLock<ChannelInner>,
}

struct ChannelInner {
    metrics: ChannelMetrics,
    busy: bool,
    transmission_finish_time: SimTime,
    buffer: Buffer,
    probe: Box<dyn ChannelProbe>,
}

#[derive(Default)]
struct Buffer {
    packets: VecDeque<(Message, Connection)>,
    acc_bytes: usize,
}

impl Buffer {
    fn enqueue(&mut self, msg: Message, con: Connection) {
        self.acc_bytes += msg.length();
        self.packets.push_back((msg, con));
    }

    fn dequeue(&mut self) -> Option<(Message, Connection)> {
        let (msg, gate) = self.packets.pop_front()?;
        self.acc_bytes -= msg.length();
        Some((msg, gate))
    }
}

/// Metrics that define a channels capabilitites.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelMetrics {
    /// The maximum throughput of the channel in bit/s
    pub bitrate: usize,
    /// The latency a message endures while transversing a channel.
    pub latency: Duration,
    /// The variance in latency.
    pub jitter: Duration,
    /// The size of the channels queue in bytes.
    pub drop_behaviour: ChannelDropBehaviour,
}

/// The behaviour a link should follow, if it is oversubscribed
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ChannelDropBehaviour {
    /// If a link is currently busy, drop packets
    #[default]
    Drop,
    /// If a link is currently busy, queue packets up to a
    /// provided queuelength (None means infinite queuelength)
    Queue(Option<usize>),
}

impl ChannelInner {
    fn dup(&self) -> Self {
        Self {
            metrics: self.metrics,
            busy: false,
            transmission_finish_time: SimTime::ZERO,
            buffer: Buffer::default(),
            probe: Box::new(DummyChannelProbe),
        }
    }
}

impl Channel {
    pub(super) fn dup(&self) -> Self {
        Channel {
            inner: RwLock::new(self.inner.read().unwrap().dup()),
        }
    }

    /// A description of the channels capabilities,
    /// independent from its current state.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    #[must_use]
    pub fn metrics(&self) -> ChannelMetrics {
        self.inner.read().unwrap().metrics
    }

    /// A indicator whether a channel is currently busy transmissting a
    /// packet onto the medium.
    ///
    /// Note that being non-busy does not mean that no packet is currently on the medium
    /// it just means that all bits have been put onto the medium.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.inner.read().unwrap().busy
    }

    /// Attaches a probe
    ///
    /// # Panics
    ///
    /// Panics if the channel mutex was poisoned.
    pub fn attach_probe(&self, probe: impl ChannelProbe) {
        let mut chan = self
            .inner
            .write()
            .expect("failed to get inner channel lock");
        let probe = Box::new(probe);
        chan.probe = probe;
    }

    /// Sets the channel busy, announcing that the message will be trabńsmitted
    /// in '`sim_time`' time units.
    pub(crate) fn set_busy_until(&self, sim_time: SimTime) {
        let mut chan = self.inner.write().unwrap();
        chan.busy = true;
        chan.transmission_finish_time = sim_time;
    }

    /// Returns the time when the packet currently being transmitted onto the medium
    /// has been fully transmitted, or [`SimTime::ZERO`] if no packet is currently being transmitted.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    #[must_use]
    pub fn transmission_finish_time(&self) -> SimTime {
        self.inner.read().unwrap().transmission_finish_time
    }

    /// Creates a new channel using the given metrics,
    /// with an initially unbusy state.
    #[must_use]
    pub fn new(metrics: ChannelMetrics) -> ChannelRef {
        ChannelRef::new(Channel {
            inner: RwLock::new(ChannelInner {
                metrics,
                busy: false,
                transmission_finish_time: SimTime::ZERO,
                buffer: Buffer::default(),
                probe: Box::new(DummyChannelProbe),
            }),
        })
    }

    /// Calcualtes the packet travel duration using the
    /// underlying metric.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    pub fn calculate_duration(&self, msg: &Message, rng: &mut StdRng) -> Duration {
        self.inner
            .read()
            .unwrap()
            .metrics
            .calculate_duration(msg, rng)
    }

    /// Calcualtes the busy time of the channel using
    /// the underlying metric.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    #[must_use]
    pub fn calculate_busy(&self, msg: &Message) -> Duration {
        self.inner.read().unwrap().metrics.calculate_busy(msg)
    }

    pub(super) fn send_message<S: EventSink<NetEvents>>(
        self: Arc<Self>,
        msg: Message,
        via: Connection,
        sink: &mut S,
    ) {
        let rng_ref = rng();
        let mut chan = self.inner.write().unwrap();

        if chan.busy {
            let ChannelInner {
                metrics, buffer, ..
            } = &mut *chan;

            metrics.drop_behaviour.handle(buffer, msg, via);
        } else {
            let ChannelInner { probe, metrics, .. } = &mut *chan;
            probe.on_message_transmit(metrics, &msg);

            let dur = metrics.calculate_duration(&msg, rng_ref);
            let busy = metrics.calculate_busy(&msg);

            if busy != Duration::ZERO {
                let transmissin_finish = SimTime::now() + busy;

                drop(chan);
                self.set_busy_until(transmissin_finish);

                sink.add(
                    NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                        channel: self.clone(),
                    }),
                    transmissin_finish,
                );
            }

            let next_event_time = SimTime::now() + dur;

            sink.add(
                NetEvents::MessageExitingConnection(MessageExitingConnection {
                    con: via.clone(),
                    msg,
                }),
                next_event_time,
            );

            // must break iteration,
            // but not perform on-module handling
        }
    }

    /// Resets the busy state of a channel.
    pub(crate) fn unbusy<S: EventSink<NetEvents>>(self: Arc<Self>, sink: &mut S) {
        let mut chan = self.inner.write().unwrap();

        chan.busy = false;
        chan.transmission_finish_time = SimTime::ZERO;

        if let Some((msg, next_gate)) = chan.buffer.dequeue() {
            drop(chan);
            self.send_message(msg, next_gate, sink);
        }
    }
}

impl ChannelDropBehaviour {
    fn handle(&self, buffer: &mut Buffer, msg: Message, via: Connection) {
        match self {
            Self::Drop => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Gate '{}' dropping message [{}] pushed onto busy channel",
                    via.prev_hop().unwrap().name(),
                    msg,
                );
                drop(msg);
            }
            Self::Queue(limit) => {
                if buffer.acc_bytes + msg.length() > limit.unwrap_or(usize::MAX) {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "Gate '{}' dropping message [{}] pushed onto busy channel",
                        via.prev_hop().unwrap().name(),
                        msg,
                    );
                    drop(msg);
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(
                        "Gate '{}' added message [{}] to queue of channel",
                        via.prev_hop().unwrap().name(),
                        msg,
                    );
                    buffer.enqueue(msg, via);
                }
            }
        }
    }
}

impl Debug for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(unused)]
        enum FmtChannelState {
            Idle,
            Busy {
                until: SimTime,
                bytes: usize,
                packets: usize,
            },
        }

        impl FmtChannelState {
            fn from(channel: &ChannelInner) -> Self {
                if channel.busy {
                    Self::Busy {
                        until: channel.transmission_finish_time,
                        bytes: channel.buffer.acc_bytes,
                        packets: channel.buffer.packets.len(),
                    }
                } else {
                    Self::Idle
                }
            }
        }

        let this = self.inner.read().unwrap();

        f.debug_struct("Channel")
            .field("metrics", &this.metrics)
            .field("state", &FmtChannelState::from(&this))
            .finish()
    }
}

impl ChannelMetrics {
    /// Creates a new instance of channel metrics.
    #[must_use]
    pub const fn new(
        bitrate: usize,
        latency: Duration,
        jitter: Duration,
        drop_behaviour: ChannelDropBehaviour,
    ) -> Self {
        Self {
            bitrate,
            latency,
            jitter,
            drop_behaviour,
        }
    }

    /// Calcualtes the duration a message travels on a link.
    #[allow(clippy::if_same_then_else, clippy::missing_panics_doc)]
    pub fn calculate_duration(&self, msg: &Message, rng: &mut dyn RngCore) -> Duration {
        let transmission_time = self.calculate_busy(msg);
        if self.jitter == Duration::ZERO {
            self.latency + transmission_time
        } else {
            let perc = rng.sample(Uniform::new(0.0f64, self.jitter.as_secs_f64()).unwrap());
            self.latency + transmission_time + Duration::from_secs_f64(perc)
        }
    }

    /// Calculate the duration the channel is busy transmitting the
    /// message onto the channel.
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

/// A trait to define channel probing.
pub trait ChannelProbe: 'static {
    /// Reacts to a message
    fn on_message_transmit(&mut self, chan: &ChannelMetrics, msg: &Message);
}

struct DummyChannelProbe;
impl ChannelProbe for DummyChannelProbe {
    #[inline]
    fn on_message_transmit(&mut self, _: &ChannelMetrics, _: &Message) {}
}
