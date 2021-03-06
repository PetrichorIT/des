#![allow(deprecated)]
#![feature(track_path)]
//!
//! A crate for extending a DES simulation with NDL definitions.
//!
//! This crate provide macros for applieing NDL module definitions to
//! rust structs to automate the module setup process.
//!

mod attributes;
mod common;

mod module;
mod subsystem;

use proc_macro::{self, TokenStream};
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, AttributeArgs, DeriveInput};

///
/// A macro for generating module specific code based on static
/// analysis and a corresponding [ndl] definition.
///
/// This macro implements [Deref](std::ops::Deref) and [DerefMut](std::ops::DerefMut)
/// for ModuleCore by either using an exisiting  module core,
/// or adding one under the key '__core' for structs, or '.0' for enum-structs.
///
/// On the other hand this macro will try to find a [ndl] definition for this module.
/// This definition must have the same name as the module and a workspace must be provided in the
/// macro definition. If one ist found Buildable traits will be implemented to allow other
/// modules or subsystem to build this module.
///
/// # Errors
///
/// This macro may fail if:
/// - No workspace was provided
/// - The [ndl] parser throws errors.
///
/// This macro may create invalid code if:
/// - the submodules used in the [ndl] definition are not in scope.
/// - the definition has more that 9 prototype parameters
/// - the crate 'des' is not in scope.
///
#[proc_macro_attribute]
#[proc_macro_error]
#[allow(non_snake_case)]
pub fn NdlModule(attr: TokenStream, item: TokenStream) -> TokenStream {
    // PARSE ATTRIBUTES
    let attrs = parse_macro_input!(attr as AttributeArgs);

    // PARSE STRUCT DEFINITION
    let inp = parse_macro_input!(item as DeriveInput);

    match module::derive_impl(inp, attrs) {
        Ok(token_stream) => token_stream,
        Err(e) => e.abort(),
    }
}

#[proc_macro_attribute]
#[proc_macro_error]
#[allow(non_snake_case)]
pub fn NdlSubsystem(attr: TokenStream, item: TokenStream) -> TokenStream {
    // PARSE ATTRIBUTES
    let attrs = parse_macro_input!(attr as AttributeArgs);

    // PARSE STRUCT DEFINITION
    let inp = parse_macro_input!(item as DeriveInput);

    match subsystem::derive_impl(inp, attrs) {
        Ok(token_stream) => token_stream,
        Err(e) => e.abort(),
    }
}
