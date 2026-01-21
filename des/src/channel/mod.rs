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
//! # use des::{runtime::EventSink, net::{internals::NetEvents, gate::Connection}};
//! # use std::sync::Arc;
//! # use std::any::Any;
//! # use des::net::channel::SendContext;
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
//!     fn send<'ctx>(
//!         &mut self,
//!         src: GateRef,
//!         message: Message,
//!         via: Connection,
//!         ctx: SendContext<'ctx>
//!     ) -> Result<(), SendError> {
//!         // Implement your custom channel send logic here
//!  #      Ok(())
//!     }
//!
//!     fn unbusy_notify<'ctx>(&mut self, info: Box<dyn Any + Send>, sink: SendContext<'ctx>) {
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

use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use crate::{
    gate::Connection,
    prelude::{GateRef, Message},
    runtime::{EventSink, NetEvents},
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
    pub(crate) channel: Arc<RwLock<dyn Channel>>,
}

/// The implementation of a gate-to-gate link.
pub trait Channel: Any {
    /// Returns the time at which the channel will be free again.
    fn transmission_finish_time(&self) -> Option<SimTime>;

    /// Register a gate to be participating in the communication domain.
    fn register(&mut self, endpoint: GateRef) {
        let _ = endpoint;
    }

    /// Unregister a gate to be participating in the communication domain.
    fn unregister(&mut self, endpoint: GateRef) {
        let _ = endpoint;
    }

    /// A method to send a message through the channel
    ///
    /// # Errors
    ///
    /// Returns an error if the channel cannot send the message.
    fn send(
        &mut self,
        src: GateRef,
        message: Message,
        via: Connection,
        ctx: SendContext<'_>,
    ) -> Result<(), SendError>;

    /// A method to notify the channel that it is no longer busy.
    fn unbusy_notify(&mut self, info: Box<dyn Any + Send>, ctx: SendContext<'_>);
}

/// A context for sending a message through a channel.
pub struct SendContext<'a> {
    /// The sink to send the message to.
    pub sink: &'a mut dyn EventSink<NetEvents>,
    /// The handle of the channel.
    pub handle: ChannelRef,
}

/// An error that occurs when sending a message through a channel.
#[derive(Debug, Clone)]
pub struct SendError {
    /// The message that could not be sent.
    pub msg: Message,
    /// The reason why the message could not be sent.
    pub reason: String,
}

/// The behaviour a link should follow, if it is oversubscribed
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
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
        self.transmission_finish_time()
            .is_some_and(|tft| tft > SimTime::now())
    }

    /// Returns the time at which the channel will be free again.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn transmission_finish_time(&self) -> Option<SimTime> {
        self.channel.try_read().unwrap().transmission_finish_time()
    }

    /// Returns a reference to the channel if it is of type T.
    #[must_use]
    pub fn downcast_ref<T: Any, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        let lock = self.channel.read().ok()?;
        let val: &dyn Any = &(*lock);
        let val = val.downcast_ref::<T>()?;
        Some(f(val))
    }

    /// Returns a mutable reference to the channel if it is of type T.
    #[must_use]
    pub fn downcast_mut<T: Any, R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let mut lock = self.channel.write().ok()?;
        let val: &mut dyn Any = &mut (*lock);
        let val = val.downcast_mut::<T>()?;
        Some(f(val))
    }
}

impl<T: Channel> From<T> for ChannelRef {
    fn from(channel: T) -> Self {
        ChannelRef {
            channel: Arc::new(RwLock::new(channel)),
        }
    }
}

impl<C: Channel> From<Arc<RwLock<C>>> for ChannelRef {
    fn from(channel: Arc<RwLock<C>>) -> Self {
        ChannelRef { channel }
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
            fn from(channel: &dyn Channel) -> Self {
                if let Some(until) = channel.transmission_finish_time() {
                    Self::Busy { until }
                } else {
                    Self::Idle
                }
            }
        }

        ChannelRefFmt::from(&*self.channel.try_read().unwrap()).fmt(f)
    }
}

impl Debug for SendContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendContext").finish()
    }
}

unsafe impl Send for ChannelRef {}
