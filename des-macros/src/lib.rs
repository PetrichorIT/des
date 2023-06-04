#![warn(clippy::pedantic)]
//! A crate for extending a DES simulation with NDL definitions.
//!
//! This crate provide macros for applieing NDL module definitions to
//! rust structs to automate the module setup process.

use proc_macro::{self, TokenStream};
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, DeriveInput};

///
/// A macro for deriving the `MessageBody` trait.
///
/// This macro requires that all subtypes of the applied type
/// implement `MessageBody` themselfs.
#[proc_macro_derive(MessageBody)]
#[proc_macro_error]
pub fn derive_message_body(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = parse_macro_input!(input);

    match des_macros_core::message_body::derive_impl(ident, data, generics) {
        Ok(ts) => ts.into(),
        Err(e) => e.abort(),
    }
}

/// Implementation detail of the `select!` macro. This macro is **not** intended
/// to be used as part of the public API and is permitted to change.
#[proc_macro]
#[doc(hidden)]
pub fn select_priv_declare_output_enum(input: TokenStream) -> TokenStream {
    des_macros_core::select::declare_output_enum(input.into()).into()
}

/// Implementation detail of the `select!` macro. This macro is **not** intended
/// to be used as part of the public API and is permitted to change.
#[proc_macro]
#[doc(hidden)]
pub fn select_priv_clean_pattern(input: TokenStream) -> TokenStream {
    des_macros_core::select::clean_pattern_macro(input.into()).into()
}
