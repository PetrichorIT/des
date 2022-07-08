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
