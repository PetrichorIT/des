//! A multi-producer, single-consumer queue for sending values between
//! asynchronous tasks, that can be monitored globally.
//!
//! This module provides two variants of the channel: bounded and unbounded. The
//! bounded variant has a limit on the number of messages that the channel can
//! store, and if this limit is reached, trying to send another message will
//! wait until a message is received from the channel. An unbounded channel has
//! an infinite capacity, so the `send` method will always complete immediately.
//! This makes the [`UnboundedSender`] usable from both synchronous and
//! asynchronous code.
//!
//! Similar to the `mpsc` channels provided by `std`, the channel constructor
//! functions provide separate send and receive handles, [`Sender`] and
//! [`Receiver`] for the bounded channel, [`UnboundedSender`] and
//! [`UnboundedReceiver`] for the unbounded channel. If there is no message to read,
//! the current task will be notified when a new value is sent. [`Sender`] and
//! [`UnboundedSender`] allow sending values into the channel. If the bounded
//! channel is at capacity, the send is rejected and the task will be notified
//! when additional capacity is available. In other words, the channel provides
//! backpressure.
//!
//!
//! # Disconnection
//!
//! When all [`Sender`] handles have been dropped, it is no longer
//! possible to send values into the channel. This is considered the termination
//! event of the stream. As such, `Receiver::poll` returns `Ok(Ready(None))`.
//!
//! If the [`Receiver`] handle is dropped, then messages can no longer
//! be read out of the channel. In this case, all further attempts to send will
//! result in an error.
//!
//! # Clean Shutdown
//!
//! When the [`Receiver`] is dropped, it is possible for unprocessed messages to
//! remain in the channel. Instead, it is usually desirable to perform a "clean"
//! shutdown. To do this, the receiver first calls `close`, which will prevent
//! any further messages to be sent into the channel. Then, the receiver
//! consumes the channel to completion, at which point the receiver can be
//! dropped.
//!
//! # Communicating between sync and async code
//!
//! When you want to communicate between synchronous and asynchronous code, there
//! are two situations to consider:
//!
//! **Bounded channel**: If you need a bounded channel, you should use a bounded
//! Tokio `mpsc` channel for both directions of communication. Instead of calling
//! the async [`send`][bounded-send] or [`recv`][bounded-recv] methods, in
//! synchronous code you will need to use the [`blocking_send`][blocking-send] or
//! [`blocking_recv`][blocking-recv] methods.
//!
//! **Unbounded channel**: You should use the kind of channel that matches where
//! the receiver is. So for sending a message _from async to sync_, you should
//! use [the standard library unbounded channel][std-unbounded] or
//! [crossbeam][crossbeam-unbounded].  Similarly, for sending a message _from sync
//! to async_, you should use an unbounded Tokio `mpsc` channel.
//!
//! Please be aware that the above remarks were written with the `mpsc` channel
//! in mind, but they can also be generalized to other kinds of channels. In
//! general, any channel method that isn't marked async can be called anywhere,
//! including outside of the runtime. For example, sending a message on a
//! oneshot channel from outside the runtime is perfectly fine.
//!
//! # Multiple runtimes
//!
//! The mpsc channel does not care about which runtime you use it in, and can be
//! used to send messages from one runtime to another. It can also be used in
//! non-Tokio runtimes.
//!
//! There is one exception to the above: the [`send_timeout`] must be used from
//! within a Tokio runtime, however it is still not tied to one specific Tokio
//! runtime, and the sender may be moved from one Tokio runtime to another.
//!
//! [`Sender`]: crate::sync::mpsc::Sender
//! [`Receiver`]: crate::sync::mpsc::Receiver
//! [bounded-send]: crate::sync::mpsc::Sender::send()
//! [bounded-recv]: crate::sync::mpsc::Receiver::recv()
//! [blocking-send]: crate::sync::mpsc::Sender::blocking_send()
//! [blocking-recv]: crate::sync::mpsc::Receiver::blocking_recv()
//! [`UnboundedSender`]: crate::sync::mpsc::UnboundedSender
//! [`UnboundedReceiver`]: crate::sync::mpsc::UnboundedReceiver
//! [`Handle::block_on`]: tokio::runtime::Handle::block_on()
//! [std-unbounded]: std::sync::mpsc::channel
//! [crossbeam-unbounded]: https://docs.rs/crossbeam/*/crossbeam/channel/fn.unbounded.html
//! [`send_timeout`]: crate::sync::mpsc::Sender::send_timeout

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub use tokio::sync::mpsc::error;
use tokio::sync::{futures::Notified, Notify};

mod bounded;
#[allow(deprecated)]
pub use bounded::{
    channel, channel_unwatched, channel_watched, OwnedPermit, Permit, Receiver, Sender,
};

mod unbounded;
#[allow(deprecated)]
pub use unbounded::{
    unbounded_channel, unbounded_channel_unwatched, unbounded_channel_watched, UnboundedReceiver,
    UnboundedSender,
};

///
/// Runtime globals for monitioring the fill state of
/// a subset of attached channels.
///
#[derive(Debug)]
pub struct Globals {
    ref_cnt: AtomicUsize,
    notify: Notify,
}

impl Globals {
    /// Generates a new set of globals to keep monitor a set of channels.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ref_cnt: AtomicUsize::new(0),
            notify: Notify::new(),
        })
    }

    /// Returns a future that resolves if all monitored
    /// channels are become empty (previously being in a non-empty
    /// state).
    ///
    /// Note that this future will not resolve, if not channel is monitored.
    pub fn notifed(&self) -> Notified {
        self.notify.notified()
    }

    fn increment_count(&self) {
        self.ref_cnt.fetch_add(1, Ordering::AcqRel);
    }

    fn decrement_count(&self) {
        if self.ref_cnt.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.notify.notify_one()
        }
    }
}
