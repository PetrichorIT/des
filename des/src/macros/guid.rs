///
/// A decl. macro for creating numeric global UIDs.
///
/// # Note
///
/// The inner type must be numeric and initalizable from a numeric interger literal.
/// Supported types are u* and i*.
///
#[allow(unused)]
macro_rules! guid {
    ($(
        $(#[$outer:meta])*
        $vis: vis $ident: ident($ty: ty) = $sident: ident;
    )+) => {

        $(
            $(#[$outer])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[repr(transparent)]
            $vis struct $ident(pub $ty);

            static $sident: $crate::macros::support::SyncWrap<::std::cell::Cell<$ty>> = $crate::macros::support::SyncWrap::new(::std::cell::Cell::new(0xff));

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
                    let a = $sident.get();
                    $sident.set(a + 1);
                    Self(a)
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
