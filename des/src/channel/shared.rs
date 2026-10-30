use std::any::Any;

use crate::{
    channel::{SendContext, SendError},
    gate::{Connection, GateRef},
    message::Message,
    prelude::{Channel, ChannelRef},
    runtime::{MessageExitingConnection, NetEvents},
    time::SimTime,
};

/// A trait that descibes a channel provider, which will provide channel implementations for a duplex connection.
///
/// By default, each simplex-stream will be managed by a separate handler.
pub trait IntoDuplexChannel {
    /// Splits the channel provider into two channel handlers.
    ///
    /// If the connection should be full-duplex, both handlers should be different.
    /// If the connection should share a single domain, a shared handler should be returned.
    fn into_duplex(self) -> (ChannelRef, ChannelRef);
}

impl<T: Channel + Clone> IntoDuplexChannel for T {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        let bwk = ChannelRef::from(self.clone());
        let fwd = ChannelRef::from(self);
        (fwd, bwk)
    }
}

impl<T: Channel, V: Channel> IntoDuplexChannel for (T, V) {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        (ChannelRef::from(self.0), ChannelRef::from(self.1))
    }
}

impl IntoDuplexChannel for ChannelRef {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        (self.clone(), self)
    }
}

impl IntoDuplexChannel for (ChannelRef, ChannelRef) {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        self
    }
}

impl Channel for () {
    fn send(
        &mut self,
        _: GateRef,
        msg: Message,
        via: Connection,
        ctx: SendContext<'_>,
    ) -> Result<(), SendError> {
        ctx.sink.add(
            NetEvents::MessageExitingConnection(MessageExitingConnection { con: via, msg }),
            SimTime::now(),
        );
        Ok(())
    }

    fn transmission_finish_time(&self) -> Option<SimTime> {
        None
    }

    fn unbusy_notify(&mut self, _: Box<dyn Any + Send>, _: SendContext<'_>) {
        unreachable!()
    }
}

// # Unidirectional

/// A wrapper that makes a gate connection unidirectional, by connecting a blocking channel to the opposite direction.
#[derive(Debug)]
pub struct Unidirectional<C>(pub C);

impl<C: Channel> IntoDuplexChannel for Unidirectional<C> {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        let fwd = ChannelRef::from(self.0);
        let bwk = ChannelRef::from(NoTransmissionChannel);
        (fwd, bwk)
    }
}

struct NoTransmissionChannel;

impl Channel for NoTransmissionChannel {
    fn send(
        &mut self,
        _: GateRef,
        msg: Message,
        _: Connection,
        _: SendContext<'_>,
    ) -> Result<(), SendError> {
        Err(SendError {
            msg,
            reason: "no transmission in this direction allowed".to_string(),
        })
    }

    fn transmission_finish_time(&self) -> Option<SimTime> {
        None
    }

    fn unbusy_notify(&mut self, _: Box<dyn Any + Send>, _: SendContext<'_>) {
        unreachable!()
    }
}
