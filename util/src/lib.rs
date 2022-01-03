pub mod bench;

#[macro_export]
macro_rules! create_global_uid {
    ($(
        $(#[$outer:meta])*
        $vis: vis $ident: ident($ty: ty) =
        $sident: ident,
    )+) => {

        $(
            $(#[$outer])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[repr(transparent)]
            $vis struct $ident($ty);

            static mut $sident: $ty = 0xff;

            impl $ident {
                $vis fn gen() -> Self {
                    unsafe {
                        let a = $sident;
                        $sident += 1;
                        Self(a)
                    }
                }

                #[allow(unused)]
                $vis fn raw(&self) -> $ty {
                    self.0
                }
            }

            impl From<$ty> for $ident {
                fn from(raw_id: $ty) -> Self {
                    Self(raw_id)
                }
            }

            impl From<$ident> for $ty {
                fn from(wrapped: $ident) -> Self {
                    wrapped.0
                }
            }

            impl std::fmt::Display for $ident {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::Display::fmt(&self.0, f)
                }
            }
        )+
    };
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
