#![feature(linked_list_remove)]

#[doc(hidden)]
pub mod const_time;
#[doc(hidden)]
pub mod overflow_heap;

use std::ops::{Add, Div, Rem};

#[cfg(feature = "linked_list")]
pub use const_time::*;

use num_traits::Zero;
#[cfg(not(feature = "linked_list"))]
pub use overflow_heap::*;

pub trait TimeLike:
    Zero + Rem<Output = Self> + Copy + Add + Div<Output = Self> + PartialOrd
{
    fn as_usize(self) -> usize;
    fn min(self, other: Self) -> Self;
}
