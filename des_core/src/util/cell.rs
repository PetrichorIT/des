use std::cell::UnsafeCell;

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
