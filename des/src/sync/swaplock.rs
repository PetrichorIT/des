use std::sync::atomic::Ordering::SeqCst;
use std::{cell::UnsafeCell, marker::PhantomData, ops::Deref, rc::Rc};

use super::AtomicUsize;

/// A lock that can only be accessed mutably by swapping the contents
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

    pub(crate) fn swap(&self, other: &mut T) {
        // SAFTEY REASONS
        assert_eq!(
            self.read_count.load(SeqCst),
            0,
            "Cannot swap context with active readers"
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
        unsafe { (&*self.lock.inner.get()).as_ref().unwrap_unchecked() }
    }
}

impl<'a, T> Drop for SwapLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.read_count.fetch_sub(1, SeqCst);
    }
}

unsafe impl<T: Sync> Sync for SwapLockReadGuard<'_, T> {}
