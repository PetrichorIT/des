// # Note
// This feature requirement comes from the fact that
// those buffers are only used in the 'net' submodule.
// Additionally feature 'net-static' requires
// 'net' so the not(feature 'static') implicitly also requires it
#[cfg(feature = "net")]
mod buffer;
#[cfg(feature = "net")]
pub use buffer::*;

mod cell;
pub use cell::*;

mod macros;
pub use macros::*;

pub mod spmc;

mod mm;
pub use mm::*;
