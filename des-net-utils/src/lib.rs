#![deny(unused_must_use)]
#![warn(clippy::pedantic)]
#![warn(unreachable_pub)]
#![allow(
    clippy::needless_doctest_main,
    clippy::module_name_repetitions,
    clippy::arc_with_non_send_sync
)]

pub mod ndl;
pub mod props;
pub mod sync;
