use crate::core::sim_time;
use std::cmp::{Eq, Ord, Ordering};
use std::fmt::{Display, Formatter, Write};
use std::ops::*;

///
/// A type that represents a non-scaled discrete point of time
/// in the simulation.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimTime {
    picos: u64,
    secs: u64,
}

const PICO_ONE_SEC: u64 = 1_000_000_000_000;
const PICO_MAX: u64 = PICO_ONE_SEC - 1;

impl SimTime {
    pub const ZERO: Self = Self { picos: 0, secs: 0 };

    pub const MIN: Self = Self { picos: 0, secs: 0 };
    pub const MAX: Self = Self {
        picos: PICO_MAX,
        secs: u64::MAX,
    };

    pub fn picos(&self) -> u64 {
        self.picos % 1000
    }

    pub fn nanos(&self) -> u64 {
        (self.picos / 1_000) % 1000
    }

    pub fn micros(&self) -> u64 {
        (self.picos / 1_000_000) % 1000
    }

    pub fn millis(&self) -> u64 {
        (self.picos / 1_000_000_000) % 1000
    }

    pub fn secs(&self) -> u64 {
        self.secs % 60
    }

    pub fn mins(&self) -> u64 {
        (self.secs / 60) % 60
    }

    pub fn hours(&self) -> u64 {
        (self.secs / (60 * 60)) % 24
    }

    pub fn days(&self) -> u64 {
        self.secs / (60 * 60 * 24)
    }

    pub fn new(picos: u64, secs: u64) -> Self {
        assert!(picos <= PICO_MAX);
        Self { picos, secs }
    }

    pub fn now() -> Self {
        sim_time()
    }
}

impl PartialOrd for SimTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.secs.cmp(&other.secs) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.picos.cmp(&other.picos),
        }
    }
}

impl Display for SimTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}s ({}ps)", self.secs, self.picos)
    }
}

impl From<f64> for SimTime {
    fn from(val: f64) -> Self {
        let secs = val.trunc() as u64;
        let picos = val.fract() * PICO_ONE_SEC as f64;
        let picos = picos as u64;

        Self { picos, secs }
    }
}

impl From<SimTime> for f64 {
    fn from(simtime: SimTime) -> Self {
        let mut result = simtime.picos as f64;
        result /= PICO_ONE_SEC as f64;
        result + simtime.secs as f64
    }
}

impl From<&'_ SimTime> for f64 {
    fn from(simtime: &'_ SimTime) -> Self {
        let mut result = simtime.picos as f64;
        result /= PICO_ONE_SEC as f64;
        result + simtime.secs as f64
    }
}

impl From<&'_ mut SimTime> for f64 {
    fn from(simtime: &'_ mut SimTime) -> Self {
        let mut result = simtime.picos as f64;
        result /= PICO_ONE_SEC as f64;
        result + simtime.secs as f64
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

impl AddAssign for SimTime {
    fn add_assign(&mut self, rhs: Self) {
        self.picos += rhs.picos;
        self.secs += rhs.secs;

        if self.picos > PICO_MAX {
            self.secs += 1;
            self.picos -= PICO_MAX;
        }
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
        if self.picos < rhs.picos {
            self.secs -= 1;
            self.picos += PICO_MAX;
        }

        self.picos -= rhs.picos;
        self.secs -= rhs.secs;
    }
}

// division with Self returns f64
// thus only Div no DivAssign

impl Div for SimTime {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        f64::from(self) / f64::from(rhs)
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
        let Self { picos, secs } = Self::from(f);
        self.picos = picos;
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

///
/// A type to represent the minimum time-step of the simulation time,
/// thus the raw value of simtime.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimTimeUnit {
    Picoseconds,
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
        Self::fmt_compact(sim_time, unit)
    }

    ///
    /// Formats the given timestamp using the unit type (simplified).
    ///
    pub fn fmt_compact(sim_time: SimTime, unit: SimTimeUnit) -> String {
        assert!(unit == Self::Seconds);

        let mut str = String::new();
        for (value, unit) in [
            (sim_time.days(), Self::Days),
            (sim_time.hours(), Self::Hours),
            (sim_time.mins(), Self::Minutes),
            (sim_time.secs(), Self::Seconds),
            (sim_time.millis(), Self::Milliseconds),
            (sim_time.micros(), Self::Microseconds),
            (sim_time.nanos(), Self::Nanoseconds),
            (sim_time.picos(), Self::Picoseconds),
        ] {
            if value != 0 {
                str.write_fmt(format_args!("{}{}", value, unit))
                    .expect("Failed write")
            }
        }

        str
    }
}

impl Display for SimTimeUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Picoseconds => write!(f, "ps"),
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
