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
    transmission_finish_time: SimTime,
    scheduled: Option<SimTime>,
}

/// Metrics that define a channels capabilitites.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
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
            transmission_finish_time: SimTime::ZERO,
            scheduled: None,
        }
    }

    /// Buffer queue length in bytes.
    pub fn queue_length(&self) -> usize {
        self.buffer.acc_bytes
    }

    /// Buffer queue length in packets.
    pub fn queue_length_packets(&self) -> usize {
        self.buffer.packets.len()
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
    fn is_busy(&self, now: SimTime) -> bool {
        self.transmission_finish_time > now
    }
}

impl Clone for DatarateChannel {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics,
            buffer: Buffer::default(),
            transmission_finish_time: SimTime::ZERO,
            scheduled: None,
        }
    }
}

impl Channel for DatarateChannel {
    fn transmission_finish_time(&self) -> Option<SimTime> {
        Some(self.transmission_finish_time)
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

        let now = SimTime::now();

        if self.is_busy(now) {
            match self.metrics.drop_behaviour {
                ChannelDropBehaviour::Drop => Err(SendError {
                    msg,
                    reason: "could not handle".into(),
                }),
                ChannelDropBehaviour::Queue(limit)
                    if self.buffer.acc_bytes + msg.length() > limit.unwrap_or(usize::MAX) =>
                {
                    Err(SendError {
                        msg,
                        reason: "could not handle: limit exceeded".into(),
                    })
                }
                ChannelDropBehaviour::Queue(_) => {
                    self.buffer.enqueue(src, msg, via);
                    Ok(())
                }
            }
        } else {
            let propagation_delay = self.metrics.latency;
            let transmission_delay = self.metrics.calculate_busy(&msg);

            let arrival_time = now + propagation_delay + transmission_delay;

            if !transmission_delay.is_zero() {
                self.transmission_finish_time = now + transmission_delay;

                // Do not yet schedule an unbusy notification, since
                // it might be unnessecary if now further events are buffered:
                // - is_buys() works without the explicit reset from notif()
                // - only sending queued packets requires an explicit wakeup, so delay notif until we know
                //   there will be queued packets
            }

            ctx.sink.add(
                NetEvents::MessageExitingConnection(MessageExitingConnection { con: via, msg }),
                arrival_time,
            );
            Ok(())
        }
        .map(|()| {
            // Send succesful <==> new tft
            // now add a notif event if we **must** wakeup at the tft
            // aka. we have elements in the buffer && we have not yet
            // scheduled a appropriate tft (success could just be in buffer)
            if !self.buffer.packets.is_empty()
                && self.scheduled != Some(self.transmission_finish_time)
            {
                ctx.sink.add(
                    NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                        channel: ctx.handle.clone(),
                        info: Box::new(()),
                    }),
                    self.transmission_finish_time,
                );
                self.scheduled = Some(self.transmission_finish_time);
            }
        })
    }

    fn unbusy_notify(&mut self, _: Box<dyn Any + Send>, ctx: SendContext<'_>) {
        let now = SimTime::now();
        if self.transmission_finish_time == now {
            self.scheduled = None;
            if let Some((src, next_msg, next_via)) = self.buffer.dequeue() {
                let _ = self.send(src, next_msg, next_via, ctx);
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
