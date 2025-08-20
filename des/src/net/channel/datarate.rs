use std::{
    collections::VecDeque,
    fmt::Debug,
    sync::{self, Arc},
    time::Duration,
};

use crate::{
    net::{
        gate::Connection,
        message::Message,
        runtime::{ChannelUnbusyNotif, MessageExitingConnection, NetEvents},
    },
    runtime::EventSink,
    time::SimTime,
};

use super::{Channel, ChannelDropBehaviour, ChannelRef};

/// A channel that supports both datarate limiting and propagation delay.
pub struct DatarateChannel {
    inner: sync::RwLock<Inner>,
}

/// Metrics that define a channels capabilitites.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DatarateChannelMetrics {
    /// The maximum throughput of the channel in bit/s
    pub bitrate: usize,
    /// The latency a message endures while transversing a channel.
    pub latency: Duration,
    /// The variance in latency.
    pub jitter: Duration,
    /// The size of the channels queue in bytes.
    pub drop_behaviour: ChannelDropBehaviour,
}

#[derive(Debug)]
struct Inner {
    metrics: DatarateChannelMetrics,
    buffer: Buffer,
    transmission_finish_time: Option<SimTime>,
}

#[derive(Debug, Default)]
struct Buffer {
    packets: VecDeque<(Message, Connection)>,
    acc_bytes: usize,
}

impl DatarateChannel {
    /// NEW
    #[must_use]
    pub fn new(metrics: DatarateChannelMetrics) -> Self {
        Self {
            inner: sync::RwLock::new(Inner {
                metrics,
                buffer: Buffer::default(),
                transmission_finish_time: None,
            }),
        }
    }
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

impl Inner {
    fn is_busy(&self) -> bool {
        self.transmission_finish_time.is_some()
    }
}

impl Clone for DatarateChannel {
    fn clone(&self) -> Self {
        Self {
            inner: sync::RwLock::new(Inner {
                metrics: self.inner.read().expect("failed to get lock").metrics,
                buffer: Buffer::default(),
                transmission_finish_time: None,
            }),
        }
    }
}

impl Channel for DatarateChannel {
    fn transmission_finish_time(&self) -> Option<SimTime> {
        self.inner
            .read()
            .expect("failed to get lock")
            .transmission_finish_time
    }

    fn send(self: Arc<Self>, msg: Message, via: Connection, sink: &mut dyn EventSink<NetEvents>) {
        let mut inner = self.inner.write().expect("failed to get lock");
        if inner.is_busy() {
            let Inner {
                metrics, buffer, ..
            } = &mut *inner;

            metrics.drop_behaviour.handle(buffer, msg, via);
        } else {
            let propagation_delay = inner.metrics.calculate_duration(&msg);
            let transmission_delay = inner.metrics.calculate_busy(&msg);

            if !transmission_delay.is_zero() {
                let transmission_finish_time = SimTime::now() + transmission_delay;
                inner.transmission_finish_time = Some(transmission_finish_time);

                sink.add(
                    NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                        channel: ChannelRef {
                            channel: self.clone(),
                        },
                    }), // trust me bro
                    transmission_finish_time,
                );
            }

            let arrival_time = SimTime::now() + propagation_delay;
            sink.add(
                NetEvents::MessageExitingConnection(MessageExitingConnection { con: via, msg }),
                arrival_time,
            );
        }
    }

    fn unbusy_notify(self: Arc<Self>, sink: &mut dyn EventSink<NetEvents>) {
        let mut inner = self.inner.write().expect("failed to get lock");
        debug_assert_eq!(Some(SimTime::now()), inner.transmission_finish_time);

        inner.transmission_finish_time = None;
        if let Some((next_msg, next_via)) = inner.buffer.dequeue() {
            drop(inner);
            self.send(next_msg, next_via, sink);
        }
    }
}

impl ChannelDropBehaviour {
    fn handle(&self, buffer: &mut Buffer, msg: Message, via: Connection) {
        match self {
            Self::Drop => {
                drop(msg);
            }
            Self::Queue(limit) => {
                if buffer.acc_bytes + msg.length() > limit.unwrap_or(usize::MAX) {
                    drop(msg);
                } else {
                    buffer.enqueue(msg, via);
                }
            }
        }
    }
}

impl DatarateChannelMetrics {
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
    #[must_use]
    #[allow(clippy::if_same_then_else, clippy::missing_panics_doc)]
    pub fn calculate_duration(&self, msg: &Message) -> Duration {
        let transmission_time = self.calculate_busy(msg);
        self.latency + transmission_time
    }

    /// Calculate the duration the channel is busy transmitting the
    /// message onto the channel.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn calculate_busy(&self, msg: &Message) -> Duration {
        if self.bitrate == 0 {
            Duration::ZERO
        } else {
            let len = msg.length() * 8;
            Duration::from_secs_f64(len as f64 / self.bitrate as f64)
        }
    }
}

impl Debug for DatarateChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.read() {
            Ok(inner) => inner.fmt(f),
            Err(_) => write!(f, "?"),
        }
    }
}
