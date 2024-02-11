cfg_not_multi_threaded! {
    use std::{
        cell::UnsafeCell,
        sync::atomic::{AtomicBool, Ordering},
        ops::{Deref, DerefMut},
    };

    #[derive(Debug)]
    pub(crate) struct Mutex<T> {
        value: UnsafeCell<T>,
        locked: AtomicBool,
    }

    impl<T> Mutex<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self {
                value: UnsafeCell::new(value),
                locked: AtomicBool::new(false),
            }
        }

        pub(crate) fn into_inner(self) -> T {
            self.value.into_inner()
        }

        pub(crate) fn is_locked(&self) -> bool {
            self.locked.load(Ordering::SeqCst)
        }


        pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
            MutexGuard::new(self)
        }
    }

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}

    pub(crate) struct MutexGuard<'a, T> {
        inner: &'a Mutex<T>,
    }

    impl<'a, T> MutexGuard<'a, T> {
        fn new(inner: &'a Mutex<T>) -> Self {
            let lock_failed = inner.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err();
            assert!(!lock_failed, "Could not lock mutex on single thread");
            Self { inner }
        }
    }

    impl<T> Deref for MutexGuard<'_, T>{
        type Target = T;
        fn deref(&self) -> &T {
            // SAFTEY lock gurantees exclusive access.
            unsafe { &*self.inner.value.get() }
        }
    }

    impl<T> DerefMut for MutexGuard<'_, T>{
        fn deref_mut(&mut self) -> &mut T {
            // SAFTEY lock gurantees exclusive access.
            unsafe { &mut *self.inner.value.get() }
        }
    }

    impl<T> Drop for MutexGuard<'_, T> {
        fn drop(&mut self) {
            self.inner.locked.store(false, Ordering::Release);
        }
    }
}

cfg_multi_threaded! {
    pub(crate) use ::spin::mutex::*;
}
