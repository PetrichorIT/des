#![feature(box_into_inner)]

pub(crate) mod core;
pub(crate) mod metrics;
pub(crate) mod misc;
pub(crate) mod net;

pub use crate::core::event::*;
pub use crate::core::interning::*;
pub use crate::core::runtime::*;
pub use crate::core::sim_time::*;
pub use crate::metrics::*;

#[cfg(feature = "net")]
pub use crate::net::*;

#[allow(unused_imports)]
pub(crate) use crate::misc::*;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
