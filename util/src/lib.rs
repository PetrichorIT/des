pub mod bench;

///
/// A decl. macro for creating numeric global UIDs.
///
/// # Syntax
///
/// ```
/// create_global_uid!(
///     pub MessageId(u32) = MESSAGE_ID_STATIC;
///     pub(crate) packetId(u16) = PKT_ID_STATIC;
/// );
/// ```
///
/// # Note
///
/// The inner type must be numeric and initalizable from a numeric interger literal.
/// Supported types are u* and i*.
///
#[macro_export]
macro_rules! create_global_uid {
    ($(
        $(#[$outer:meta])*
        $vis: vis $ident: ident($ty: ty) = $sident: ident;
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
/// A decl. macro for generating a event set.
///
/// # Syntax
///
/// ```
/// create_event_set!(
///     pub enum EventSet {
///         type App = NetworkRuntime<A>;
///
///         EventA(A),
///         EventB(B),
///     };
/// );
/// ```
///
#[macro_export]
macro_rules! create_event_set {

    (
        $(#[$outer:meta])*
        $vis: vis enum $ident: ident {
            type App = $ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )? ),* >;

            $(
                $variant: ident($variant_ty: ty),
            )+
        };
    ) => {
        $(#[$outer])*
        $vis enum $ident {
            $(
                $variant($variant_ty),
            )+
        }

        impl< $( $N $(: $b0 $(+$b)* )? ),* > EventSet<$ty< $( $N ),* >> for $ident {
            fn handle(self, rt: &mut Runtime<$ty< $( $N ),* >>) {
                match self {
                    $(
                        Self::$variant(event) => event.handle(rt),
                    )+
                }
            }
        }

        $(
            impl From<$variant_ty> for $ident {
                fn from(variant: $variant_ty) -> Self {
                    Self::$variant(variant)
                }
            }
        )+
    };
    (
        $(#[$outer:meta])*
        $vis: vis enum $ident: ident {
            type App = $ty:ident;

            $(
                $variant: ident($variant_ty: ty),
            )+
        };
    ) => {
        create_event_set!(
            $vis enum $ident {
                type App = $ty<>;

                $(
                    $variant($variant_ty),
                )+
            };
        );
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
