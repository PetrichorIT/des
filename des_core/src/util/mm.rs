//!
//!  Memory management
//!

use std::{
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    rc::Rc,
};

///
/// A version of [Rc] that allows internal mutation without explicit
/// syncroniszation (in single threaded enviroments).
///
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
