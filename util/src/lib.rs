pub mod bench;

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

    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }
}

impl<T: ?Sized> SyncCell<T> {
    pub fn get(&self) -> *mut T {
        self.cell.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.cell.get_mut()
    }
}

unsafe impl<T: ?Sized> Sync for SyncCell<T> {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
