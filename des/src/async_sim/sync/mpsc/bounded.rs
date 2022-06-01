use std::fmt::Debug;
use std::future::Future;

use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use tokio::sync::mpsc::error;
pub use tokio::sync::mpsc::OwnedPermit;
pub use tokio::sync::mpsc::Permit;

use super::Globals;
use crate::prelude::Duration;

/// Creates a bounded mpsc channel for communicating between asynchronous tasks
/// with backpressure, reporing metrics to the given Global instance.
///
/// The channel will buffer up to the provided number of messages.  Once the
/// buffer is full, attempts to send new messages will wait until a message is
/// received from the channel. The provided buffer capacity must be at least 1.
///
/// All data sent on `Sender` will become available on `Receiver` in the same
/// order as it was sent.
///
/// The `Sender` can be cloned to `send` to the same channel from multiple code
/// locations. Only one `Receiver` is supported.
///
/// If the `Receiver` is disconnected while trying to `send`, the `send` method
/// will return a `SendError`. Similarly, if `Sender` is disconnected while
/// trying to `recv`, the `recv` method will return `None`.
///
/// # Panics
///
/// Panics if the buffer capacity is 0.
///
/// # Examples
///
/// ```rust
/// use des::async_sim::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() {
///     let (tx, mut rx) = mpsc::channel(100);
///
///     tokio::spawn(async move {
///         for i in 0..10 {
///             if let Err(_) = tx.send(i).await {
///                 println!("receiver dropped");
///                 return;
///             }
///         }
///     });
///
///     while let Some(i) = rx.recv().await {
///         println!("got = {}", i);
///     }
/// }
/// ```
pub fn channel_watched<T>(buffer: usize, globals: Arc<Globals>) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer);
    (
        Sender {
            inner: tx,
            globals: Arc::clone(&globals),
        },
        Receiver { inner: rx, globals },
    )
}

/// Creates a bounded mpsc channel for communicating between asynchronous tasks
/// with backpressure, without reporting metrics.
///
/// The channel will buffer up to the provided number of messages.  Once the
/// buffer is full, attempts to send new messages will wait until a message is
/// received from the channel. The provided buffer capacity must be at least 1.
///
/// All data sent on `Sender` will become available on `Receiver` in the same
/// order as it was sent.
///
/// The `Sender` can be cloned to `send` to the same channel from multiple code
/// locations. Only one `Receiver` is supported.
///
/// If the `Receiver` is disconnected while trying to `send`, the `send` method
/// will return a `SendError`. Similarly, if `Sender` is disconnected while
/// trying to `recv`, the `recv` method will return `None`.
///
/// # Panics
///
/// Panics if the buffer capacity is 0.
///
/// # Examples
///
/// ```rust
/// use des::async_sim::sync::mpsc;
///
/// #[tokio::main]
/// async fn main() {
///     let (tx, mut rx) = mpsc::channel(100);
///
///     tokio::spawn(async move {
///         for i in 0..10 {
///             if let Err(_) = tx.send(i).await {
///                 println!("receiver dropped");
///                 return;
///             }
///         }
///     });
///
///     while let Some(i) = rx.recv().await {
///         println!("got = {}", i);
///     }
/// }
/// ```
pub fn channel_unwatched<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    channel_watched(buffer, Globals::new())
}

///
/// An alias for [channel_unwatched].
/// This alias exists to gurantee compatibility with existing tokio projects
/// to create non-supervied channel. Use either [channel_unwatched] or
/// [channel_watched] for specific behaviour inside a simulation context
///
#[deprecated]
pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    channel_unwatched(buffer)
}

/// Sends values to the associated `Receiver`.
///
/// Instances are created by the [`channel`](channel) function.
///
pub struct Sender<T> {
    inner: tokio::sync::mpsc::Sender<T>,
    globals: Arc<Globals>,
}

impl<T> Sender<T> {
    /// Completes when the receiver has dropped.
    ///
    /// This allows the producers to get notified when interest in the produced
    /// values is canceled and immediately stop doing work.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. Once the channel is closed, it stays closed
    /// forever and all future calls to `closed` will return immediately.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx1, rx) = mpsc::channel::<()>(1);
    ///     let tx2 = tx1.clone();
    ///     let tx3 = tx1.clone();
    ///     let tx4 = tx1.clone();
    ///     let tx5 = tx1.clone();
    ///     tokio::spawn(async move {
    ///         drop(rx);
    ///     });
    ///
    ///     futures::join!(
    ///         tx1.closed(),
    ///         tx2.closed(),
    ///         tx3.closed(),
    ///         tx4.closed(),
    ///         tx5.closed()
    ///     );
    ///     println!("Receiver dropped");
    /// }
    /// ```
    pub async fn closed(&self) {
        self.inner.closed().await
    }

    /// Checks if the channel has been closed. This happens when the
    /// [`Receiver`] is dropped, or when the [`Receiver::close`] method is
    /// called.
    ///
    /// [`Receiver`]: crate::async_sim::sync::mpsc::Receiver
    /// [`Receiver::close`]: crate::async_sim::sync::mpsc::Receiver::close
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    /// let (tx, rx) = mpsc::channel::<()>(42);
    /// assert!(!tx.is_closed());
    ///
    /// let tx2 = tx.clone();
    /// assert!(!tx2.is_closed());
    ///
    /// drop(rx);
    /// assert!(tx.is_closed());
    /// assert!(tx2.is_closed());
    /// ```
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Returns `true` if senders belong to the same channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    /// let (tx, rx) = mpsc::channel::<()>(1);
    /// let  tx2 = tx.clone();
    /// assert!(tx.same_channel(&tx2));
    ///
    /// let (tx3, rx3) = mpsc::channel::<()>(1);
    /// assert!(!tx3.same_channel(&tx2));
    /// ```
    pub fn same_channel(&self, other: &Self) -> bool {
        self.inner.same_channel(&other.inner)
    }

    /// Returns the current capacity of the channel.
    ///
    /// The capacity goes down when sending a value by calling [`send`] or by reserving capacity
    /// with [`reserve`]. The capacity goes up when values are received by the [`Receiver`].
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel::<()>(5);
    ///
    ///     assert_eq!(tx.capacity(), 5);
    ///
    ///     // Making a reservation drops the capacity by one.
    ///     let permit = tx.reserve().await.unwrap();
    ///     assert_eq!(tx.capacity(), 4);
    ///
    ///     // Sending and receiving a value increases the capacity by one.
    ///     permit.send(());
    ///     rx.recv().await.unwrap();
    ///     assert_eq!(tx.capacity(), 5);
    /// }
    /// ```
    ///
    /// [`send`]: Sender::send
    /// [`reserve`]: Sender::reserve
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Sends a value, waiting until there is capacity.
    ///
    /// A successful send occurs when it is determined that the other end of the
    /// channel has not hung up already. An unsuccessful send would be one where
    /// the corresponding receiver has already been closed. Note that a return
    /// value of `Err` means that the data will never be received, but a return
    /// value of `Ok` does not mean that the data will be received. It is
    /// possible for the corresponding receiver to hang up immediately after
    /// this function returns `Ok`.
    ///
    /// # Errors
    ///
    /// If the receive half of the channel is closed, either due to [`close`]
    /// being called or the [`Receiver`] handle dropping, the function returns
    /// an error. The error includes the value passed to `send`.
    ///
    /// [`close`]: Receiver::close
    /// [`Receiver`]: Receiver
    ///
    /// # Cancel safety
    ///
    /// If `send` is used as the event in a [`tokio::select!`](tokio::select)
    /// statement and some other branch completes first, then it is guaranteed
    /// that the message was not sent.
    ///
    /// This channel uses a queue to ensure that calls to `send` and `reserve`
    /// complete in the order they were requested.  Cancelling a call to
    /// `send` makes you lose your place in the queue.
    ///
    /// # Examples
    ///
    /// In the following example, each call to `send` will block until the
    /// previously sent value was received.
    ///
    /// ```rust
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     tokio::spawn(async move {
    ///         for i in 0..10 {
    ///             if let Err(_) = tx.send(i).await {
    ///                 println!("receiver dropped");
    ///                 return;
    ///             }
    ///         }
    ///     });
    ///
    ///     while let Some(i) = rx.recv().await {
    ///         println!("got = {}", i);
    ///     }
    /// }
    /// ```
    pub async fn send(&self, value: T) -> Result<(), error::SendError<T>> {
        self.globals.increment_count();
        self.inner.send(value).await.map_err(|e| {
            self.globals.decrement_count();
            e
        })
    }

    /// Attempts to immediately send a message on this `Sender`
    ///
    /// This method differs from [`send`] by returning immediately if the channel's
    /// buffer is full or no receiver is waiting to acquire some data. Compared
    /// with [`send`], this function has two failure cases instead of one (one for
    /// disconnection, one for a full buffer).
    ///
    /// # Errors
    ///
    /// If the channel capacity has been reached, i.e., the channel has `n`
    /// buffered values where `n` is the argument passed to [`channel`], then an
    /// error is returned.
    ///
    /// If the receive half of the channel is closed, either due to [`close`]
    /// being called or the [`Receiver`] handle dropping, the function returns
    /// an error. The error includes the value passed to `send`.
    ///
    /// [`send`]: Sender::send
    /// [`channel`]: channel
    /// [`close`]: Receiver::close
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     // Create a channel with buffer size 1
    ///     let (tx1, mut rx) = mpsc::channel(1);
    ///     let tx2 = tx1.clone();
    ///
    ///     tokio::spawn(async move {
    ///         tx1.send(1).await.unwrap();
    ///         tx1.send(2).await.unwrap();
    ///         // task waits until the receiver receives a value.
    ///     });
    ///
    ///     tokio::spawn(async move {
    ///         // This will return an error and send
    ///         // no message if the buffer is full
    ///         let _ = tx2.try_send(3);
    ///     });
    ///
    ///     let mut msg;
    ///     msg = rx.recv().await.unwrap();
    ///     println!("message {} received", msg);
    ///
    ///     msg = rx.recv().await.unwrap();
    ///     println!("message {} received", msg);
    ///
    ///     // Third message may have never been sent
    ///     match rx.recv().await {
    ///         Some(msg) => println!("message {} received", msg),
    ///         None => println!("the third message was never sent"),
    ///     }
    /// }
    /// ```
    pub fn try_send(&self, value: T) -> Result<(), error::TrySendError<T>> {
        self.globals.increment_count();
        self.inner.try_send(value).map_err(|e| {
            self.globals.decrement_count();
            e
        })
    }

    /// This function is deprecated in a simulation context.
    #[deprecated]
    pub async fn send_timeout(
        &self,
        _value: T,
        _timeout: Duration,
    ) -> Result<(), error::SendTimeoutError<T>> {
        panic!("This function is not supported in a simulation context")
    }

    /// Blocking send to call outside of asynchronous contexts.
    ///
    /// This method is intended for use cases where you are sending from
    /// synchronous code to asynchronous code, and will work even if the
    /// receiver is not using [`blocking_recv`] to receive the message.
    ///
    /// [`blocking_recv`]: fn@crate::async_sim::sync::mpsc::Receiver::blocking_recv
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use tokio::runtime::Runtime;
    /// use des::async_sim::sync::mpsc;
    ///
    /// fn main() {
    ///     let (tx, mut rx) = mpsc::channel::<u8>(1);
    ///
    ///     let sync_code = thread::spawn(move || {
    ///         tx.blocking_send(10).unwrap();
    ///     });
    ///
    ///     Runtime::new().unwrap().block_on(async move {
    ///         assert_eq!(Some(10), rx.recv().await);
    ///     });
    ///     sync_code.join().unwrap()
    /// }
    /// ```
    pub fn blocking_send(&self, value: T) -> Result<(), error::SendError<T>> {
        self.globals.increment_count();
        self.inner.blocking_send(value).map_err(|e| {
            self.globals.decrement_count();
            e
        })
    }

    /// Waits for channel capacity. Once capacity to send one message is
    /// available, it is reserved for the caller.
    ///
    /// If the channel is full, the function waits for the number of unreceived
    /// messages to become less than the channel capacity. Capacity to send one
    /// message is reserved for the caller. A [`Permit`] is returned to track
    /// the reserved capacity. The [`send`] function on [`Permit`] consumes the
    /// reserved capacity.
    ///
    /// Dropping [`Permit`] without sending a message releases the capacity back
    /// to the channel.
    ///
    /// [`Permit`]: Permit
    /// [`send`]: Permit::send
    ///
    /// # Cancel safety
    ///
    /// This channel uses a queue to ensure that calls to `send` and `reserve`
    /// complete in the order they were requested.  Cancelling a call to
    /// `reserve` makes you lose your place in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     // Reserve capacity
    ///     let permit = tx.reserve().await.unwrap();
    ///
    ///     // Trying to send directly on the `tx` will fail due to no
    ///     // available capacity.
    ///     assert!(tx.try_send(123).is_err());
    ///
    ///     // Sending on the permit succeeds
    ///     permit.send(456);
    ///
    ///     // The value sent on the permit is received
    ///     assert_eq!(rx.recv().await.unwrap(), 456);
    /// }
    /// ```
    pub async fn reserve(&self) -> Result<Permit<'_, T>, error::SendError<()>> {
        self.inner.reserve().await
    }

    /// Waits for channel capacity, moving the `Sender` and returning an owned
    /// permit. Once capacity to send one message is available, it is reserved
    /// for the caller.
    ///
    /// This moves the sender _by value_, and returns an owned permit that can
    /// be used to send a message into the channel. Unlike [`Sender::reserve`],
    /// this method may be used in cases where the permit must be valid for the
    /// `'static` lifetime. `Sender`s may be cloned cheaply (`Sender::clone` is
    /// essentially a reference count increment, comparable to [`Arc::clone`]),
    /// so when multiple [`OwnedPermit`]s are needed or the `Sender` cannot be
    /// moved, it can be cloned prior to calling `reserve_owned`.
    ///
    /// If the channel is full, the function waits for the number of unreceived
    /// messages to become less than the channel capacity. Capacity to send one
    /// message is reserved for the caller. An [`OwnedPermit`] is returned to
    /// track the reserved capacity. The [`send`] function on [`OwnedPermit`]
    /// consumes the reserved capacity.
    ///
    /// Dropping the [`OwnedPermit`] without sending a message releases the
    /// capacity back to the channel.
    ///
    /// # Cancel safety
    ///
    /// This channel uses a queue to ensure that calls to `send` and `reserve`
    /// complete in the order they were requested.  Cancelling a call to
    /// `reserve_owned` makes you lose your place in the queue.
    ///
    /// # Examples
    /// Sending a message using an [`OwnedPermit`]:
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     // Reserve capacity, moving the sender.
    ///     let permit = tx.reserve_owned().await.unwrap();
    ///
    ///     // Send a message, consuming the permit and returning
    ///     // the moved sender.
    ///     let tx = permit.send(123);
    ///
    ///     // The value sent on the permit is received.
    ///     assert_eq!(rx.recv().await.unwrap(), 123);
    ///
    ///     // The sender can now be used again.
    ///     tx.send(456).await.unwrap();
    /// }
    /// ```
    ///
    /// When multiple [`OwnedPermit`]s are needed, or the sender cannot be moved
    /// by value, it can be inexpensively cloned before calling `reserve_owned`:
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     // Clone the sender and reserve capacity.
    ///     let permit = tx.clone().reserve_owned().await.unwrap();
    ///
    ///     // Trying to send directly on the `tx` will fail due to no
    ///     // available capacity.
    ///     assert!(tx.try_send(123).is_err());
    ///
    ///     // Sending on the permit succeeds.
    ///     permit.send(456);
    ///
    ///     // The value sent on the permit is received
    ///     assert_eq!(rx.recv().await.unwrap(), 456);
    /// }
    /// ```
    ///
    /// [`Sender::reserve`]: Sender::reserve
    /// [`OwnedPermit`]: OwnedPermit
    /// [`send`]: OwnedPermit::send
    /// [`Arc::clone`]: std::sync::Arc::clone
    pub fn try_reserve(&self) -> Result<Permit<'_, T>, error::TrySendError<()>> {
        self.inner.try_reserve()
    }

    // Tries to acquire a slot in the channel without waiting for the slot to become
    /// available.
    ///
    /// If the channel is full this function will return [`TrySendError`](error::TrySendError), otherwise
    /// if there is a slot available it will return a [`Permit`] that will then allow you
    /// to [`send`] on the channel with a guaranteed slot. This function is similar to
    /// [`reserve`] except it does not await for the slot to become available.
    ///
    /// Dropping [`Permit`] without sending a message releases the capacity back
    /// to the channel.
    ///
    /// [`Permit`]: Permit
    /// [`send`]: Permit::send
    /// [`reserve`]: Sender::reserve
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     // Reserve capacity
    ///     let permit = tx.try_reserve().unwrap();
    ///
    ///     // Trying to send directly on the `tx` will fail due to no
    ///     // available capacity.
    ///     assert!(tx.try_send(123).is_err());
    ///
    ///     // Trying to reserve an additional slot on the `tx` will
    ///     // fail because there is no capacity.
    ///     assert!(tx.try_reserve().is_err());
    ///
    ///     // Sending on the permit succeeds
    ///     permit.send(456);
    ///
    ///     // The value sent on the permit is received
    ///     assert_eq!(rx.recv().await.unwrap(), 456);
    ///
    /// }
    /// ```
    pub async fn reserve_owned(self) -> Result<OwnedPermit<T>, error::SendError<()>> {
        self.inner.reserve_owned().await
    }

    /// Tries to acquire a slot in the channel without waiting for the slot to become
    /// available, returning an owned permit.
    ///
    /// This moves the sender _by value_, and returns an owned permit that can
    /// be used to send a message into the channel. Unlike [`Sender::try_reserve`],
    /// this method may be used in cases where the permit must be valid for the
    /// `'static` lifetime.  `Sender`s may be cloned cheaply (`Sender::clone` is
    /// essentially a reference count increment, comparable to [`Arc::clone`]),
    /// so when multiple [`OwnedPermit`]s are needed or the `Sender` cannot be
    /// moved, it can be cloned prior to calling `try_reserve_owned`.
    ///
    /// If the channel is full this function will return a [`TrySendError`](error::TrySendError).
    /// Since the sender is taken by value, the `TrySendError` returned in this
    /// case contains the sender, so that it may be used again. Otherwise, if
    /// there is a slot available, this method will return an [`OwnedPermit`]
    /// that can then be used to [`send`] on the channel with a guaranteed slot.
    /// This function is similar to  [`reserve_owned`] except it does not await
    /// for the slot to become available.
    ///
    /// Dropping the [`OwnedPermit`] without sending a message releases the capacity back
    /// to the channel.
    ///
    /// [`OwnedPermit`]: OwnedPermit
    /// [`send`]: OwnedPermit::send
    /// [`reserve_owned`]: Sender::reserve_owned
    /// [`Arc::clone`]: std::sync::Arc::clone
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     // Reserve capacity
    ///     let permit = tx.clone().try_reserve_owned().unwrap();
    ///
    ///     // Trying to send directly on the `tx` will fail due to no
    ///     // available capacity.
    ///     assert!(tx.try_send(123).is_err());
    ///
    ///     // Trying to reserve an additional slot on the `tx` will
    ///     // fail because there is no capacity.
    ///     assert!(tx.try_reserve().is_err());
    ///
    ///     // Sending on the permit succeeds
    ///     permit.send(456);
    ///
    ///     // The value sent on the permit is received
    ///     assert_eq!(rx.recv().await.unwrap(), 456);
    ///
    /// }
    /// ```
    pub fn try_reserve_owned(self) -> Result<OwnedPermit<T>, error::TrySendError<Self>> {
        let Self { globals, inner } = self;
        inner.try_reserve_owned().map_err(|e| {
            use error::*;
            match e {
                TrySendError::Closed(sender) => TrySendError::Closed(Self {
                    inner: sender,
                    globals,
                }),
                TrySendError::Full(sender) => TrySendError::Full(Self {
                    inner: sender,
                    globals,
                }),
            }
        })
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            globals: Arc::clone(&self.globals),
        }
    }
}

impl<T> Debug for Sender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// Receives values from the associated `Sender`.
///
/// Instances are created by the [`channel`](channel) function.
///
/// This receiver can be turned into a `Stream` using [`ReceiverStream`].
///
/// [`ReceiverStream`]: https://docs.rs/tokio-stream/0.1/tokio_stream/wrappers/struct.ReceiverStream.html
pub struct Receiver<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
    globals: Arc<Globals>,
}

impl<T> Receiver<T> {
    /// Closes the receiving half of a channel without dropping it.
    ///
    /// This prevents any further messages from being sent on the channel while
    /// still enabling the receiver to drain messages that are buffered. Any
    /// outstanding [`Permit`] values will still be able to send messages.
    ///
    /// To guarantee that no messages are dropped, after calling `close()`,
    /// `recv()` must be called until `None` is returned. If there are
    /// outstanding [`Permit`] or [`OwnedPermit`] values, the `recv` method will
    /// not return `None` until those are released.
    ///
    /// [`Permit`]: Permit
    /// [`OwnedPermit`]: OwnedPermit
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(20);
    ///
    ///     tokio::spawn(async move {
    ///         let mut i = 0;
    ///         while let Ok(permit) = tx.reserve().await {
    ///             permit.send(i);
    ///             i += 1;
    ///         }
    ///     });
    ///
    ///     rx.close();
    ///
    ///     while let Some(msg) = rx.recv().await {
    ///         println!("got {}", msg);
    ///     }
    ///
    ///     // Channel closed and no messages are lost.
    /// }
    /// ```
    pub fn close(&mut self) {
        self.inner.close()
    }

    /// Receives the next value for this receiver.
    ///
    /// This method returns `None` if the channel has been closed and there are
    /// no remaining messages in the channel's buffer. This indicates that no
    /// further values can ever be received from this `Receiver`. The channel is
    /// closed when all senders have been dropped, or when [`close`] is called.
    ///
    /// If there are no messages in the channel's buffer, but the channel has
    /// not yet been closed, this method will sleep until a message is sent or
    /// the channel is closed.  Note that if [`close`] is called, but there are
    /// still outstanding [`Permits`] from before it was closed, the channel is
    /// not considered closed by `recv` until the permits are released.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. If `recv` is used as the event in a
    /// [`tokio::select!`](tokio::select) statement and some other branch
    /// completes first, it is guaranteed that no messages were received on this
    /// channel.
    ///
    /// [`close`]: Self::close
    /// [`Permits`]: struct@crate::async_sim::sync::mpsc::Permit
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///
    ///     tokio::spawn(async move {
    ///         tx.send("hello").await.unwrap();
    ///     });
    ///
    ///     assert_eq!(Some("hello"), rx.recv().await);
    ///     assert_eq!(None, rx.recv().await);
    /// }
    /// ```
    ///
    /// Values are buffered:
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///
    ///     tx.send("hello").await.unwrap();
    ///     tx.send("world").await.unwrap();
    ///
    ///     assert_eq!(Some("hello"), rx.recv().await);
    ///     assert_eq!(Some("world"), rx.recv().await);
    /// }
    /// ```
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await.map(|v| {
            self.globals.decrement_count();
            v
        })
    }

    /// Tries to receive the next value for this receiver.
    ///
    /// This method returns the [`Empty`] error if the channel is currently
    /// empty, but there are still outstanding [senders] or [permits].
    ///
    /// This method returns the [`Disconnected`] error if the channel is
    /// currently empty, and there are no outstanding [senders] or [permits].
    ///
    /// Unlike the [`poll_recv`] method, this method will never return an
    /// [`Empty`] error spuriously.
    ///
    /// [`Empty`]: crate::async_sim::sync::mpsc::error::TryRecvError::Empty
    /// [`Disconnected`]: crate::async_sim::sync::mpsc::error::TryRecvError::Disconnected
    /// [`poll_recv`]: Self::poll_recv
    /// [senders]: crate::async_sim::sync::mpsc::Sender
    /// [permits]: crate::async_sim::sync::mpsc::Permit
    ///
    /// # Examples
    ///
    /// ```
    /// use des::async_sim::sync::mpsc;
    /// use des::async_sim::sync::mpsc::error::TryRecvError;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::channel(100);
    ///
    ///     tx.send("hello").await.unwrap();
    ///
    ///     assert_eq!(Ok("hello"), rx.try_recv());
    ///     assert_eq!(Err(TryRecvError::Empty), rx.try_recv());
    ///
    ///     tx.send("hello").await.unwrap();
    ///     // Drop the last sender, closing the channel.
    ///     drop(tx);
    ///
    ///     assert_eq!(Ok("hello"), rx.try_recv());
    ///     assert_eq!(Err(TryRecvError::Disconnected), rx.try_recv());
    /// }
    /// ```
    pub fn try_recv(&mut self) -> Result<T, error::TryRecvError> {
        self.inner.try_recv().map(|v| {
            self.globals.decrement_count();
            v
        })
    }

    /// Blocking receive to call outside of asynchronous contexts.
    ///
    /// This method returns `None` if the channel has been closed and there are
    /// no remaining messages in the channel's buffer. This indicates that no
    /// further values can ever be received from this `Receiver`. The channel is
    /// closed when all senders have been dropped, or when [`close`] is called.
    ///
    /// If there are no messages in the channel's buffer, but the channel has
    /// not yet been closed, this method will block until a message is sent or
    /// the channel is closed.
    ///
    /// This method is intended for use cases where you are sending from
    /// asynchronous code to synchronous code, and will work even if the sender
    /// is not using [`blocking_send`] to send the message.
    ///
    /// Note that if [`close`] is called, but there are still outstanding
    /// [`Permits`] from before it was closed, the channel is not considered
    /// closed by `blocking_recv` until the permits are released.
    ///
    /// [`close`]: Self::close
    /// [`Permits`]: struct@crate::async_sim::sync::mpsc::Permit
    /// [`blocking_send`]: fn@crate::async_sim::sync::mpsc::Sender::blocking_send
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use tokio::runtime::Runtime;
    /// use des::async_sim::sync::mpsc;
    ///
    /// fn main() {
    ///     let (tx, mut rx) = mpsc::channel::<u8>(10);
    ///
    ///     let sync_code = thread::spawn(move || {
    ///         assert_eq!(Some(10), rx.blocking_recv());
    ///     });
    ///
    ///     Runtime::new()
    ///         .unwrap()
    ///         .block_on(async move {
    ///             let _ = tx.send(10).await;
    ///         });
    ///     sync_code.join().unwrap()
    /// }
    /// ```
    pub fn blocking_recv(&mut self) -> Option<T> {
        self.inner.blocking_recv().map(|v| {
            self.globals.decrement_count();
            v
        })
    }

    /// Polls to receive the next message on this channel.
    ///
    /// This method returns:
    ///
    ///  * `Poll::Pending` if no messages are available but the channel is not
    ///    closed, or if a spurious failure happens.
    ///  * `Poll::Ready(Some(message))` if a message is available.
    ///  * `Poll::Ready(None)` if the channel has been closed and all messages
    ///    sent before it was closed have been received.
    ///
    /// When the method returns `Poll::Pending`, the `Waker` in the provided
    /// `Context` is scheduled to receive a wakeup when a message is sent on any
    /// receiver, or when the channel is closed.  Note that on multiple calls to
    /// `poll_recv`, only the `Waker` from the `Context` passed to the most
    /// recent call is scheduled to receive a wakeup.
    ///
    /// If this method returns `Poll::Pending` due to a spurious failure, then
    /// the `Waker` will be notified when the situation causing the spurious
    /// failure has been resolved. Note that receiving such a wakeup does not
    /// guarantee that the next call will succeed — it could fail with another
    /// spurious failure.
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.inner.poll_recv(cx).map(|v| {
            if v.is_some() {
                self.globals.decrement_count();
            }
            v
        })
    }

    /// Receives the next value for this receiver, calling
    /// the provided callback once obtained.
    ///
    /// This funtions behaves almost identically to [Receiver::recv] with
    /// the slight difference that the value count in the globals is
    /// updated after the callback was executed, not immiedatly after
    /// the value is received.
    pub async fn scoped_recv<F, R>(&mut self, f: impl FnOnce(Option<T>) -> F) -> R
    where
        F: Future<Output = R>,
    {
        let item = self.inner.recv().await;
        let result = f(item).await;
        self.globals.decrement_count();
        result
    }
}

impl<T> Debug for Receiver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> Unpin for Receiver<T> {}
