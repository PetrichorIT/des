//!
//!  Memory management
//!

use std::ops::{Deref, DerefMut};

pub(crate) struct SyncWrap<T> {
    inner: T,
}

impl<T> SyncWrap<T> {
    pub(crate) const fn new(item: T) -> Self {
        Self { inner: item }
    }
}

impl<T> Deref for SyncWrap<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for SyncWrap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// SAFTY:
// This wrapper should only be used to make statics thread safe,
// since by design event simulation is single-threded (in the same context).
unsafe impl<T> Sync for SyncWrap<T> {}
