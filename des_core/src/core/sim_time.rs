use std::cmp::{Eq, Ord, Ordering};
use std::fmt::{Display, Formatter, Write};
use std::ops::*;

use crate::sim_time;

///
/// A type that represents a non-scaled discrete point of time
/// in the simulation.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimTime(f64);

impl SimTime {
    /// The start time of the simulation.
    pub const ZERO: SimTime = SimTime(0.0);

    /// The end time of the simulation.
    pub const INFINITY: SimTime = SimTime(f64::INFINITY);

    ///
    /// Creates a new instance from a raw f64.
    /// Note that this function only accepts valid timestamps (positiv).
    ///
    pub fn new(time: f64) -> Self {
        assert!(time >= 0.0);
        Self(time)
    }
    ///
    /// Creates a new instance holding the current simulation time.
    /// Note that this requires a global runtime core to be created beforhand,
    /// if not this function will panic.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use dse::*;
    ///
    /// let t = SimTime::now();
    /// ```
    ///
    pub fn now() -> Self {
        sim_time()
    }

    ///
    /// Indicates whether the simtime can ever be reached.
    ///
    pub fn is_valid(&self) -> bool {
        self.0.is_finite()
    }

    ///
    /// Returns the absolute part of the simtime.
    ///
    pub fn abs(self) -> f64 {
        self.0.abs()
    }

    ///
    /// Returns the value closer to the simulation start.
    ///
    pub fn min(&self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    ///
    /// Returns the value closer to the simulation end.
    ///
    pub fn max(&self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    ///
    /// Returns the integer (super-unit) part of the timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use dse::SimTime;
    ///
    /// let st_1 = SimTime::new(3.4);
    /// let st_2 = SimTime::new(3.0);
    ///
    /// assert_eq!(st_1.trunc(), 3.0);
    /// assert_eq!(st_2.trunc(), 3.0);
    /// ```
    pub fn trunc(self) -> SimTime {
        Self(self.0.trunc())
    }

    ///
    /// Returns the fraction (sub-unit) part of the timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use dse::SimTime;
    ///
    /// let st_1 = SimTime::new(3.4);
    /// let st_2 = SimTime::new(1.4);
    ///
    /// assert!((st_1.fract().abs() - 0.4).abs() < 1e-10);
    /// assert!((st_2.fract().abs() - 0.4).abs() < 1e-10);
    /// ```
    ///
    pub fn fract(self) -> SimTime {
        Self(self.0.fract())
    }
}

impl PartialEq<f64> for SimTime {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl Eq for SimTime {}

impl PartialOrd for SimTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let eq = self.0 == other.0;
        let gt = self.0 > other.0;

        Some(match (eq, gt) {
            (_, true) => Ordering::Greater,
            (true, false) => Ordering::Equal,
            (false, false) => Ordering::Less,
        })
    }
}

impl Ord for SimTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let eq = self.0 == other.0;
        let gt = self.0 > other.0;

        match (eq, gt) {
            (_, true) => Ordering::Greater,
            (true, false) => Ordering::Equal,
            (false, false) => Ordering::Less,
        }
    }
}

impl Display for SimTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<f64> for SimTime {
    fn from(fval: f64) -> Self {
        Self(fval)
    }
}

impl From<SimTime> for f64 {
    fn from(sim_time: SimTime) -> Self {
        sim_time.0
    }
}

// Self + Self

impl Add for SimTime {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for SimTime {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Mul for SimTime {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl MulAssign for SimTime {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 *= rhs.0
    }
}

impl Sub for SimTime {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for SimTime {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

impl Div for SimTime {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl DivAssign for SimTime {
    fn div_assign(&mut self, rhs: Self) {
        self.0 /= rhs.0
    }
}

// Self + f64

impl Add<f64> for SimTime {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<f64> for SimTime {
    fn add_assign(&mut self, rhs: f64) {
        self.0 += rhs
    }
}

impl Mul<f64> for SimTime {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl MulAssign<f64> for SimTime {
    fn mul_assign(&mut self, rhs: f64) {
        self.0 *= rhs
    }
}

impl Sub<f64> for SimTime {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<f64> for SimTime {
    fn sub_assign(&mut self, rhs: f64) {
        self.0 -= rhs
    }
}

impl Div<f64> for SimTime {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl DivAssign<f64> for SimTime {
    fn div_assign(&mut self, rhs: f64) {
        self.0 /= rhs
    }
}

///
/// A type to represent the minimum time-step of the simulation time,
/// thus the raw value of simtime.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimTimeUnit {
    Microseconds,
    Nanoseconds,
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
    Years,

    Undefined,
}

impl SimTimeUnit {
    ///
    /// Formats the given timestamp using the unit (unsimplified).
    ///
    pub fn fmt_full(sim_time: SimTime, unit: SimTimeUnit) -> String {
        let mut str = String::new();
        let (sim_time, unit) = SimTimeUnit::simplifiy(sim_time, unit);
        str.write_fmt(format_args!("{}{}", sim_time, unit))
            .expect("Failed core fmt");
        str
    }

    ///
    /// Scales the given type (interpreted as unit) to the most matching unit type.
    ///
    pub fn simplifiy(mut sim_time: SimTime, mut unit: SimTimeUnit) -> (SimTime, SimTimeUnit) {
        match unit {
            Self::Undefined => (sim_time, unit),
            _ => {
                // Shrink num, grow unit
                while sim_time >= unit.grow_factor().into() {
                    match unit.growed() {
                        Some(new_unit) => {
                            sim_time /= unit.grow_factor();
                            unit = new_unit;
                        }
                        None => break,
                    }
                }

                // Grow num, shrink unit
                while sim_time < 1.0.into() {
                    match unit.shrinked() {
                        Some(new_unit) => {
                            sim_time *= unit.shrink_factor();
                            unit = new_unit;
                        }
                        None => break,
                    }
                }

                (sim_time, unit)
            }
        }
    }

    ///
    /// Formats the given timestamp using the unit type (simplified).
    ///
    pub fn fmt_compact(sim_time: SimTime, unit: SimTimeUnit) -> String {
        let mut str = String::new();
        let (mut sim_time, mut unit) = SimTimeUnit::simplifiy(sim_time, unit);

        // Only present partial fractals
        // a.b where a 1..1000 b unknown

        // Ignore easy case
        if unit == Self::Microseconds || unit == Self::Undefined {
            str.write_fmt(format_args!("{}{}", sim_time, unit))
                .expect("Failed core fmt");
            str
        } else {
            loop {
                let mut intg = sim_time.0.trunc();
                let mut fract = sim_time.0.fract();

                if let Some(new_unit) = unit.shrinked() {
                    if (1.0 - fract) <= 0.001 {
                        intg += 1.0;
                        fract = 0.0;
                    }

                    if intg != 0.0 {
                        str.write_fmt(format_args!("{}{} ", intg, unit))
                            .expect("Failed core fmt");
                    }
                    sim_time = (fract * unit.shrink_factor()).into();
                    unit = new_unit;
                } else {
                    // This bound prevents floating point errors from
                    // poisioing output.
                    if sim_time > 0.01.into() {
                        str.write_fmt(format_args!("{}{} ", sim_time, unit))
                            .expect("Failed core fmt");
                    }
                    break;
                }

                if sim_time == 0.0 {
                    break;
                }
            }
            str
        }
    }

    ///
    /// Returns the next smaller unit, or None if not possible.
    ///
    pub fn shrinked(self) -> Option<Self> {
        match self {
            Self::Microseconds => None,
            Self::Nanoseconds => Some(Self::Microseconds),
            Self::Milliseconds => Some(Self::Nanoseconds),
            Self::Seconds => Some(Self::Milliseconds),
            Self::Minutes => Some(Self::Seconds),
            Self::Hours => Some(Self::Minutes),
            Self::Days => Some(Self::Hours),
            Self::Years => Some(Self::Days),
            _ => None,
        }
    }

    ///
    /// Returns the scaling factor to the next smaller unit type.
    ///
    /// Note that this returns 0 if no such unit is found.
    ///
    pub fn shrink_factor(&self) -> f64 {
        match self {
            Self::Microseconds | Self::Nanoseconds | Self::Milliseconds | Self::Seconds => 1000.0,
            Self::Minutes | Self::Hours => 60.0,
            Self::Days => 24.0,
            Self::Years => 356.0,
            Self::Undefined => 0.0,
        }
    }

    ///
    /// Returns the next bigger unit, or None if not possible.
    ///
    pub fn growed(self) -> Option<Self> {
        match self {
            Self::Microseconds => Some(Self::Nanoseconds),
            Self::Nanoseconds => Some(Self::Milliseconds),
            Self::Milliseconds => Some(Self::Seconds),
            Self::Seconds => Some(Self::Minutes),
            Self::Minutes => Some(Self::Hours),
            Self::Hours => Some(Self::Days),
            Self::Days => Some(Self::Years),
            Self::Years => None,
            _ => None,
        }
    }

    ///
    /// Returns the scaling factor to the next bigger unit type.
    ///
    /// Note that this will return 0 if no unit was found.
    ///
    pub fn grow_factor(&self) -> f64 {
        match self.growed() {
            Some(gt) => gt.shrink_factor(),
            None => 0.0,
        }
    }
}

impl Display for SimTimeUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Microseconds => write!(f, "µs"),
            Self::Nanoseconds => write!(f, "ns"),
            Self::Milliseconds => write!(f, "ms"),
            Self::Seconds => write!(f, "s"),
            Self::Minutes => write!(f, "min"),
            Self::Hours => write!(f, "h"),
            Self::Days => write!(f, "days"),
            Self::Years => write!(f, "years"),

            Self::Undefined => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{SimTime, SimTimeUnit};

    #[test]
    fn sim_time_unit() {
        let st = SimTime::new(9209.0);

        SimTimeUnit::fmt_compact(st, SimTimeUnit::Seconds);
    }
}
