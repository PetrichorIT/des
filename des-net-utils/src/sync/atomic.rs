//! A wrapper around atomics.
//!
//! While still cheap, atomic operations are more expensive than primitive integer
//! operations. This difference matters, since some lock use atomics to share state.
//! In single-threaded contexts this is not nessecary. Accordingly there are
//! wrappers around a not-really atomic implementation of Atomics for single-thread use.
//!
//! Not that this implemation results in 2-3 % performance increase since
//! atomic based locks are used in every event.

use std::{cell::UnsafeCell, sync::atomic::Ordering};

pub struct AtomicUsize {
    v: UnsafeCell<usize>,
}

impl AtomicUsize {
    #[must_use]
    pub const fn new(v: usize) -> Self {
        Self {
            v: UnsafeCell::new(v),
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_v(&self) -> &mut usize {
        unsafe { &mut *self.v.get() }
    }

    pub fn get_mut(&mut self) -> &mut usize {
        self.get_v()
    }

    pub fn into_inner(self) -> usize {
        *self.get_v()
    }

    pub fn load(&self, _order: Ordering) -> usize {
        *self.get_v()
    }

    pub fn store(&self, val: usize, _order: Ordering) {
        *self.get_v() = val;
    }

    pub fn swap(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v = val;
        ret
    }

    /// # Errors
    /// Returns an error if the current value is not equal to the expected value.
    pub fn compare_exchange(
        &self,
        cur: usize,
        new: usize,
        suc: Ordering,
        fail: Ordering,
    ) -> Result<usize, usize> {
        let v = self.get_v();
        if *v != cur {
            return Err(*v);
        }
        Ok(self.swap(new, suc))
    }

    pub fn fetch_add(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v = v.wrapping_add(val);
        ret
    }

    pub fn fetch_sub(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v = v.wrapping_sub(val);
        ret
    }

    pub fn fetch_and(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v &= val;
        ret
    }

    pub fn fetch_nand(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v = !(*v & val);
        ret
    }

    pub fn fetch_or(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v |= val;
        ret
    }

    pub fn fetch_xor(&self, val: usize, _order: Ordering) -> usize {
        let v = self.get_v();
        let ret = *v;
        *v ^= val;
        ret
    }
}

unsafe impl Send for AtomicUsize {}
unsafe impl Sync for AtomicUsize {}
