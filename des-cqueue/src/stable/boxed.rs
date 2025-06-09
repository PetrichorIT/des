use std::{
    alloc::Layout,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

use super::alloc::CQueueLLAllocator;

pub struct LocalBox<E> {
    ptr: *mut E,
    alloc: CQueueLLAllocator,
}

impl<E> LocalBox<E> {
    pub fn new_in(value: E, mut alloc: CQueueLLAllocator) -> LocalBox<E> {
        let bytes = alloc.allocate(Layout::new::<E>()).unwrap();
        let ptr = bytes.cast::<E>();
        unsafe {
            ptr::write_volatile(ptr, value);
        }
        LocalBox { ptr, alloc }
    }

    pub unsafe fn from_raw_in(ptr: *mut E, alloc: CQueueLLAllocator) -> LocalBox<E> {
        LocalBox { ptr, alloc }
    }
}

impl<E> Deref for LocalBox<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<E> DerefMut for LocalBox<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<E> Drop for LocalBox<E> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.ptr);

            let ptr = NonNull::new(self.ptr.cast::<u8>()).unwrap();
            self.alloc.deallocate(ptr, Layout::new::<E>());
        }
    }
}
