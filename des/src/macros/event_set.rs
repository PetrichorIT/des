///
/// A declaritive macro for generating event sets.
///
/// This macro combines an number of types that implement [`Event`](crate::runtime::Event)
/// into an `EventSet` for an application 'App'.
///
/// ```rust
/// # use des::prelude::*;
/// # use des::event_set;
/// struct PingEvent;
/// struct PongEvent;
/// /* ... */
/// # impl Event<App> for PingEvent { fn handle(self, rt: &mut Runtime<App>) { } }
/// # impl Event<App> for PongEvent { fn handle(self, rt: &mut Runtime<App>) { } }
///
/// struct App;
/// impl Application for App {
///     /* ... */
/// #   type EventSet = Events;
/// #   type Lifecycle = ();
/// }
///
/// event_set! {
///     #[derive(Debug)]
///     pub enum Events {
///         type App = App;
///         
///         PingEvent(PingEvent),
///         PongEvent(PongEvent),
///     };
/// }
/// ```
#[macro_export]
macro_rules! event_set {

    (
        $(#[$outer:meta])*
        $vis: vis enum $ident: ident {
            type App = $ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )? ),* >;

            $(
                $variant:ident($variant_ty: ty),
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
        $crate::event_set!(
            $vis enum $ident {
                type App = $ty<>;

                $(
                    $variant($variant_ty),
                )+
            };
        );
    };
}
