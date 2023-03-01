//! Implements are classic non-poisonable Read-Write-Lock with an
//! API inspired by `spin::RwLock`.
//!
//! In single-threaded contexts this will be implemented using a `RefCell`
//! like structure (with appropiate API).
//! In multi-threaded contexts, `spin::RwLock` is used.

cfg_not_multi_threaded! {
    use std::{
        cell::{Cell, UnsafeCell},
        marker::PhantomData,
        ops::{Deref, DerefMut},
        ptr::NonNull,
    };

    type BorrowFlag = isize;
    const UNUSED: BorrowFlag = 0;

    #[inline]
    fn is_writing(x: BorrowFlag) -> bool {
        x < UNUSED
    }

    #[inline]
    fn is_reading(x: BorrowFlag) -> bool {
        x > UNUSED
    }

    pub(crate) struct RwLock<T> {
        flag: Cell<BorrowFlag>,
        value: UnsafeCell<T>,
    }

    impl<T> RwLock<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self {
                flag: Cell::new(0),
                value: UnsafeCell::new(value),
            }
        }

        pub(crate) fn into_inner(self) -> T {
            self.value.into_inner()
        }

        pub(crate) fn read(&self) -> RwLockReadGuard<'_, T> {
            self.try_read()
                .expect("Failed to get read lock on single thread")
        }

        pub(crate) fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
            let permit = ReadBorrow::new(&self.flag)?;
            let value = unsafe { NonNull::new_unchecked(self.value.get()) };
            Some(RwLockReadGuard { permit, value })
        }

        pub(crate) fn write(&self) -> RwLockWriteGuard<'_, T> {
            self.try_write()
                .expect("Failed to get read lock on single thread")
        }

        pub(crate) fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
            let permit = WriteBorrow::new(&self.flag)?;
            let value = unsafe { NonNull::new_unchecked(self.value.get()) };
            Some(RwLockWriteGuard {
                permit,
                value,
                marker: PhantomData,
            })
        }
    }

    unsafe impl<T: Send> Send for RwLock<T> {}
    unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

    pub(crate) struct RwLockReadGuard<'a, T> {
        permit: ReadBorrow<'a>,
        value: NonNull<T>,
    }

    impl<T> Deref for RwLockReadGuard<'_, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            unsafe { self.value.as_ref() }
        }
    }

    impl<'a, T> RwLockReadGuard<'a, T> {
        pub(crate) fn leak(self) -> &'a T {
            std::mem::forget(self.permit);
            unsafe { self.value.as_ref() }
        }
    }

    unsafe impl<T: Send> Send for RwLockReadGuard<'_, T> {}
    unsafe impl<T: Send + Sync> Sync for RwLockReadGuard<'_, T> {}

    struct ReadBorrow<'b> {
        cell: &'b Cell<BorrowFlag>,
    }

    impl<'b> ReadBorrow<'b> {
        fn new(cell: &'b Cell<BorrowFlag>) -> Option<Self> {
            let b = cell.get().wrapping_add(1);
            if is_reading(b) {
                cell.set(b);
                Some(Self { cell })
            } else {
               None
            }
        }
    }

    impl Drop for ReadBorrow<'_> {
        fn drop(&mut self) {
            let b = self.cell.get();
            self.cell.set(b - 1);
        }
    }

    impl Clone for ReadBorrow<'_> {
        fn clone(&self) -> Self {
            let b = self.cell.get();
            self.cell.set(b + 1);
            Self { cell: self.cell }
        }
    }

    pub(crate) struct RwLockWriteGuard<'a, T> {
        permit: WriteBorrow<'a>,
        value: NonNull<T>,

        marker: PhantomData<&'a mut T>,
    }

    impl<T> Deref for RwLockWriteGuard<'_, T> {
        type Target = T;
        fn deref(&self) -> &T {
            // SAFETY: the value is accessible as long as we hold our borrow.
            unsafe { self.value.as_ref() }
        }
    }

    impl<T> DerefMut for RwLockWriteGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut T {
            // SAFETY: the value is accessible as long as we hold our borrow.
            unsafe { self.value.as_mut() }
        }
    }

    unsafe impl<T: Send + Sync> Send for RwLockWriteGuard<'_, T> {}
    unsafe impl<T: Send + Sync> Sync for RwLockWriteGuard<'_, T> {}

    struct WriteBorrow<'b> {
        cell: &'b Cell<BorrowFlag>,
    }

    impl<'b> WriteBorrow<'b> {
        fn new(cell: &'b Cell<BorrowFlag>) -> Option<Self> {
            // NOTE: Unlike BorrowRefMut::clone, new is called to create the initial
            // mutable reference, and so there must currently be no existing
            // references. Thus, while clone increments the mutable refcount, here
            // we explicitly only allow going from UNUSED to UNUSED - 1.
            match cell.get() {
                UNUSED => {
                    cell.set(UNUSED - 1);
                    Some(Self { cell })
                }
                _ => None,
            }
        }
    }

    impl Drop for WriteBorrow<'_> {
        fn drop(&mut self) {
            let b = self.cell.get();
            self.cell.set(b + 1);
        }
    }
}

cfg_multi_threaded! {
    pub(crate) use spin::rwlock::*;
}
