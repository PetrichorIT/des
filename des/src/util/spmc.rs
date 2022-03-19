use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    cmp::Eq,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    rc::Rc,
};

///
/// A writer to a single-producer multiple-consumer datapoint.
///
pub struct SpmcWriter<T> {
    inner: Rc<UnsafeCell<T>>,
}

impl<T> SpmcWriter<T> {
    ///
    /// Creates a new instance of self.
    ///
    pub fn new(item: T) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(item)),
        }
    }

    ///
    /// Derives a reader from a writer.
    ///
    pub fn get_reader(&self) -> SpmcReader<T> {
        SpmcReader {
            inner: self.inner.clone(),
        }
    }
}

impl<T> AsRef<T> for SpmcWriter<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T> Borrow<T> for SpmcWriter<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T> Deref for SpmcWriter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFTY:
        // Since by definition only one writer can exist, as_ref can also be safe
        // if performed by the writer itself.
        unsafe { &*(*self.inner).get() }
    }
}

impl<T> DerefMut for SpmcWriter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFTY:
        // Since by definition only one writer can exist, as_ref can also be safe
        // if performed by the writer itself.
        unsafe { &mut *(*self.inner).get() }
    }
}

impl<T> Debug for SpmcWriter<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> Display for SpmcWriter<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> PartialEq for SpmcWriter<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T> Eq for SpmcWriter<T> where T: Eq {}

///
/// A reader to a single-producer multipled consumer
/// datapoint.
///
pub struct SpmcReader<T> {
    inner: Rc<UnsafeCell<T>>,
}

impl<T> AsRef<T> for SpmcReader<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T> Borrow<T> for SpmcReader<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T> Deref for SpmcReader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFTY:
        // Since by definition only one reader can exist, as_ref can also be safe
        // if performed by the writer itself.
        unsafe { &*(*self.inner).get() }
    }
}

impl<T> Debug for SpmcReader<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> Display for SpmcReader<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> PartialEq for SpmcReader<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T> Eq for SpmcReader<T> where T: Eq {}

impl<T> Clone for SpmcReader<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
