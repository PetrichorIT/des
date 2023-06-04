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

// SELECT

pub use std::future::Future;
pub use std::pin::Pin;
pub use std::task::Poll;

pub use futures::future::maybe_done;
pub use futures::future::poll_fn;

use crate::runtime;

#[doc(hidden)]
pub fn thread_rng_n(n: u32) -> u32 {
    runtime::random::<u32>() % n
}
