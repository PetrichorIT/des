use crate::core::runtime::sim_time;

use std::f64::EPSILON;
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, Sub, SubAssign};

// Reexport

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Duration(std::time::Duration);

impl Duration {
    pub const ZERO: Duration = Duration(std::time::Duration::ZERO);
    pub const MAX: Duration = Duration(std::time::Duration::MAX);

    pub const fn new(secs: u64, nanos: u32) -> Self {
        Self(std::time::Duration::new(secs, nanos))
    }

    pub const fn from_secs(secs: u64) -> Self {
        Self(std::time::Duration::from_secs(secs))
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self(std::time::Duration::from_millis(millis))
    }

    pub const fn from_micros(micros: u64) -> Self {
        Self(std::time::Duration::from_micros(micros))
    }

    pub const fn from_nanos(nanos: u64) -> Self {
        Self(std::time::Duration::from_nanos(nanos))
    }

    pub const fn checked_add(self, rhs: Duration) -> Option<Duration> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub const fn saturating_add(self, rhs: Duration) -> Duration {
        Self(self.0.saturating_add(rhs.0))
    }

    pub const fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub const fn saturating_sub(self, rhs: Duration) -> Duration {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub const fn checked_mul(self, rhs: u32) -> Option<Duration> {
        self.0.checked_mul(rhs).map(Self)
    }

    pub const fn saturating_mul(self, rhs: u32) -> Duration {
        Self(self.0.saturating_mul(rhs))
    }

    pub const fn checked_div(self, rhs: u32) -> Option<Duration> {
        self.0.checked_div(rhs).map(Self)
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
    fn from(value: f64) -> Self {
        let sub_sec = value.fract();
        let secs = value.trunc();

        let nanos = sub_sec * 1_000_000_000.0;

        Duration::new(secs as u64, nanos as u32)
    }
}

// impl From<f32> for Duration {
//     fn from(value: f32) -> Self {
//         let sub_sec = value.fract();
//         let secs = value.trunc();

//         let nanos = sub_sec * 1_000_000_000.0;

//         Duration::new(secs as u64, nanos as u32)
//     }
// }

impl From<Duration> for f64 {
    fn from(value: Duration) -> Self {
        value.as_secs_f64()
    }
}

// impl From<Duration> for f32 {
//     fn from(value: Duration) -> Self {
//         value.as_secs_f32()
//     }
// }

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimTime(Duration);

impl SimTime {
    #[must_use]
    pub fn now() -> Self {
        sim_time()
    }

    #[must_use]
    pub fn duration_since(&self, earlier: SimTime) -> Duration {
        self.checked_duration_since(earlier).unwrap()
    }

    #[must_use]
    pub fn checked_duration_since(&self, earlier: SimTime) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    #[must_use]
    pub fn saturating_duration_since(&self, earlier: SimTime) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Self::now() - *self
    }

    #[must_use]
    pub fn checked_add(&self, duration: Duration) -> Option<SimTime> {
        self.0.checked_add(duration).map(SimTime)
    }

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
