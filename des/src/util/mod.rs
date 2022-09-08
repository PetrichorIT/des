//!
//! Common utilities.
//!

mod mm;
pub(crate) use mm::*;

cfg_net! {
    mod any;
    pub(crate) use any::*;
}

mod ptr;
pub use ptr::*;
