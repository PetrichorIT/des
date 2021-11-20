use std::ops::{Deref, DerefMut};

#[repr(transparent)]
pub struct OrdVec<T: Ord> {
    inner: Vec<T>,
}

#[allow(dead_code)]
impl<T: Ord> OrdVec<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, item: T) {
        self.inner.push(item);
        self.inner.sort();
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }
}

impl<T: Ord> Deref for OrdVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: Ord> DerefMut for OrdVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}
