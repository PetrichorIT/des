//!
//! A crate for extending a DES simulation with NDL definitions.
//!
//! This crate provide macros for applieing NDL module definitions to
//! rust structs to automate the module setup process.
//!

mod attributes;

use attributes::*;
use des_ndl::*;
use lazy_static::lazy_static;
use proc_macro::{self, TokenStream};
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote;
use quote::ToTokens;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use syn::DataStruct;
use syn::{parse_macro_input, Data, DeriveInput, FieldsNamed, FieldsUnnamed, Type};

lazy_static! {
    static ref RESOLVERS: Mutex<HashMap<String, NdlResolver>> = Mutex::new(HashMap::new());
}

fn get_resolver(workspace: &str) -> Result<(OwnedTySpecContext, bool, Vec<PathBuf>), &'static str> {
    let mut resolvers = RESOLVERS.lock().unwrap();

    if !resolvers.contains_key(workspace) {
        resolvers.insert(
            workspace.to_owned(),
            NdlResolver::new(workspace).expect("#[derive(Module)] Invalid NDL workspace."),
        );
    }
    resolvers
        .get_mut(workspace)
        .unwrap()
        .run_cached()
        .map(|(gtyctx, has_err, pars)| (gtyctx.to_owned(), has_err, pars))
}

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
pub fn derive_module(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let attrs = Attributes::from_attr(attrs);

    let mut static_gen = gen_static_module_core(ident.clone(), data);
    let dynamic_gen = gen_dynamic_module_core(ident, attrs);

    static_gen.extend(dynamic_gen.into_iter());

    static_gen
}

fn gen_static_module_core(ident: Ident, data: Data) -> TokenStream {
    let token_stream: proc_macro2::TokenStream;

    let elem_ident = match &data {
        syn::Data::Struct(s) => {
            token_stream = gen_named_object(ident.clone(), s);

            match &s.fields {
                syn::Fields::Named(FieldsNamed { named, .. }) => named
                    .iter()
                    .find(|f| {
                        if let Type::Path(ty) = &f.ty {
                            ty.path.segments.last().unwrap().ident
                                == Ident::new(
                                    "ModuleCore",
                                    ty.path.segments.last().unwrap().ident.span(),
                                )
                        } else {
                            false
                        }
                    })
                    .map(|field| (Some(field.ident.clone().unwrap()), 0)),
                syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => unnamed
                    .iter()
                    .enumerate()
                    .find(|(_, f)| {
                        if let Type::Path(ty) = &f.ty {
                            ty.path.segments.last().unwrap().ident
                                == Ident::new(
                                    "ModuleCore",
                                    ty.path.segments.last().unwrap().ident.span(),
                                )
                        } else {
                            false
                        }
                    })
                    .map(|(idx, _)| (None, idx)),
                syn::Fields::Unit => None,
            }
        }
        _ => unimplemented!(),
    };

    if let Some((eident, idx)) = elem_ident {
        if let Some(eident) = eident {
            let mut ts = quote! {
                impl ::des_core::StaticModuleCore for #ident {
                    fn module_core(&self) -> &::des_core::ModuleCore {
                        &self.#eident
                    }

                    fn module_core_mut(&mut self) -> &mut ::des_core::ModuleCore {
                        &mut self.#eident
                    }
                }

                impl ::des_core::Indexable for #ident {
                    type Id = ::des_core::ModuleId;

                    fn id(&self) -> ::des_core::ModuleId {
                        use ::des_core::StaticModuleCore;
                        self.module_core().id()
                    }
                }
            };

            ts.extend(token_stream);
            ts.into()
        } else {
            let idx = syn::Index::from(idx);
            let mut ts = quote! {
                impl ::des_core::StaticModuleCore for #ident {
                    fn module_core(&self) -> &::des_core::ModuleCore {
                        &self.#idx
                    }

                    fn module_core_mut(&mut self) -> &mut ::des_core::ModuleCore {
                        &mut self.#idx
                    }
                }

                impl ::des_core::Indexable for #ident {
                    type Id = ::des_core::ModuleId;

                    fn id(&self) -> ::des_core::ModuleId {
                        use ::des_core::StaticModuleCore;
                        self.module_core().id()
                    }
                }
            };

            ts.extend(token_stream);
            ts.into()
        }
    } else {
        panic!("#[derive(Module)] -- No assosicated module core field found.")
    }
}

fn gen_named_object(ident: Ident, data: &DataStruct) -> proc_macro2::TokenStream {
    match &data.fields {
        syn::Fields::Named(FieldsNamed { named, .. }) => {
            if named.len() == 1 {
                let field = named.first().unwrap().ident.clone().unwrap();
                quote! {
                    impl ::des_core::NameableModule for #ident {
                        fn named(core: ::des_core::ModuleCore) -> Self {
                            Self { #field: core }
                        }
                    }
                }
            } else {
                proc_macro2::TokenStream::new()
            }
        }
        syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            if unnamed.len() == 1 {
                quote! {
                    impl ::des_core::NameableModule for #ident {
                        fn named(core: ::des_core::ModuleCore) -> Self {
                            Self(core)
                        }
                    }
                }
            } else {
                proc_macro2::TokenStream::new()
            }
        }
        _ => proc_macro2::TokenStream::new(),
    }
}

fn gen_dynamic_module_core(ident: Ident, attrs: Attributes) -> TokenStream {
    if let Some(workspace) = &attrs.workspace {
        match get_resolver(workspace) {
            Ok((res, has_errors, _)) => {
                if has_errors {
                    panic!("#[derive(Module)] NDL resolver found erros while parsing")
                }

                let module = if let Some(ident) = attrs.ident {
                    // TODO
                    // Not yet possible since ndl_ident was not yet added to attributes
                    // First implement mapping inside resolver.
                    res.module(ident)
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name.")
                } else {
                    res.module(ident.clone())
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name.")
                };

                let mut token_stream = proc_macro2::TokenStream::new();

                // Submodule configuration

                for module in &module.submodules {
                    let ChildModuleSpec { descriptor, ty, .. } = module;
                    let ident = Ident::new(&format!("{}_child", descriptor), Span::call_site());
                    let ty = Ident::new(ty, Span::call_site());
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: Box<#ty> = #ty::build_named_with_parent(#descriptor, &mut self, rt);
                    })
                }

                // Gate configuration

                for gate in &module.gates {
                    let GateSpec { ident, size, .. } = gate;
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let _ = self.create_gate_cluster(#ident, #size, rt);
                    })
                }

                // Connection configuration.

                for connection in &module.connections {
                    let ConSpec {
                        source,
                        channel,
                        target,
                        ..
                    } = connection;

                    // get gate cluster for specific nodes
                    let to_ident = ident_from_conident(&mut token_stream, target);
                    token_stream.extend(quote! {
                        let #to_ident = #to_ident.id();
                    });

                    let from_ident = ident_from_conident(&mut token_stream, source);
                    // Define n channels (n == gate_cluster.size())
                    if let Some(channel) = channel {
                        let ChannelSpec {
                            bitrate,
                            latency,
                            jitter,
                            ..
                        } = channel;

                        token_stream.extend(quote! {
                            let channel = rt.create_channel(des_core::ChannelMetrics {
                                bitrate: #bitrate,
                                latency: des_core::SimTime::from(#latency),
                                jitter: des_core::SimTime::from(#jitter),
                            });
                            #from_ident.set_next_gate(#to_ident);
                            #from_ident.set_channel(channel);
                        });
                    } else {
                        token_stream.extend(quote! {
                            // assert_eq!(#from_ident.len(), #to_ident.len());
                            // for i in 0..#from_ident.len() {
                                #from_ident.set_next_gate(#to_ident);
                            // }
                        });
                    }
                }

                // Add submodule to rt

                for module in &module.submodules {
                    let ChildModuleSpec { descriptor, .. } = module;
                    let ident = Ident::new(&format!("{}_child", descriptor), Span::call_site());

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.create_module(#ident);
                    })
                }

                // Compile token stream

                let wrapped = WrappedTokenStream(token_stream);

                quote! {
                    impl ::des_core::BuildableModule for #ident {
                        fn build<A>(mut self: Box<Self>, rt: &mut des_core::NetworkRuntime<A>) -> Box<Self> {
                            #wrapped
                            self
                        }
                    }
                }
                .into()
            }
            Err(_) => panic!("#[derive(Module)] -- Failed to parse NDl file."),
        }
    } else {
        quote! {
            impl ::des_core::BuildableModule for #ident {}
        }
        .into()
    }
}

/// Resolve a concreate conident to the associated gate clusters
fn ident_from_conident(
    token_stream: &mut proc_macro2::TokenStream,
    ident: &ConSpecNodeIdent,
) -> Ident {
    match ident {
        ConSpecNodeIdent::Child {
            child_ident,
            gate_ident,
            pos,
            ..
        } => {
            let submodule_ident = Ident::new(&format!("{}_child", child_ident), Span::call_site());
            let ident_token = Ident::new(
                &format!("{}_child_{}_gate{}", child_ident, gate_ident, pos),
                Span::call_site(),
            );

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident_token: &mut des_core::Gate = #submodule_ident.gate_mut(#gate_ident, #pos)
                    .expect("Internal macro err.");
            });

            ident_token
        }
        ConSpecNodeIdent::Local {
            gate_ident, pos, ..
        } => {
            let ident = Ident::new(
                &format!("{}_gate{}_ref", gate_ident, pos),
                Span::call_site(),
            );

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident: &mut des_core::Gate = self.gate_mut(#gate_ident, #pos)
                    .expect("Internal macro err.");
            });

            ident
        }
    }
}

//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//

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
pub fn derive_network(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, attrs, .. } = parse_macro_input!(input);

    let attrs = Attributes::from_attr(attrs);

    gen_network_main(ident, attrs)
}

fn gen_network_main(ident: Ident, attrs: Attributes) -> TokenStream {
    match get_resolver(
        &attrs
            .workspace
            .expect("#[derive(Network)] Missing NDL worspace"),
    ) {
        Ok((res, has_errors, par_files)) => {
            if has_errors {
                panic!("#[derive(Network)] NDL resolver found erros while parsing")
            }

            let network = if let Some(ident) = attrs.ident {
                // TODO
                // Not yet possible since ndl_ident was not yet added to attributes
                // First implement mapping inside resolver.
                res.network(ident)
            } else {
                res.network(ident.clone())
            };

            if let Some(network) = network {
                let mut token_stream = proc_macro2::TokenStream::new();

                // Import parameters

                for par_file in par_files {
                    let string_literal = par_file.to_str().unwrap();
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.include_par_file(#string_literal);
                    })
                }

                // Config nodes.

                for node in &network.nodes {
                    let ChildModuleSpec { descriptor, ty, .. } = node;
                    let ident = Ident::new(&format!("{}_child", descriptor), Span::call_site());
                    let ty = Ident::new(ty, Span::call_site());
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: Box<#ty> = #ty::build_named(#descriptor.parse().unwrap(), rt);
                    })
                }

                // Create connections.

                for connection in &network.connections {
                    let ConSpec {
                        source,
                        channel,
                        target,
                        ..
                    } = connection;

                    // get gate cluster for specific nodes
                    let from_ident = ident_from_conident(&mut token_stream, source);
                    let to_ident = ident_from_conident(&mut token_stream, target);

                    // Define n channels (n == gate_cluster.size())
                    if let Some(channel) = channel {
                        let ChannelSpec {
                            bitrate,
                            latency,
                            jitter,
                            ..
                        } = channel;

                        token_stream.extend(quote! {
                            let channel = rt.create_channel(des_core::ChannelMetrics {
                                bitrate: #bitrate,
                                latency: des_core::SimTime::from(#latency),
                                jitter: des_core::SimTime::from(#jitter),
                            });
                            #from_ident.set_next_gate(#to_ident.id());
                            #from_ident.set_channel(channel);
                        });
                    } else {
                        token_stream.extend(quote! {
                            // assert_eq!(#from_ident.len(), #to_ident.len());
                            for i in 0..#from_ident.len() {
                                #from_ident[i].set_next_gate(#to_ident[i].id());
                            }
                        });
                    }
                }
                // Add nodes to rt.

                for node in &network.nodes {
                    let ChildModuleSpec { descriptor, .. } = node;
                    let ident = Ident::new(&format!("{}_child", descriptor), Span::call_site());

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.create_module(#ident);
                    })
                }

                // Compile token stream

                let token_stream = WrappedTokenStream(token_stream);

                quote! {
                    impl #ident {
                        pub fn run(self) -> (Self, ::des_core::SimTime) {
                            self.run_with_options(::des_core::RuntimeOptions::default())
                        }

                        pub fn run_with_options(self, options: ::des_core::RuntimeOptions) -> (Self, ::des_core::SimTime) {
                            use ::des_core::Runtime;
                            use ::des_core::NetworkRuntime;

                            let net_rt = self.build_rt();
                            let rt = Runtime::<NetworkRuntime<Self>>::new_with(net_rt, options);
                            let (net_rt, end_time) = rt.run().expect("RT exceeded itr limit.");
                            (net_rt.finish(), end_time)
                        }

                        pub fn build_rt(self) -> ::des_core::NetworkRuntime<Self> {
                            let mut runtime = ::des_core::NetworkRuntime::new(self);
                            let rt: &mut ::des_core::NetworkRuntime<Self> = &mut runtime;

                            #token_stream

                            rt.finish_building();
                            runtime
                        }
                    }
                }.into()
            } else {
                panic!(
                    "#[derive(Network)] NDL resolver failed to find network called '{}'",
                    ident,
                );
            }
        }
        Err(e) => panic!("#[derive(Network)] NDL resolver failed: {}", e),
    }
}

struct WrappedTokenStream(proc_macro2::TokenStream);

impl ToTokens for WrappedTokenStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<proc_macro2::TokenStream>(self.0.clone())
    }
}
