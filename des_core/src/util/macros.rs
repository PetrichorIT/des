///
/// A decl. macro for creating numeric global UIDs.
///
/// # Syntax
///
/// ```
/// use des_core::create_global_uid;
///
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
                ///
                /// Generates a new unqiue instance of Self.
                ///
                $vis fn gen() -> Self {
                    unsafe {
                        let a = $sident;
                        $sident += 1;
                        Self(a)
                    }
                }

                ///
                /// Returns the raw primitiv the UID is contructed over.
                ///
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
