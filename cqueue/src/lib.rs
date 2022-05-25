#![feature(linked_list_remove)]

#[doc(hidden)]
pub mod const_time;
#[doc(hidden)]
pub mod overflow_heap;

use std::ops::Add;

#[cfg(feature = "optimized")]
pub use const_time::*;
#[cfg(not(feature = "optimized"))]
pub use overflow_heap::*;

pub trait AsNanosU128 {
    fn as_nanos(&self) -> u128;
}

pub trait Timespec: AsNanosU128 + Copy + PartialOrd + Add<Output = Self> {
    const ZERO: Self;
    const ONE: Self;
}

// IMPL: Duration

impl AsNanosU128 for std::time::Duration {
    fn as_nanos(&self) -> u128 {
        self.as_nanos()
    }
}

impl Timespec for std::time::Duration {
    const ZERO: Self = std::time::Duration::ZERO;
    const ONE: Self = std::time::Duration::new(1, 0);
}

// IMPL: F64

impl AsNanosU128 for f64 {
    fn as_nanos(&self) -> u128 {
        let nanos = self * 1_000_000_000.0;
        nanos as u128
    }
}

impl Timespec for f64 {
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
}
