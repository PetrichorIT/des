use super::{
    driver::{Driver, TimerSlotEntry, TimerSlotEntryHandle},
    SimTime,
};
use pin_project_lite::pin_project;
use std::{
    cmp::Ordering, future::Future, pin::Pin, sync::atomic::AtomicUsize, task::Poll, time::Duration,
};

/// Waits until `duration` has elapsed.
///
/// Equivalent to `sleep_until(Instant::now() + duration)`. An asynchronous
/// analog to `std::thread::sleep`.
///
/// No work is performed while awaiting on the sleep future to complete. `Sleep`
/// operates at millisecond granularity and should not be used for tasks that
/// require high-resolution timers. The implementation is platform specific,
/// and some platforms (specifically Windows) will provide timers with a
/// larger resolution than 1 ms.
///
/// To run something regularly on a schedule, see [`interval`].
///
/// The maximum duration for a sleep is 68719476734 milliseconds (approximately 2.2 years).
///
/// # Cancellation
///
/// Canceling a sleep instance is done by dropping the returned future. No additional
/// cleanup work is required.
///
/// # Panics
///
/// This function panics whenever a timer is created outside of a
/// Tokio runtime. That is why `rt.block_on(sleep(...))` will panic,
/// since the function is executed outside of the runtime.
/// Whereas `rt.block_on(async {sleep(...).await})` doesn't panic.
/// And this is because wrapping the function on an async makes it lazy,
/// and so gets executed inside the runtime successfully without
/// panicking.
///
/// [`Sleep`]: struct@crate::time::Sleep
/// [`interval`]: crate::time::interval()
/// [`Builder::enable_time`]: crate::runtime::Builder::enable_time
/// [`Builder::enable_all`]: crate::runtime::Builder::enable_all
pub fn sleep(duration: Duration) -> Sleep {
    match SimTime::now().checked_add(duration) {
        Some(deadline) => Sleep::new(deadline),
        None => Sleep::far_future(),
    }
}

/// Waits until `deadline` is reached.
///
/// No work is performed while awaiting on the sleep future to complete. `Sleep`
/// operates at millisecond granularity and should not be used for tasks that
/// require high-resolution timers.
///
/// To run something regularly on a schedule, see [`interval`].
///
/// # Cancellation
///
/// Canceling a sleep instance is done by dropping the returned future. No additional
/// cleanup work is required.
///
/// # Panics
///
/// This function panics whenever a timer is created outside of a
/// Tokio runtime. That is why `rt.block_on(sleep(...))` will panic,
/// since the function is executed outside of the runtime.
/// Whereas `rt.block_on(async {sleep(...).await})` doesn't panic.
/// And this is because wrapping the function on an async makes it lazy,
/// and so gets executed inside the runtime successfully without
/// panicking.
///
/// [`Sleep`]: struct@crate::time::Sleep
/// [`interval`]: crate::time::interval()
/// [`Builder::enable_time`]: crate::runtime::Builder::enable_time
/// [`Builder::enable_all`]: crate::runtime::Builder::enable_all
pub fn sleep_until(deadline: SimTime) -> Sleep {
    Sleep::new(deadline)
}

pin_project! {
    /// Future returned by [`sleep`](sleep) and [`sleep_until`](sleep_until).
    ///
    /// This type does not implement the `Unpin` trait, which means that if you
    /// use it with [`select!`] or by calling `poll`, you have to pin it first.
    /// If you use it with `.await`, this does not apply.
    ///
    #[project(!Unpin)]
    #[must_use  = "futures do nothing unless you `.await` or poll them"]
    #[derive(Debug)]
    pub struct Sleep {
        deadline: SimTime,
        id: usize,

        #[pin]
        handle: Option<TimerSlotEntryHandle>
    }
}

static SLEEP_ID: AtomicUsize = AtomicUsize::new(0);

impl Sleep {
    pub(super) fn new(deadline: SimTime) -> Sleep {
        let next = SLEEP_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Sleep {
            deadline,
            id: next,
            handle: None,
        }
    }

    pub(super) fn far_future() -> Sleep {
        Self::new(SimTime::MAX)
    }

    /// Returns the instant at which the future will complete.
    #[must_use]
    pub fn deadline(&self) -> SimTime {
        self.deadline
    }

    /// Returns `true` if `Sleep` has elapsed.
    ///
    /// A `Sleep` instance is elapsed when the requested duration has elapsed.
    #[must_use]
    pub fn is_elapsed(&self) -> bool {
        self.deadline <= SimTime::now()
    }

    /// Resets the `Sleep` instance to a new deadline.
    ///
    /// Calling this function allows changing the instant at which the `Sleep`
    /// future completes without having to create new associated state.
    ///
    /// This function can be called both before and after the future has
    /// completed.
    ///
    /// To call this method, you will usually combine the call with
    /// [`Pin::as_mut`], which lets you call the method without consuming the
    /// `Sleep` itself.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use des::time::{Duration, Instant};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let sleep = des::time::sleep(Duration::from_millis(10));
    /// tokio::pin!(sleep);
    ///
    /// sleep.as_mut().reset(Instant::now() + Duration::from_millis(20));
    /// # }
    /// ```
    ///
    /// See also the top-level examples.
    ///
    /// [`Pin::as_mut`]: fn@std::pin::Pin::as_mut
    pub fn reset(self: Pin<&mut Self>, deadline: SimTime) {
        self.reset_inner(deadline);
    }

    fn reset_inner(self: Pin<&mut Self>, deadline: SimTime) {
        let mut me = self.project();
        if let Some(handle) = me.handle.take() {
            // Reogranize timer calls.
            handle.reset(deadline);
        }
        *me.deadline = deadline;
    }
}

impl Future for Sleep {
    type Output = ();
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // A poll might come from one of three sources
        // a) inital poll
        // b) poll at scheduled deadline
        // c) spurious wakeup i.e. tokio::select

        let scheduled = self.handle.is_some();

        // Project initaly

        let mut me = self.project();
        match (*me.deadline).cmp(&SimTime::now()) {
            Ordering::Greater => {
                if !scheduled {
                    let handle = Driver::with_current(|ctx| {
                        ctx.queue.add(
                            TimerSlotEntry {
                                id: *me.id,
                                waker: cx.waker().clone(),
                            },
                            *me.deadline,
                        )
                    });
                    *me.handle = Some(handle);
                }
                Poll::Pending
            }
            _ => {
                if let Some(mut handle) = me.handle.take() {
                    handle.resolve();
                }
                Poll::Ready(())
            }
        }
    }
}
