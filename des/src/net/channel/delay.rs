use std::{any::Any, time::Duration};

use crate::{
    net::{
        channel::{SendContext, SendError},
        gate::Connection,
        runtime::{MessageExitingConnection, NetEvents},
    },
    prelude::{Channel, GateRef, Message},
    time::SimTime,
};

/// A channel that introduces a delay before forwarding messages.
#[derive(Debug, Clone)]
pub struct DelayChannel {
    /// The delay duration.
    pub delay: Duration,
}

impl DelayChannel {
    /// Creates a new `DelayChannel` with the specified delay.
    #[must_use]
    pub fn new(delay: Duration) -> Self {
        DelayChannel { delay }
    }
}

impl Channel for DelayChannel {
    fn transmission_finish_time(&self) -> Option<SimTime> {
        None
    }

    fn send(
        &mut self,
        _: GateRef,
        message: Message,
        via: Connection,
        ctx: SendContext<'_>,
    ) -> Result<(), SendError> {
        ctx.sink.add(
            NetEvents::MessageExitingConnection(MessageExitingConnection {
                con: via,
                msg: message,
            }),
            SimTime::now() + self.delay,
        );

        Ok(())
    }

    fn unbusy_notify(&mut self, _: Box<dyn Any + Send>, _: SendContext<'_>) {}
}
