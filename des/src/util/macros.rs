///
/// A decl. macro for creating numeric global UIDs.
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
                ///
                /// A general prupose identifier for a empty
                /// instance.
                ///
                pub const NULL: Self = Self(0);

                ///
                /// Generates a new unique id.
                ///
                pub fn gen() -> Self {
                    unsafe {
                        let a = $sident;
                        $sident += 1;
                        Self(a)
                    }
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

#[macro_export]
macro_rules! static_ref {
    ($e:expr) => {
        unsafe {
            let ptr: *const _ = $e;
            let r: &'static _ = &*ptr;
            r
        }
    };
}
