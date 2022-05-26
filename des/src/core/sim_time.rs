use crate::core::runtime::sim_time;

use std::f64::EPSILON;
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

// Reexport

///
/// An extended version of [std::time::Duration] designed to interact
/// with a simulation time.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Duration(std::time::Duration);

impl Duration {
    /// A duration of zero time.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let duration = Duration::ZERO;
    /// assert!(duration.is_zero());
    /// assert_eq!(duration.as_nanos(), 0);
    /// ```
    pub const ZERO: Duration = Duration(std::time::Duration::ZERO);

    /// The maximum duration.
    ///
    /// May vary by platform as necessary. Must be able to contain the difference between
    /// two instances of [`Instant`](std::time::Instant) or two instances of [`SystemTime`](std::time::SystemTime).
    /// This constraint gives it a value of about 584,942,417,355 years in practice,
    /// which is currently used on all platforms.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::MAX, Duration::new(u64::MAX, 1_000_000_000 - 1));
    /// ```
    pub const MAX: Duration = Duration(std::time::Duration::MAX);

    /// Creates a new `Duration` from the specified number of whole seconds and
    /// additional nanoseconds.
    ///
    /// If the number of nanoseconds is greater than 1 billion (the number of
    /// nanoseconds in a second), then it will carry over into the seconds provided.
    ///
    /// # Panics
    ///
    /// This constructor will panic if the carry from the nanoseconds overflows
    /// the seconds counter.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let five_seconds = Duration::new(5, 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(secs: u64, nanos: u32) -> Self {
        Self(std::time::Duration::new(secs, nanos))
    }

    /// Creates a new `Duration` from the specified number of whole seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let duration = Duration::from_secs(5);
    ///
    /// assert_eq!(5, duration.as_secs());
    /// assert_eq!(0, duration.subsec_nanos());
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_secs(secs: u64) -> Self {
        Self(std::time::Duration::from_secs(secs))
    }

    /// Creates a new `Duration` from the specified number of milliseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let duration = Duration::from_millis(2569);
    ///
    /// assert_eq!(2, duration.as_secs());
    /// assert_eq!(569_000_000, duration.subsec_nanos());
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(std::time::Duration::from_millis(millis))
    }

    /// Creates a new `Duration` from the specified number of microseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let duration = Duration::from_micros(1_000_002);
    ///
    /// assert_eq!(1, duration.as_secs());
    /// assert_eq!(2000, duration.subsec_nanos());
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self(std::time::Duration::from_micros(micros))
    }

    /// Creates a new `Duration` from the specified number of nanoseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let duration = Duration::from_nanos(1_000_000_123);
    ///
    /// assert_eq!(1, duration.as_secs());
    /// assert_eq!(123, duration.subsec_nanos());
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(std::time::Duration::from_nanos(nanos))
    }

    /// Creates a new `Duration` from the specified number of seconds represented
    /// as `f32`.
    ///
    /// # Panics
    /// This constructor will panic if `secs` is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let res = Duration::from_secs_f32(0.0);
    /// assert_eq!(res, Duration::new(0, 0));
    /// let res = Duration::from_secs_f32(1e-20);
    /// assert_eq!(res, Duration::new(0, 0));
    /// let res = Duration::from_secs_f32(4.2e-7);
    /// assert_eq!(res, Duration::new(0, 419));
    /// let res = Duration::from_secs_f32(2.7);
    /// assert_eq!(res, Duration::new(2, 700_000_047));
    /// let res = Duration::from_secs_f32(3e10);
    /// assert_eq!(res, Duration::new(30_000_001_024, 0));
    /// // subnormal float
    /// let res = Duration::from_secs_f32(f32::from_bits(1));
    /// assert_eq!(res, Duration::new(0, 0));
    /// // conversion uses truncation, not rounding
    /// let res = Duration::from_secs_f32(0.999e-9);
    /// assert_eq!(res, Duration::new(0, 0));
    /// ```
    #[must_use]
    #[inline]
    pub fn from_secs_f32(secs: f32) -> Self {
        Self(std::time::Duration::from_secs_f32(secs))
    }

    /// Creates a new `Duration` from the specified number of seconds represented
    /// as `f64`.
    ///
    /// # Panics
    /// This constructor will panic if `secs` is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let res = Duration::from_secs_f64(0.0);
    /// assert_eq!(res, Duration::new(0, 0));
    /// let res = Duration::from_secs_f64(1e-20);
    /// assert_eq!(res, Duration::new(0, 0));
    /// let res = Duration::from_secs_f64(4.2e-7);
    /// assert_eq!(res, Duration::new(0, 420));
    /// let res = Duration::from_secs_f64(2.7);
    /// assert_eq!(res, Duration::new(2, 700_000_000));
    /// let res = Duration::from_secs_f64(3e10);
    /// assert_eq!(res, Duration::new(30_000_000_000, 0));
    /// // subnormal float
    /// let res = Duration::from_secs_f64(f64::from_bits(1));
    /// assert_eq!(res, Duration::new(0, 0));
    /// // conversion uses truncation, not rounding
    /// let res = Duration::from_secs_f64(0.999e-9);
    /// assert_eq!(res, Duration::new(0, 0));
    /// ```
    #[must_use]
    #[inline]
    pub fn from_secs_f64(secs: f64) -> Self {
        Self(std::time::Duration::from_secs_f64(secs))
    }

    /// Checked `Duration` addition. Computes `self + rhs`, returning [`None`]
    /// if overflow occurred.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(0, 0).checked_add(Duration::new(0, 1)), Some(Duration::new(0, 1)));
    /// assert_eq!(Duration::new(1, 0).checked_add(Duration::new(u64::MAX, 0)), None);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn checked_add(self, rhs: Duration) -> Option<Duration> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Saturating `Duration` addition. Computes `self + rhs`, returning [`Duration::MAX`]
    /// if overflow occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(0, 0).saturating_add(Duration::new(0, 1)), Duration::new(0, 1));
    /// assert_eq!(Duration::new(1, 0).saturating_add(Duration::new(u64::MAX, 0)), Duration::MAX);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn saturating_add(self, rhs: Duration) -> Duration {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Checked `Duration` subtraction. Computes `self - rhs`, returning [`None`]
    /// if the result would be negative or if overflow occurred.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(0, 1).checked_sub(Duration::new(0, 0)), Some(Duration::new(0, 1)));
    /// assert_eq!(Duration::new(0, 0).checked_sub(Duration::new(0, 1)), None);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    /// Saturating `Duration` subtraction. Computes `self - rhs`, returning [`Duration::ZERO`]
    /// if the result would be negative or if overflow occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(0, 1).saturating_sub(Duration::new(0, 0)), Duration::new(0, 1));
    /// assert_eq!(Duration::new(0, 0).saturating_sub(Duration::new(0, 1)), Duration::ZERO);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn saturating_sub(self, rhs: Duration) -> Duration {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Checked `Duration` multiplication. Computes `self * rhs`, returning
    /// [`None`] if overflow occurred.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(0, 500_000_001).checked_mul(2), Some(Duration::new(1, 2)));
    /// assert_eq!(Duration::new(u64::MAX - 1, 0).checked_mul(2), None);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn checked_mul(self, rhs: u32) -> Option<Duration> {
        self.0.checked_mul(rhs).map(Self)
    }

    /// Saturating `Duration` multiplication. Computes `self * other`, returning
    /// [`Duration::MAX`] if overflow occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(0, 500_000_001).saturating_mul(2), Duration::new(1, 2));
    /// assert_eq!(Duration::new(u64::MAX - 1, 0).saturating_mul(2), Duration::MAX);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn saturating_mul(self, rhs: u32) -> Duration {
        Self(self.0.saturating_mul(rhs))
    }

    /// Checked `Duration` division. Computes `self / other`, returning [`None`]
    /// if `other == 0`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use des::prelude::Duration;
    ///
    /// assert_eq!(Duration::new(2, 0).checked_div(2), Some(Duration::new(1, 0)));
    /// assert_eq!(Duration::new(1, 0).checked_div(2), Some(Duration::new(0, 500_000_000)));
    /// assert_eq!(Duration::new(2, 0).checked_div(0), None);
    /// ```
    #[inline]
    #[must_use = "This returns the result of the operation without modifying the originial"]
    pub const fn checked_div(self, rhs: u32) -> Option<Duration> {
        self.0.checked_div(rhs).map(Self)
    }
    /// Multiplies `Duration` by `f64`.
    ///
    /// # Panics
    /// This method will panic if result is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let dur = Duration::new(2, 700_000_000);
    /// assert_eq!(dur.mul_f64(3.14), Duration::new(8, 478_000_000));
    /// assert_eq!(dur.mul_f64(3.14e5), Duration::new(847_800, 0));
    /// ```
    #[must_use]
    #[inline]
    pub fn mul_f64(self, rhs: f64) -> Duration {
        Self(self.0.mul_f64(rhs))
    }

    /// Multiplies `Duration` by `f32`.
    ///
    /// # Panics
    /// This method will panic if result is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let dur = Duration::new(2, 700_000_000);
    /// assert_eq!(dur.mul_f32(3.14), Duration::new(8, 478_000_640));
    /// assert_eq!(dur.mul_f32(3.14e5), Duration::new(847800, 0));
    /// ```
    #[must_use]
    #[inline]
    pub fn mul_f32(self, rhs: f32) -> Duration {
        Self(self.0.mul_f32(rhs))
    }

    /// Divide `Duration` by `f32`.
    ///
    /// # Panics
    /// This method will panic if result is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let dur = Duration::new(2, 700_000_000);
    /// // note that due to rounding errors result is slightly
    /// // different from 0.859_872_611
    /// assert_eq!(dur.div_f32(3.14), Duration::new(0, 859_872_579));
    /// // note that truncation is used, not rounding
    /// assert_eq!(dur.div_f32(3.14e5), Duration::new(0, 8_598));
    /// ```
    #[must_use]
    #[inline]
    pub fn div_f32(self, rhs: f32) -> Duration {
        Self(self.0.div_f32(rhs))
    }

    /// Divide `Duration` by `f64`.
    ///
    /// # Panics
    /// This method will panic if result is negative, overflows `Duration` or not finite.
    ///
    /// # Examples
    /// ```
    /// use des::prelude::Duration;
    ///
    /// let dur = Duration::new(2, 700_000_000);
    /// assert_eq!(dur.div_f64(3.14), Duration::new(0, 859_872_611));
    /// // note that truncation is used, not rounding
    /// assert_eq!(dur.div_f64(3.14e5), Duration::new(0, 8_598));
    /// ```
    #[must_use]
    #[inline]
    pub fn div_f64(self, rhs: f64) -> Duration {
        Self(self.0.div_f64(rhs))
    }
}

// FMT

impl Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// OPS

impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0.add(rhs.0))
    }
}

impl AddAssign<Duration> for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        self.0.add_assign(rhs.0)
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0.sub(rhs.0))
    }
}

impl SubAssign<Duration> for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs
    }
}

impl Mul<u32> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0.mul(rhs))
    }
}

impl Mul<Duration> for u32 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Self::Output {
        Duration(rhs.0.mul(self))
    }
}

impl MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl Div<u32> for Duration {
    type Output = Duration;

    fn div(self, rhs: u32) -> Self::Output {
        Self(self.0.div(rhs))
    }
}

impl DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs
    }
}

// THIRD PARTY TYPES

impl Add<Duration> for SimTime {
    type Output = SimTime;

    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs)
            .expect("Overflow when adding Duration to SimTime")
    }
}

impl AddAssign<Duration> for SimTime {
    fn add_assign(&mut self, rhs: Duration) {
        self.0.add_assign(rhs)
    }
}

// # Missing # Add<Duration> for SystemTime

// # Missing # AddAssign<Duration> for SystemTime

impl Div<Duration> for Duration {
    type Output = f64;

    fn div(self, rhs: Duration) -> Self::Output {
        self.0.as_secs_f64() / rhs.0.as_secs_f64()
    }
}

impl Div<f64> for Duration {
    type Output = Duration;

    fn div(self, rhs: f64) -> Self::Output {
        Self::from(self.0.as_secs_f64() / rhs)
    }
}

// DEREF

impl Deref for Duration {
    type Target = std::time::Duration;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Duration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// FROM

impl From<f64> for Duration {
    fn from(secs: f64) -> Self {
        Duration::from_secs_f64(secs)
    }
}

impl From<Duration> for f64 {
    fn from(value: Duration) -> Self {
        value.as_secs_f64()
    }
}

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
