//!
//!  Memory management
//!

use std::{
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

///
/// A version of [Rc] that allows internal mutation without explicit
/// syncroniszation (in single threaded enviroments).
///
#[derive(Debug)]
pub struct Mrc<T>
where
    T: ?Sized,
{
    inner: Rc<UnsafeCell<T>>,
}

impl<T> Mrc<T> {
    ///
    /// Constructs a new [Mrc<T>]
    ///
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(value)),
        }
    }
}

impl<T> AsRef<T> for Mrc<T>
where
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        // SAFTY:
        // This deref in considered safe since it only extends Mrc
        // with the default Rc behaviour
        unsafe { &*self.inner.as_ref().get() }
    }
}

impl<T> Borrow<T> for Mrc<T>
where
    T: ?Sized,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T> BorrowMut<T> for Mrc<T>
where
    T: ?Sized,
{
    fn borrow_mut(&mut self) -> &mut T {
        // SAFTY:
        // This can be considered a valid extension of the safty contract
        // acording to the type definition
        unsafe { &mut *self.inner.as_ref().get() }
    }
}

impl<T> Clone for Mrc<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Deref for Mrc<T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> DerefMut for Mrc<T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut T {
        // SAFTY:
        // This can be considered a valid extension of the safty contract
        // acording to the type definition
        unsafe { &mut *self.inner.as_ref().get() }
    }
}

///
/// A implementation of UnsafeCell that implements Sync
/// since a corrolated DES simulation is inherintly single threaded.
///
#[repr(transparent)]
#[derive(Debug)]
pub struct SyncCell<T: ?Sized> {
    cell: std::cell::UnsafeCell<T>,
}

impl<T> SyncCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            cell: std::cell::UnsafeCell::new(value),
        }
    }

    #[allow(unused)]
    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }
}

impl<T> SyncCell<T>
where
    T: ?Sized,
{
    pub fn get(&self) -> *mut T {
        self.cell.get()
    }

    #[allow(unused)]
    pub fn get_mut(&mut self) -> &mut T {
        self.cell.get_mut()
    }
}

unsafe impl<T> Sync for SyncCell<T> where T: ?Sized {}

impl<T> Clone for SyncCell<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let r = unsafe { &*self.cell.get() };
        Self {
            cell: UnsafeCell::new(r.clone()),
        }
    }
}
