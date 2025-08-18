//! Link abstractions.
//!
//! Channels are the basic building blocks for modifying link behaviour in a graph
//! of connected gates. An individual channel is a handler that schedules packet transmissions
//! for one given communication domain (usually a simplex).
//!
//! Channels can be used to create simple datarate-limited, delayed links or
//! to model shared access on a single medium from multiple gate-links.
//!
//! ## Exclusive media
//!
//! To connect two gates with a delaying link use either [`DelayChannel`] or [`DatarateChannel`].
//! This following example will apply a [`DatarateChannel`] to both directions of the duplex link
//! between `alice` and `bob`.
//!
//! ```
//! # use des::prelude::*;
//! # use std::time::Duration;
//! # let mut sim = Sim::new(());
//! # sim.node("alice", ());
//! # sim.node("bob", ());
//! let eth0_alice = sim.gate("alice", "eth0");
//! let eth0_bob = sim.gate("bob", "eth0");
//!
//! let chan = DatarateChannel::new(
//!     DatarateChannelMetrics::new(
//!         8_000_000, Duration::from_millis(10),
//!         Duration::ZERO, ChannelDropBehaviour::Drop,
//!     ),
//! );
//! eth0_alice.connect_with(eth0_bob, Some(chan));
//! ```
//!
//! If you want to treat one direction different than the other, you can provide two different channel handlers
//! (aka. types that implement [`Channel`]):
//!
//! ```
//! # use des::prelude::*;
//! # use std::time::Duration;
//! # let mut sim = Sim::new(());
//! # sim.node("alice", ());
//! # sim.node("bob", ());
//! # let eth0_alice = sim.gate("alice", "eth0");
//! # let eth0_bob = sim.gate("bob", "eth0");
//! let chan_alice_to_bob = DatarateChannel::new(
//! #    DatarateChannelMetrics::new(
//! #        8_000_000, Duration::from_millis(10),
//! #        Duration::ZERO, ChannelDropBehaviour::Drop,
//! #    ),
//!     /* ... */
//! );
//! let chan_bob_to_alice = DatarateChannel::new(
//! #   DatarateChannelMetrics::new(
//! #        4_000_000, Duration::from_millis(5),
//! #       Duration::ZERO, ChannelDropBehaviour::Drop,
//! #   ),
//!     /* ... */
//! );
//!
//! eth0_alice.connect_with(eth0_bob, Some((chan_alice_to_bob, chan_bob_to_alice)));
//! ```
//!
//! ### Shared media
//!
//! Many real-life communication systems use shared communication domains. This can be modeled by providing
//! a single channel handler to multiple gate-connections. Depending on the internal implementation of the handler,
//! multiple simplex connection can share a single "physical link". To create a shared medium, create the handler
//! directly using [`ChannelRef::from`].
//!
//! ```
//! # use des::prelude::*;
//! # use std::time::Duration;
//! # let mut sim = Sim::new(());
//! # sim.node("tower", ());
//! # sim.node("alice", ());
//! # sim.node("bob", ());
//! # let tower_port_1 = sim.gate("tower", "port-1");
//! # let tower_port_2 = sim.gate("tower", "port-2");
//! # let port_alice = sim.gate("alice", "port");
//! # let port_bob = sim.gate("bob", "port");
//! let basic_channel = DatarateChannel::new(
//! #    DatarateChannelMetrics::new(
//! #        8_000_000, Duration::from_millis(10),
//! #        Duration::ZERO, ChannelDropBehaviour::Drop,
//! #    ),
//!     /* ... */
//! );
//! let shared_channel = ChannelRef::from(basic_channel);
//!
//! port_alice.connect_with(tower_port_1, Some(shared_channel.clone()));
//! port_bob.connect_with(tower_port_2, Some(shared_channel.clone()));
//! ```
//!
//! If the provided channel implementation does not suffice, you can always implement
//! additional channels by creating a new struct that implements the [Channel] trait.
//!
//! ```
//! # use des::prelude::*;
//! # use std::time::Duration;
//! # let mut sim = Sim::new(());
//! # sim.node("tower", ());
//! # sim.node("alice", ());
//! # sim.node("bob", ());
//! # let tower_port_1 = sim.gate("tower", "port-1");
//! # let tower_port_2 = sim.gate("tower", "port-2");
//! # let port_alice = sim.gate("alice", "port");
//! # let port_bob = sim.gate("bob", "port");
//! # use des::{runtime::EventSink, net::{NetEvents, gate::Connection}};
//! # use std::sync::Arc;
//! struct CustomChannel {
//!     // Define your custom channel fields here
//! }
//!
//! impl Channel for CustomChannel {
//!     fn transmission_finish_time(&self) -> Option<SimTime> {
//!         // A transmission finish time must always be provided, so that
//!         // nodes may figure out when a channel is usable again
//! #        None
//!     }
//!
//!     fn send(
//!         self: Arc<Self>,
//!         message: Message,
//!         via: Connection,
//!         sink: &mut dyn EventSink<NetEvents>,
//!     ) {
//!         // Implement your custom channel send logic here
//!     }
//!
//!     fn unbusy_notify(self: Arc<Self>, sink: &mut dyn EventSink<NetEvents>) {
//!         // Implement your custom channel unbusy notification logic here
//!     }
//! }
//!
//! let custom_channel = CustomChannel {
//!     // Initialize your custom channel fields here
//! };
//! let custom_channel_shared = ChannelRef::from(custom_channel);
//!
//! port_alice.connect_with(tower_port_1, Some(custom_channel_shared.clone()));
//! port_bob.connect_with(tower_port_2, Some(custom_channel_shared.clone()));
//! ```

use std::{fmt::Debug, sync::Arc};

use crate::{
    net::{NetEvents, gate::Connection},
    prelude::Message,
    runtime::EventSink,
    time::SimTime,
};

mod datarate;
mod delay;
mod shared;

pub use datarate::*;
pub use delay::*;
pub use shared::*;

/// A reference to a channel.
///
/// Can be freely cloned, as it is a reference-counted pointer.
#[derive(Clone)]
pub struct ChannelRef {
    pub(super) channel: Arc<dyn Channel>,
}

/// The implementation of a gate-to-gate link.
pub trait Channel: 'static {
    /// Returns the time at which the channel will be free again.
    fn transmission_finish_time(&self) -> Option<SimTime>;

    /// A method to send a message through the channek
    fn send(
        self: Arc<Self>,
        message: Message,
        via: Connection,
        sink: &mut dyn EventSink<NetEvents>,
    );

    /// A method to notify the channel that it is no longer busy.
    fn unbusy_notify(self: Arc<Self>, sink: &mut dyn EventSink<NetEvents>);
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

impl ChannelRef {
    /// Returns true if the channel is currently busy.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.channel.transmission_finish_time().is_some()
    }

    /// Returns the time at which the channel will be free again.
    #[must_use]
    pub fn transmission_finish_time(&self) -> Option<SimTime> {
        self.channel.transmission_finish_time()
    }
}

impl<T: Channel> From<T> for ChannelRef {
    fn from(channel: T) -> Self {
        ChannelRef {
            channel: Arc::new(channel),
        }
    }
}

impl<C: Channel> From<Arc<C>> for ChannelRef {
    fn from(channel: Arc<C>) -> Self {
        ChannelRef { channel: channel }
    }
}

impl Debug for ChannelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(unused)]
        enum ChannelRefFmt {
            Idle,
            Busy { until: SimTime },
        }

        impl ChannelRefFmt {
            fn from(channel: &Arc<dyn Channel>) -> Self {
                if let Some(until) = channel.transmission_finish_time() {
                    Self::Busy { until }
                } else {
                    Self::Idle
                }
            }
        }

        ChannelRefFmt::from(&self.channel).fmt(f)
    }
}
