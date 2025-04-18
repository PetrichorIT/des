//!
//! Temporal quantification in a simulation context.
//!
//! Note that the implementation of [`SimTime`] depends on the features that
//! are active. If features "async" is active, tokio provides an implementation
//! for [`SimTime`] based on its internal feature "sim". If not a drop-in replacement
//! is provided by des.
//!
//! # Examples
//!
//! A [`Duration`] describes a span of time, either in the context of
//! real [`SystemTime`](std::time::SystemTime) or provided [`SimTime`].
//! There are mutiple ways to create a new [`Duration`].
//!
//! ```rust
//! # use des::time::*;
//! let five_seconds = Duration::from_secs(5);
//! assert_eq!(five_seconds, Duration::from_millis(5_000));
//! assert_eq!(five_seconds, Duration::from_micros(5_000_000));
//! assert_eq!(five_seconds, Duration::from_nanos(5_000_000_000));
//!
//! let ten_seconds = Duration::from_secs(10);
//! let seven_nanos = Duration::from_nanos(7);
//! let total = ten_seconds + seven_nanos;
//! assert_eq!(total, Duration::new(10, 7));
//! ```

mod duration;
pub use duration::*;
use serde::de::Visitor;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Display};
use std::ops::{Deref, Div, Sub, SubAssign};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

cfg_async! {
    pub mod error;

    mod driver;
    pub(crate) use driver::*;

    mod sleep;
    pub use sleep::*;

    mod timeout;
    pub use timeout::*;

    mod interval;
    pub use interval::*;
}

static SIMTIME: (AtomicU64, AtomicU32) = (AtomicU64::new(0), AtomicU32::new(0));

///
/// A specific point of time in the simulation.
///
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]

pub struct SimTime(Duration);

impl SimTime {
    /// Returns an instant corresponding to "now" in the simulation context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use des::prelude::SimTime;
    ///
    /// let now = SimTime::now();
    /// ```
    #[must_use]
    pub fn now() -> Self {
        SimTime(Duration::new(
            SIMTIME.0.load(Ordering::SeqCst),
            SIMTIME.1.load(Ordering::SeqCst),
        ))
    }

    ///
    /// Sets the sim time
    ///
    pub(crate) fn set_now(time: SimTime) {
        SIMTIME.0.store(time.as_secs(), Ordering::SeqCst);
        SIMTIME.1.store(time.subsec_nanos(), Ordering::SeqCst);
    }

    ///
    /// Constructs an instance of `SimTime` from a give duration since `SimTime::ZERO`.
    ///
    #[must_use]
    pub const fn from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    ///
    /// Makes an equallity check with an error margin.
    ///
    #[must_use]
    pub fn eq_approx(&self, other: SimTime, error: Duration) -> bool {
        let dur = self.duration_diff(other);
        dur < error
    }

    /// Retursn the amount of time elapsed from the earlier of the two values
    /// to the higher.
    #[must_use]
    pub fn duration_diff(&self, other: SimTime) -> Duration {
        if *self > other {
            self.duration_since(other)
        } else {
            other.duration_since(*self)
        }
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    ///
    /// # Panics
    ///
    /// This function  panics of the checked operation fails.
    #[must_use]
    pub fn duration_since(&self, earlier: SimTime) -> Duration {
        self.checked_duration_since(earlier)
            .expect("duration subtraction invalid")
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or None if that instant is later than this one.
    #[must_use]
    pub fn checked_duration_since(&self, earlier: SimTime) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    #[must_use]
    pub fn saturating_duration_since(&self, earlier: SimTime) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    /// Returns the amount of time elapsed since this instant was created.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Self::now() - *self
    }

    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    #[must_use]
    pub fn checked_add(&self, duration: Duration) -> Option<SimTime> {
        self.0.checked_add(duration).map(SimTime)
    }

    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    #[must_use]
    pub fn checked_sub(&self, duration: Duration) -> Option<SimTime> {
        self.0.checked_sub(duration).map(SimTime)
    }
}

// # Custom Additions
impl SimTime {
    /// The smallest instance of a [`SimTime`].
    pub const ZERO: SimTime = SimTime(Duration::ZERO);
    /// The smallest valid instance of a [`SimTime`].
    pub const MIN: SimTime = SimTime(Duration::ZERO);
    /// The greatest instance of a [`SimTime`].
    pub const MAX: SimTime = SimTime(Duration::MAX);
}

// Serialize

impl Serialize for SimTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_f64(self.as_secs_f64())
        } else {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("secs", &self.as_secs())?;
            map.serialize_entry("nanos", &self.subsec_nanos())?;
            map.end()
        }
    }
}

impl<'de> Deserialize<'de> for SimTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SimTimeVisitor;
        impl<'de> Visitor<'de> for SimTimeVisitor {
            type Value = SimTime;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an positive floating point value or an encoded Duration")
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(SimTime::from_duration(Duration::from_secs_f64(v)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut secs = 0;
                let mut nanos = 0;
                while let Some(key) = map.next_key()? {
                    match key {
                        "secs" => secs = map.next_value()?,
                        "nanos" => nanos = map.next_value()?,
                        _ => return Err(serde::de::Error::unknown_field(key, &["secs", "nanos"])),
                    }
                }
                Ok(SimTime::from_duration(Duration::new(secs, nanos)))
            }
        }

        deserializer.deserialize_any(SimTimeVisitor)
    }
}

// CMP

impl PartialEq<f64> for SimTime {
    fn eq(&self, other: &f64) -> bool {
        let diff = (self.0.as_secs_f64() - *other).abs();
        diff < f64::EPSILON
    }
}

// OPS

impl Sub<Duration> for SimTime {
    type Output = SimTime;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs)
            .expect("Overflow when substracting Duration from SimTime")
    }
}

impl SubAssign<Duration> for SimTime {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl Sub<SimTime> for SimTime {
    type Output = Duration;

    fn sub(self, rhs: SimTime) -> Self::Output {
        self.duration_since(rhs)
    }
}

impl Div<SimTime> for SimTime {
    type Output = f64;

    fn div(self, rhs: SimTime) -> Self::Output {
        self.0.as_secs_f64() / rhs.0.as_secs_f64()
    }
}

impl Div<f64> for SimTime {
    type Output = SimTime;

    fn div(self, rhs: f64) -> Self::Output {
        Self::from(self.0.as_secs_f64() / rhs)
    }
}

// DEREF

impl Deref for SimTime {
    type Target = Duration;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// FMT

impl Debug for SimTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for SimTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

// FROM

impl From<SimTime> for f64 {
    fn from(this: SimTime) -> Self {
        this.0.as_secs_f64()
    }
}

impl From<f64> for SimTime {
    fn from(value: f64) -> Self {
        SimTime(Duration::from_secs_f64(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops() {
        assert_eq!(
            f64::from(SimTime::from_duration(Duration::from_millis(300))),
            0.3
        );

        assert_eq!(SimTime::from(60.0) / 3.0, SimTime::from(20.0));
        assert_eq!(SimTime::from(60.0) / SimTime::from(3.0), 20.0);

        assert_eq!(
            SimTime::from(30.0) - SimTime::from(10.0),
            Duration::from_secs(20)
        );
        assert_eq!(SimTime::from(30.0) - Duration::from_secs(10), 20.0);
        let mut time = SimTime::from(30.0);
        time -= Duration::from_secs(10);
        assert_eq!(time, 20.0);
    }
}
