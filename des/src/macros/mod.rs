#[macro_use]
mod cfg;

#[macro_use]
mod guid;

#[doc(hidden)]
pub mod support;

cfg_macros! {
    #[macro_use]
    mod event_set;

    cfg_ndl! {
        mod registry;
    }
}
