//! Allows a future to execute for a maximum amount of time.
//!
//! See [`Timeout`] documentation for more details.
//!
//! [`Timeout`]: struct@Timeout

use std::future::Future;
use std::time::Duration;

use super::SimTime;

pub use tokio::time::Timeout;

/// Requires a `Future` to complete before the specified duration has elapsed.
///
/// If the future completes before the duration has elapsed, then the completed
/// value is returned. Otherwise, an error is returned and the future is
/// canceled.
///
/// # Cancelation
///
/// Cancelling a timeout is done by dropping the future. No additional cleanup
/// or other work is required.
///
/// The original future may be obtained by calling [`Timeout::into_inner`]. This
/// consumes the `Timeout`.
///
/// # Examples
///
/// Create a new `Timeout` set to expire in 10 milliseconds.
///
/// ```rust
/// use des::time::timeout;
/// use tokio::sync::oneshot;
///
/// use std::time::Duration;
///
/// # async fn dox() {
/// let (tx, rx) = oneshot::channel();
/// # tx.send(()).unwrap();
///
/// // Wrap the future with a `Timeout` set to expire in 10 milliseconds.
/// if let Err(_) = timeout(Duration::from_millis(10), rx).await {
///     println!("did not receive value within 10 ms");
/// }
/// # }
/// ```
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
#[track_caller]
pub fn timeout<T>(duration: Duration, future: T) -> Timeout<T>
where
    T: Future,
{
    tokio::time::timeout(duration, future)
}

/// Requires a `Future` to complete before the specified instant in time.
///
/// If the future completes before the instant is reached, then the completed
/// value is returned. Otherwise, an error is returned.
///
/// # Cancelation
///
/// Cancelling a timeout is done by dropping the future. No additional cleanup
/// or other work is required.
///
/// The original future may be obtained by calling [`Timeout::into_inner`]. This
/// consumes the `Timeout`.
///
/// # Examples
///
/// Create a new `Timeout` set to expire in 10 milliseconds.
///
/// ```rust
/// use des::time::{SimTime, timeout_at};
/// use tokio::sync::oneshot;
///
/// use std::time::Duration;
///
/// # async fn dox() {
/// let (tx, rx) = oneshot::channel();
/// # tx.send(()).unwrap();
///
/// // Wrap the future with a `Timeout` set to expire 10 milliseconds into the
/// // future.
/// if let Err(_) = timeout_at(SimTime::now() + Duration::from_millis(10), rx).await {
///     println!("did not receive value within 10 ms");
/// }
/// # }
/// ```
pub fn timeout_at<T>(deadline: SimTime, future: T) -> Timeout<T>
where
    T: Future,
{
    let duration = deadline.saturating_sub(SimTime::now().0);
    tokio::time::timeout(duration, future)
}
