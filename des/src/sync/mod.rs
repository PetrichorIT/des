#![allow(unused)]
//! Synchronisation primitives for internal use
//!
//! The primitives defined here provide either specialised capabilities
//! or a more efficient implementation if the feature multi-threaded is not
//! set. As an example, the RwLock that is implemented here uses a RefCell inspired
//! implementation in single-threaded contexts, which is significantly more performant.

mod swaplock;
pub(crate) use self::swaplock::*;

mod atomic;
pub(crate) use self::atomic::*;

mod rwlock;
pub(crate) use self::rwlock::*;
