//!
//! Time primitives mirroring [std::time] bound to the simulation time.
//!

use crate::runtime::sim_time;

use std::f64::EPSILON;
use std::fmt::*;
use std::ops::*;

mod duration;
pub use duration::*;

// Reexport

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
        sim_time()
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    #[must_use]
    pub fn duration_since(&self, earlier: SimTime) -> Duration {
        self.checked_duration_since(earlier).unwrap()
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
    pub const ZERO: SimTime = SimTime(Duration::ZERO);
    pub const MIN: SimTime = SimTime(Duration::ZERO);
    pub const MAX: SimTime = SimTime(Duration::MAX);
}

// CMP

impl PartialEq<f64> for SimTime {
    fn eq(&self, other: &f64) -> bool {
        let diff = (self.0.as_secs_f64() - *other).abs();
        diff < EPSILON
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
        *self = *self - rhs
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

// FMT

impl Debug for SimTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for SimTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

// FROM

// impl From<SimTime> for f32 {
//     fn from(this: SimTime) -> Self {
//         this.0.as_secs_f32()
//     }
// }

impl From<SimTime> for f64 {
    fn from(this: SimTime) -> Self {
        this.0.as_secs_f64()
    }
}

impl From<f64> for SimTime {
    fn from(value: f64) -> Self {
        SimTime(Duration::from(value))
    }
}

// impl From<f32> for SimTime {
//     fn from(value: f32) -> Self {
//         SimTime(Duration::from(value))
//     }
// }

// TIMESPEC

#[cfg(feature = "cqueue")]
impl cqueue::AsNanosU128 for SimTime {
    fn as_nanos(&self) -> u128 {
        self.0.as_nanos()
    }
}

#[cfg(feature = "cqueue")]
impl cqueue::Timespec for SimTime {
    const ZERO: Self = SimTime(Duration::ZERO);
    const ONE: Self = SimTime(Duration::new(1, 0));
}

#[cfg(feature = "cqueue")]
impl Add<SimTime> for SimTime {
    type Output = SimTime;
    fn add(self, rhs: SimTime) -> SimTime {
        Self(self.0 + rhs.0)
    }
}
