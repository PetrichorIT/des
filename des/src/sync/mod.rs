#![allow(unused)]
//! Synchronisation primitives for internal use
//!

mod swaplock;
pub(crate) use self::swaplock::*;
