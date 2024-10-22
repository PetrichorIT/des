/// A Duration type to represent a span of time.
pub use std::time::Duration;

use super::SimTime;
use std::ops::{Add, AddAssign};

impl Add<Duration> for SimTime {
    type Output = SimTime;

    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs)
            .expect("Overflow when adding Duration to SimTime")
    }
}

impl AddAssign<Duration> for SimTime {
    fn add_assign(&mut self, rhs: Duration) {
        self.0.add_assign(rhs);
    }
}

// f64

impl Add<f64> for SimTime {
    type Output = SimTime;

    fn add(self, rhs: f64) -> Self::Output {
        self.checked_add(Duration::from_secs_f64(rhs))
            .expect("Overflow when adding Duration to SimTime")
    }
}

impl AddAssign<f64> for SimTime {
    fn add_assign(&mut self, rhs: f64) {
        self.0.add_assign(Duration::from_secs_f64(rhs));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let mut time = SimTime::from(14.2);
        let duration = 3.4;

        assert_eq!(time + duration, SimTime::from(17.6));
        time += duration;
        assert_eq!(time, SimTime::from(17.6));

        let mut time = SimTime::from(14.2);
        let duration = Duration::from_secs_f64(3.4);

        assert_eq!(time + duration, SimTime::from(17.6));
        time += duration;
        assert_eq!(time, SimTime::from(17.6));
    }
}
