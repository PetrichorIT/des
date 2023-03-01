//! Implements a lock, that only provides write access using the swap
//! method.
//!
//! This ensures that writes will not leak any &mut T accordingly W-W conflicts cannot appear.
//!
//! # Contract
//!
//! All swap operations must be coordianted from a single thread.
//! -> simulation core runs on only one thread.
//!
//! All read handles must be closed when a swap is performed.
//! -> Swaps happen inbetween events, while read handles are only handed out in
//! the event processing itself. Additionaly read handles are not leaked to the user
//! so we can ensure all are closed at event end.

use std::sync::atomic::Ordering::SeqCst;
use std::{cell::UnsafeCell, marker::PhantomData, ops::Deref, rc::Rc};

use super::AtomicUsize;

/// A lock that can only be accessed mutably by swapping the contents.
pub(crate) struct SwapLock<T> {
    inner: UnsafeCell<T>,
    read_count: AtomicUsize,
}

impl<T> SwapLock<T> {
    pub(crate) const fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            read_count: AtomicUsize::new(0),
        }
    }

    pub(crate) unsafe fn reset(&self, inner: T) {
        *self.inner.get() = inner;
        self.read_count.store(0, SeqCst);
    }

    pub(crate) fn swap(&self, other: &mut T) {
        // SAFTEY REASONS
        assert!(
            self.read_count.load(SeqCst) == 0,
            "SwapLock cannot swap, since {} read handles are still alive",
            self.read_count.load(SeqCst)
        );

        let inner = unsafe { &mut *self.inner.get() };
        std::mem::swap(inner, other);
    }

    pub(crate) fn read(&self) -> SwapLockReadGuard<'_, T> {
        SwapLockReadGuard::new(self)
    }
}

unsafe impl<T: Send> Send for SwapLock<T> {}
unsafe impl<T: Sync> Sync for SwapLock<T> {}

pub(crate) struct SwapLockReadGuard<'a, T> {
    lock: &'a SwapLock<T>,
    _phantom: PhantomData<Rc<T>>,
}

impl<'a, T> SwapLockReadGuard<'a, T> {
    fn new(lock: &'a SwapLock<T>) -> Self {
        let ptr: *const SwapLock<T> = lock;
        lock.read_count.fetch_add(1, SeqCst);
        Self {
            lock,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T> Deref for SwapLockReadGuard<'a, Option<T>> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { (*self.lock.inner.get()).as_ref().unwrap_unchecked() }
    }
}

impl<'a, T> Drop for SwapLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let ptr: *const SwapLock<T> = self.lock;
        self.lock.read_count.fetch_sub(1, SeqCst);
    }
}

unsafe impl<T: Sync> Sync for SwapLockReadGuard<'_, T> {}
