#[macro_use]
mod cfg;

#[doc(hidden)]
pub mod support;

cfg_macros! {
    cfg_net! {
        mod registry;
    }
}
