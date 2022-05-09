#![feature(track_path)]
//!
//! A crate for extending a DES simulation with NDL definitions.
//!
//! This crate provide macros for applieing NDL module definitions to
//! rust structs to automate the module setup process.
//!

mod attributes;
mod common;
mod module_impl;
mod network_impl;

use attributes::*;
use proc_macro::{self, TokenStream};
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, DeriveInput};

///
/// A macro for generating implementations for a Module based on
/// static analysis and NDL files.
///
/// This macro inmplements the StaticModuleCore trait
/// and the NdlBuildableModule trait.
///
/// Thereby the StaticModuleCore trait will be derived by performing static analysis
/// over the fields of the struct the macro used on.
/// If one of the fields has the type ModuleCore it will be used to implement
/// the module_core() and module_core_mut() required funtions of the StaticModuleCore trait.
///
/// On the other hand the NdlBuildableModule trait will be implemented
/// in a placeholder way independent whether a NDL module was provided.
/// If one was provided the build method will be implemented according to the
/// specifications of the NDL module.
/// To make this possible a ndl workspace must be provided as attribute, and this
/// workspace must contain a module with the same name as the Rust struct the macro
/// is appllied to.
///
/// # Note
///
/// Make sure all modules types used submodule definitions are in scope.
///
#[proc_macro_derive(Module, attributes(ndl_workspace))]
#[proc_macro_error]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let attrs = Attributes::from_attr(attrs);

    match module_impl::derive_module_impl(ident, data, attrs) {
        Ok(token_stream) => token_stream,
        Err(e) => e.abort(),
    }
}

///
/// A macro for generating build functions for a network in a DES simulation.
///
/// This macro inmplements three functions:
/// - run
/// - run_with_options
/// - build_rt
///
/// The build_rt function allows the struct the macro is applied to to generate a
/// NetworkRuntime where A is the struct itself.
/// This network runtime has preconfigured modules and connections according to the
/// networks NDL specification and intern the used modules NDL specification.
///
/// The run and run_with_options functions present a way of automaticlly excuting the simulation
/// upon runtime creation.
///
/// # Note
///
/// Make sure all modules used in the top-level sepcification of the network are in scope.
///

#[proc_macro_derive(Network, attributes(ndl_workspace))]
#[proc_macro_error]
pub fn derive_network(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, attrs, .. } = parse_macro_input!(input);
    let attrs = Attributes::from_attr(attrs);

    match network_impl::derive_network_impl(ident, attrs) {
        Ok(token_stream) => token_stream,
        Err(e) => e.abort(),
    }
}
