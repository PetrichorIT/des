/// A Duration type to represent a span of time.
pub use std::time::Duration;

cfg_not_async! {
    use std::ops::{Add, AddAssign};
    use super::SimTime;

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

}
