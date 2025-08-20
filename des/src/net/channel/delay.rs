use std::{sync::Arc, time::Duration};

use crate::{
    net::{MessageExitingConnection, NetEvents, gate::Connection},
    prelude::{Channel, ChannelRef, Message},
    runtime::EventSink,
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
    #[allow(clippy::new_ret_no_self)]
    pub fn new(delay: Duration) -> ChannelRef {
        ChannelRef {
            channel: Arc::new(DelayChannel { delay }),
        }
    }
}

impl Channel for DelayChannel {
    // fn dup(self: Arc<Self>) -> Arc<dyn Channel> {
    //     Arc::new((*self).clone())
    // }

    fn transmission_finish_time(&self) -> Option<SimTime> {
        None
    }

    fn send(
        self: Arc<Self>,
        message: Message,
        via: Connection,
        sink: &mut dyn EventSink<NetEvents>,
    ) {
        sink.add(
            NetEvents::MessageExitingConnection(MessageExitingConnection {
                con: via,
                msg: message,
            }),
            SimTime::now() + self.delay,
        );
    }

    fn unbusy_notify(self: Arc<Self>, _: &mut dyn EventSink<NetEvents>) {}
}
