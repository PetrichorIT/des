#[macro_export]
macro_rules! assert_eq_time {
    ($left:expr, $right:expr $(,)?) => {
        assert!($left.eq_approx($crate::time::SimTime::from_duration(
            std::time::Duration::from_secs_f64($right)
        ), std::time::Duration::from_nanos(100)))
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        assert!($left.eq_approx($crate::time::SimTime::from_duration(
            std::time::Duration::from_secs_f64($right)
        ), std::time::Duration::from_nanos(100)), $($arg)+)
    };
}
