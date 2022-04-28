#![feature(linked_list_remove)]

#[doc(hidden)]
pub mod const_time;
#[doc(hidden)]
pub mod overflow_heap;

use std::ops::{Add, Div, Rem};

#[cfg(feature = "optimized")]
pub use const_time::*;

use num_traits::Zero;
#[cfg(not(feature = "optimized"))]
pub use overflow_heap::*;


use std::fmt::Debug;

pub trait TimeLike:
    Debug + Zero + Rem<Output = Self> + Copy + Add + Div<Output = Self> + PartialOrd
{
    fn as_usize(self) -> usize;
    fn min(self, other: Self) -> Self;
}
