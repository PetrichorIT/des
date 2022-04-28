use num_traits::{One, Zero};

use crate::core::sim_time;
use std::cmp::{Eq, Ord, Ordering};
use std::fmt::{Display, Formatter};
use std::ops::*;

///
/// A type that represents a non-scaled discrete point of time
/// in the simulation.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimTime {
    femtos: u64,
    secs: u64,
}

const FEMTO_ONE_SEC: u64 = 1_000_000_000_000_000;
const FEMTO_MAX: u64 = FEMTO_ONE_SEC - 1;

impl SimTime {
    ///
    /// The simtime instance made of of literal zero values.
    ///
    /// Zero-initalized memory transmuted as a simtime should
    /// result in this instance.
    ///
    pub const ZERO: Self = Self { femtos: 0, secs: 0 };

    ///
    /// The smalles valid simtime instance, the default start point
    /// of a simulation.
    ///
    pub const MIN: Self = Self { femtos: 0, secs: 0 };

    ///
    /// The biggest valid simtime instance.
    ///
    pub const MAX: Self = Self {
        femtos: FEMTO_MAX,
        secs: u64::MAX,
    };

    ///
    /// Creates a new instance from raw parts.
    ///
    pub fn new(femtos: u64, secs: u64) -> Self {
        assert!(
            femtos <= FEMTO_MAX,
            "The value 'femtos' must be below {}.",
            FEMTO_MAX
        );
        Self { femtos, secs }
    }

    ///
    /// Creates a new instance holding the current simulation time.
    /// Note that this requires a global runtime core to be created beforhand,
    /// if not this function will panic.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use des::prelude::*;
    ///
    /// let t = SimTime::now();
    /// ```
    ///
    pub fn now() -> Self {
        sim_time()
    }

    ///
    /// Indicates whether the simtime is a valid instance that,
    /// can ever be reached.
    ///
    pub fn is_valid(&self) -> bool {
        self.femtos <= FEMTO_MAX
    }

    ///
    /// Returns the femtoseconds of the current instance.
    ///
    pub fn femto(&self) -> u64 {
        self.femtos % 1000
    }

    ///
    /// Returns the picoseconds of the current instance.
    ///
    pub fn picos(&self) -> u64 {
        (self.femtos / 1_000) % 1000
    }

    ///
    /// Returns the nanoseconds of the current instance.
    ///
    pub fn nanos(&self) -> u64 {
        (self.femtos / 1_000_000) % 1000
    }

    ///
    /// Returns the microseconds of the current instance.
    ///
    pub fn micros(&self) -> u64 {
        (self.femtos / 1_000_000_000) % 1000
    }

    ///
    /// Returns the miliseconds of the current instance.
    ///
    pub fn millis(&self) -> u64 {
        (self.femtos / 1_000_000_000_000) % 1000
    }

    ///
    /// Returns the seconds of the current instance.
    ///
    pub fn secs(&self) -> u64 {
        self.secs % 60
    }

    ///
    /// Returns the minutes of the current instance.
    ///
    pub fn mins(&self) -> u64 {
        (self.secs / 60) % 60
    }

    ///
    /// Returns the hours of the current instance.
    ///
    pub fn hours(&self) -> u64 {
        (self.secs / (60 * 60)) % 24
    }

    ///
    /// Returns the days of the current instance.
    ///
    pub fn days(&self) -> u64 {
        self.secs / (60 * 60 * 24)
    }
}

impl PartialOrd for SimTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    // To prevent the usage of cmp, which is lesse effiecent event
    // in release mode.
    #[allow(clippy::comparison_chain)]
    fn lt(&self, other: &Self) -> bool {
        if self.secs < other.secs {
            true
        } else if self.secs == other.secs {
            self.femtos < other.femtos
        } else {
            false
        }
    }

    // To prevent the usage of cmp, which is lesse effiecent event
    // in release mode.
    #[allow(clippy::comparison_chain)]
    fn le(&self, other: &Self) -> bool {
        if self.secs < other.secs {
            true
        } else if self.secs == other.secs {
            self.femtos <= other.femtos
        } else {
            false
        }
    }

    // To prevent the usage of cmp, which is lesse effiecent event
    // in release mode.
    #[allow(clippy::comparison_chain)]
    fn gt(&self, other: &Self) -> bool {
        if self.secs > other.secs {
            true
        } else if self.secs == other.secs {
            self.femtos > other.femtos
        } else {
            false
        }
    }

    // To prevent the usage of cmp, which is lesse effiecent event
    // in release mode.
    #[allow(clippy::comparison_chain)]
    fn ge(&self, other: &Self) -> bool {
        if self.secs > other.secs {
            true
        } else if self.secs == other.secs {
            self.femtos >= other.femtos
        } else {
            false
        }
    }
}

impl Ord for SimTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.secs.cmp(&other.secs) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.femtos.cmp(&other.femtos),
        }
    }
}

impl PartialEq<f64> for SimTime {
    fn eq(&self, other: &f64) -> bool {
        *self == Self::from(*other)
    }
}

impl Display for SimTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // ignore femtos they are just for rounding
        for (value, unit) in [
            (self.days(), "days"),
            (self.hours(), "h"),
            (self.mins(), "min"),
            (self.secs(), "s"),
            (self.millis(), "ms"),
            (self.micros(), "µs"),
            (self.nanos(), "ns"),
            (self.picos(), "ps"),
        ] {
            if value != 0 {
                write!(f, "{}{} ", value, unit)?
            }
        }

        Ok(())
    }
}

impl From<f64> for SimTime {
    fn from(val: f64) -> Self {
        let secs = val.trunc() as u64;
        let femtos = val.fract() * FEMTO_ONE_SEC as f64;
        let femtos = femtos as u64;

        Self { femtos, secs }
    }
}

impl From<SimTime> for f64 {
    fn from(simtime: SimTime) -> Self {
        let mut result = simtime.femtos as f64;
        result /= FEMTO_ONE_SEC as f64;
        result += simtime.secs as f64;

        result
    }
}

impl From<&'_ SimTime> for f64 {
    fn from(simtime: &'_ SimTime) -> Self {
        let mut result = simtime.femtos as f64;
        result /= FEMTO_ONE_SEC as f64;
        result += simtime.secs as f64;

        result
    }
}

impl From<&'_ mut SimTime> for f64 {
    fn from(simtime: &'_ mut SimTime) -> Self {
        let mut result = simtime.femtos as f64;
        result /= FEMTO_ONE_SEC as f64;
        result += simtime.secs as f64;

        result
    }
}

// primitiv time op

impl Add for SimTime {
    type Output = SimTime;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add<f64> for SimTime {
    type Output = SimTime;

    fn add(self, rhs: f64) -> Self::Output {
        Self::add(self, Self::from(rhs))
    }
}

impl AddAssign for SimTime {
    fn add_assign(&mut self, rhs: Self) {
        self.femtos += rhs.femtos;
        self.secs += rhs.secs;

        if self.femtos > FEMTO_MAX {
            self.secs += 1;
            self.femtos -= FEMTO_MAX;
        }
    }
}

impl AddAssign<f64> for SimTime {
    fn add_assign(&mut self, rhs: f64) {
        Self::add_assign(self, Self::from(rhs))
    }
}

impl Sub for SimTime {
    type Output = SimTime;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl SubAssign for SimTime {
    fn sub_assign(&mut self, rhs: Self) {
        if self.femtos < rhs.femtos {
            self.secs -= 1;
            self.femtos += FEMTO_MAX;
        }

        self.femtos -= rhs.femtos;
        self.secs -= rhs.secs;
    }
}

// division with Self returns f64
// thus only Div no DivAssign

impl Div for SimTime {
    type Output = SimTime;

    fn div(self, rhs: Self) -> Self::Output {
        SimTime::from(f64::from(self) / f64::from(rhs))
    }
}

// division with f64 assumes simtime is duration
// thus duration scaling
// Mul MulAssign Div DivAssign

impl Mul<f64> for SimTime {
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self::Output {
        self.mul_assign(rhs);
        self
    }
}

impl MulAssign<f64> for SimTime {
    fn mul_assign(&mut self, rhs: f64) {
        let rawref = &*self;
        let f = f64::from(rawref) * rhs;
        let Self {
            femtos: picos,
            secs,
        } = Self::from(f);
        self.femtos = picos;
        self.secs = secs;
    }
}

impl Div<f64> for SimTime {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        self.mul(1.0_f64 / rhs)
    }
}

impl DivAssign<f64> for SimTime {
    fn div_assign(&mut self, rhs: f64) {
        self.mul_assign(1.0_f64 / rhs);
    }
}

#[cfg(feature = "cqueue")]
impl cqueue::TimeLike for SimTime {
    fn as_usize(self) -> usize {
        self.secs as usize
    }

    fn min(self, other: Self) -> Self {
        if self < other {
            self
        } else {
            other
        }
    }
}

impl Rem for SimTime {
    type Output = SimTime;

    fn rem(self, rhs: Self) -> Self::Output {
        // self % rhs

        let secs = self.secs % rhs.secs;
        let femtos = self.femtos % rhs.femtos;

        Self { femtos, secs }
    }
}

impl Zero for SimTime {
    fn zero() -> Self {
        SimTime { secs: 0, femtos: 0 }
    }

    fn is_zero(&self) -> bool {
        self.secs == 0 && self.femtos == 0
    }
}

impl One for SimTime {
    fn one() -> Self {
        SimTime { secs: 1, femtos: 0 }
    }
}

impl Mul for SimTime {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let secs = self.secs * rhs.secs;
        let femtos = self.femtos * rhs.femtos;

        let rep = femtos / FEMTO_MAX;
        let rem = femtos % FEMTO_MAX;

        Self {
            secs: secs + rep,
            femtos: rem,
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn add() {
        use super::*;

        let s1 = SimTime::new(123, 23123);
        let s2 = SimTime::new(FEMTO_MAX - 2, 12);

        let sum = s1 + s2;

        assert_eq!(sum.secs, 23123 + 12 + 1);
        assert_eq!(sum.femtos, 121);
    }

    #[test]
    fn sub() {
        use super::*;

        let s1 = SimTime::new(123, 23123);
        let s2 = SimTime::new(FEMTO_MAX - 2, 12);

        let sum = s1 - s2;

        assert_eq!(sum.secs, 23123 - 12 - 1);
        assert_eq!(sum.femtos, 125);
    }

    #[test]
    fn div() {
        use super::*;

        let s1 = SimTime::new(2000, 1000);
        let s2 = SimTime::new(0, 2);

        let div = s1 / s2;

        assert_eq!(div.secs, 500);
        assert_eq!(div.femtos, 100);
    }
}
