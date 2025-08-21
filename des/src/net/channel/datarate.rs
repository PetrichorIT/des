use std::{any::Any, collections::VecDeque, fmt::Debug, sync::Arc, time::Duration};

use crate::{
    net::{
        channel::{SendContext, SendError},
        gate::Connection,
        message::Message,
        runtime::{ChannelUnbusyNotif, MessageExitingConnection, NetEvents},
    },
    prelude::GateRef,
    time::SimTime,
};

use super::{Channel, ChannelDropBehaviour};

/// A channel that supports both datarate limiting and propagation delay.
#[derive(Debug)]
pub struct DatarateChannel {
    metrics: DatarateChannelMetrics,
    buffer: Buffer,
    transmission_finish_time: Option<SimTime>,
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

#[derive(Debug, Default)]
struct Buffer {
    packets: VecDeque<(GateRef, Message, Connection)>,
    acc_bytes: usize,
}

impl DatarateChannel {
    /// NEW
    #[must_use]
    pub fn new(metrics: DatarateChannelMetrics) -> Self {
        Self {
            metrics,
            buffer: Buffer::default(),
            transmission_finish_time: None,
        }
    }
}

impl Buffer {
    fn enqueue(&mut self, src: GateRef, msg: Message, con: Connection) {
        self.acc_bytes += msg.length();
        self.packets.push_back((src, msg, con));
    }

    fn dequeue(&mut self) -> Option<(GateRef, Message, Connection)> {
        let (src, msg, gate) = self.packets.pop_front()?;
        self.acc_bytes -= msg.length();
        Some((src, msg, gate))
    }
}

impl DatarateChannel {
    fn is_busy(&self) -> bool {
        self.transmission_finish_time.is_some()
    }
}

impl Clone for DatarateChannel {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics,
            buffer: Buffer::default(),
            transmission_finish_time: None,
        }
    }
}

impl Channel for DatarateChannel {
    fn transmission_finish_time(&self) -> Option<SimTime> {
        self.transmission_finish_time
    }

    fn send(
        &mut self,
        src: GateRef,
        msg: Message,
        via: Connection,
        ctx: SendContext<'_>,
    ) -> Result<(), SendError> {
        assert!(Arc::ptr_eq(
            &ctx.handle.channel,
            &via.channel.as_ref().unwrap().channel
        ));

        if self.is_busy() {
            self.metrics
                .drop_behaviour
                .handle(&mut self.buffer, src, msg, via)
        } else {
            let propagation_delay = self.metrics.calculate_duration(&msg);
            let transmission_delay = self.metrics.calculate_busy(&msg);

            if !transmission_delay.is_zero() {
                let transmission_finish_time = SimTime::now() + transmission_delay;
                self.transmission_finish_time = Some(transmission_finish_time);

                ctx.sink.add(
                    NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                        channel: ctx.handle.clone(),
                        info: Box::new(()),
                    }), // trust me bro
                    transmission_finish_time,
                );
            }

            let arrival_time = SimTime::now() + propagation_delay;
            ctx.sink.add(
                NetEvents::MessageExitingConnection(MessageExitingConnection { con: via, msg }),
                arrival_time,
            );
            Ok(())
        }
    }

    fn unbusy_notify(&mut self, _: Box<dyn Any + Send>, ctx: SendContext<'_>) {
        debug_assert_eq!(Some(SimTime::now()), self.transmission_finish_time);

        self.transmission_finish_time = None;
        if let Some((src, next_msg, next_via)) = self.buffer.dequeue() {
            let _ = self.send(src, next_msg, next_via, ctx);
        }
    }
}

impl ChannelDropBehaviour {
    fn handle(
        &self,
        buffer: &mut Buffer,
        src: GateRef,
        msg: Message,
        via: Connection,
    ) -> Result<(), SendError> {
        match self {
            Self::Drop => Err(SendError {
                msg,
                reason: "could not handle".into(),
            }),
            Self::Queue(limit) => {
                if buffer.acc_bytes + msg.length() > limit.unwrap_or(usize::MAX) {
                    Err(SendError {
                        msg,
                        reason: "could not handle".into(),
                    })
                } else {
                    buffer.enqueue(src, msg, via);
                    Ok(())
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
