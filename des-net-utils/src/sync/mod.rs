#![allow(unused)]
//! Synchronisation primitives for internal use
//!
//! The primitives defined here provide either specialised capabilities
//! or a more efficient implementation if the feature multi-threaded is not
//! set. As an example, the `RwLock` that is implemented here uses a `RefCell` inspired
//! implementation in single-threaded contexts, which is significantly more performant.

mod atomic;
mod mutex;
mod rwlock;
mod swaplock;

pub use self::atomic::*;
pub use self::mutex::*;
pub use self::rwlock::*;
pub use self::swaplock::*;
