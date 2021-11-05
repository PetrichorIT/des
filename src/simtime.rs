use std::cmp::{Eq, Ord, Ordering};
use std::fmt::Display;
use std::ops::*;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SimTime(f64);

impl SimTime {
    pub const ZERO: SimTime = SimTime(0.0);
    pub const INFINITY: SimTime = SimTime(f64::INFINITY);

    pub fn new(time: f64) -> Self {
        Self(time)
    }

    pub fn is_valid(&self) -> bool {
        self.0.is_finite()
    }

    pub fn min(&self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn max(&self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

impl Eq for SimTime {}

impl Ord for SimTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let eq = self == other;
        let gt = self > other;

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
