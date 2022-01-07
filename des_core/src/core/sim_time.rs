#[cfg(feature = "simtime_u128")]
pub use simtime_u128::*;
#[cfg(feature = "simtime_u128")]
mod simtime_u128 {
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
}

#[cfg(not(feature = "simtime_u128"))]
pub use simtime_f64::*;
#[cfg(not(feature = "simtime_u128"))]
mod simtime_f64 {
    use std::cmp::{Eq, Ord, Ordering};
    use std::fmt::{Display, Formatter, Write};
    use std::ops::*;

    use crate::core::sim_time;

    ///
    /// A type that represents a non-scaled discrete point of time
    /// in the simulation.
    ///
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SimTime(pub(crate) f64);

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
        /// use des_core::*;
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
        /// use des_core::SimTime;
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
        /// use des_core::SimTime;
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
                Self::Microseconds | Self::Nanoseconds | Self::Milliseconds | Self::Seconds => {
                    1000.0
                }
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
}
