use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use tokio::sync::mpsc::error;

use super::Globals;

/// Creates an unbounded mpsc channel for communicating between asynchronous
/// tasks without backpressure, that will be monitored by the provided observer.
///
/// A `send` on this channel will always succeed as long as the receive half has
/// not been closed. If the receiver falls behind, messages will be arbitrarily
/// buffered.
///
/// **Note** that the amount of available system memory is an implicit bound to
/// the channel. Using an `unbounded` channel has the ability of causing the
/// process to run out of memory. In this case, the process will be aborted.
pub fn unbounded_channel_watched<T>(
    globals: Arc<Globals>,
) -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (
        UnboundedSender {
            inner: tx,
            globals: Arc::clone(&globals),
        },
        UnboundedReceiver { inner: rx, globals },
    )
}

/// Creates an unbounded mpsc channel for communicating between asynchronous
/// tasks without backpressure, that will not be monitored.
///
/// A `send` on this channel will always succeed as long as the receive half has
/// not been closed. If the receiver falls behind, messages will be arbitrarily
/// buffered.
///
/// **Note** that the amount of available system memory is an implicit bound to
/// the channel. Using an `unbounded` channel has the ability of causing the
/// process to run out of memory. In this case, the process will be aborted.
pub fn unbounded_channel_unwatched<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    unbounded_channel_watched(Globals::new())
}

///
/// An alias for [unbounded_channel_unwatched].
/// This alias exists to gurantee compatibility with existing tokio projects
/// to create non-supervied channel. Use either [unbounded_channel_unwatched] or
/// [unbounded_channel_watched] for specific behaviour inside a simulation context
///
#[deprecated]
pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    unbounded_channel_unwatched()
}

/// Send values to the associated `UnboundedReceiver`.
///
/// Instances are created by the
/// [`unbounded_channel_unwatched`](unbounded_channel_unwatched) or
/// [`unbounded_channel_watched`](unbounded_channel_watched) functions.
pub struct UnboundedSender<T> {
    inner: tokio::sync::mpsc::UnboundedSender<T>,
    globals: Arc<Globals>,
}

impl<T> UnboundedSender<T> {
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
    /// # use des_tokio as des;
    /// use des::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx1, rx) = mpsc::unbounded_channel::<()>();
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
    ////     println!("Receiver dropped");
    /// }
    /// ```
    pub async fn closed(&self) {
        self.inner.closed().await
    }

    /// Checks if the channel has been closed. This happens when the
    /// [`UnboundedReceiver`] is dropped, or when the
    /// [`UnboundedReceiver::close`] method is called.
    ///
    /// [`UnboundedReceiver`]: crate::sync::mpsc::UnboundedReceiver
    /// [`UnboundedReceiver::close`]: crate::sync::mpsc::UnboundedReceiver::close
    ///
    /// ```
    /// # use des_tokio as des;
    /// use des::sync::mpsc;
    ///
    /// let (tx, rx) = mpsc::unbounded_channel::<()>();
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
    /// # use des_tokio as des;
    /// use des::sync::mpsc;
    /// let (tx, rx) = mpsc::unbounded_channel::<()>();
    /// let  tx2 = tx.clone();
    /// assert!(tx.same_channel(&tx2));
    ///
    /// let (tx3, rx3) = mpsc::unbounded_channel::<()>();
    /// assert!(!tx3.same_channel(&tx2));
    /// ```
    pub fn same_channel(&self, other: &Self) -> bool {
        self.inner.same_channel(&other.inner)
    }

    /// Attempts to send a message on this `UnboundedSender` without blocking.
    ///
    /// This method is not marked async because sending a message to an unbounded channel
    /// never requires any form of waiting. Because of this, the `send` method can be
    /// used in both synchronous and asynchronous code without problems.
    ///
    /// If the receive half of the channel is closed, either due to [`close`]
    /// being called or the [`UnboundedReceiver`] having been dropped, this
    /// function returns an error. The error includes the value passed to `send`.
    ///
    /// [`close`]: UnboundedReceiver::close
    /// [`UnboundedReceiver`]: UnboundedReceiver
    pub fn send(&self, value: T) -> Result<(), error::SendError<T>> {
        self.globals.increment_count();
        self.inner.send(value).map_err(|e| {
            self.globals.decrement_count();
            e
        })
    }
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            globals: Arc::clone(&self.globals),
        }
    }
}

impl<T> Debug for UnboundedSender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// Receive values from the associated `UnboundedSender`.
///
/// Instances are created by the
/// [`unbounded_channel_watched`](unbounded_channel_watched) or
/// [`unbounded_channel_unwatched`](unbounded_channel_unwatched) functions.
pub struct UnboundedReceiver<T> {
    inner: tokio::sync::mpsc::UnboundedReceiver<T>,
    globals: Arc<Globals>,
}

impl<T> UnboundedReceiver<T> {
    /// Closes the receiving half of a channel, without dropping it.
    ///
    /// This prevents any further messages from being sent on the channel while
    /// still enabling the receiver to drain messages that are buffered.
    ///
    /// To guarantee that no messages are dropped, after calling `close()`,
    /// `recv()` must be called until `None` is returned.
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
    /// the channel is closed.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. If `recv` is used as the event in a
    /// [`tokio::select!`](tokio::select) statement and some other branch
    /// completes first, it is guaranteed that no messages were received on this
    /// channel.
    ///
    /// [`close`]: Self::close
    ///
    /// # Examples
    ///
    /// ```
    /// # use des_tokio as des;
    /// use des::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::unbounded_channel();
    ///
    ///     tokio::spawn(async move {
    ///         tx.send("hello").unwrap();
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
    /// # use des_tokio as des;
    /// use des::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::unbounded_channel();
    ///
    ///     tx.send("hello").unwrap();
    ///     tx.send("world").unwrap();
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
    /// [`Empty`]: crate::sync::mpsc::error::TryRecvError::Empty
    /// [`Disconnected`]: crate::sync::mpsc::error::TryRecvError::Disconnected
    /// [`poll_recv`]: Self::poll_recv
    /// [senders]: crate::sync::mpsc::Sender
    /// [permits]: crate::sync::mpsc::Permit
    ///
    /// # Examples
    ///
    /// ```
    /// # use des_tokio as des;
    /// use des::sync::mpsc;
    /// use des::sync::mpsc::error::TryRecvError;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::unbounded_channel();
    ///
    ///     tx.send("hello").unwrap();
    ///
    ///     assert_eq!(Ok("hello"), rx.try_recv());
    ///     assert_eq!(Err(TryRecvError::Empty), rx.try_recv());
    ///
    ///     tx.send("hello").unwrap();
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
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des_tokio as des;
    /// use std::thread;
    /// use des::sync::mpsc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (tx, mut rx) = mpsc::unbounded_channel::<u8>();
    ///
    ///     let sync_code = thread::spawn(move || {
    ///         assert_eq!(Some(10), rx.blocking_recv());
    ///     });
    ///
    ///     let _ = tx.send(10);
    ///     sync_code.join().unwrap();
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
    /// This funtions behaves almost identically to [UnboundedReceiver::recv] with
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

impl<T> Debug for UnboundedReceiver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
