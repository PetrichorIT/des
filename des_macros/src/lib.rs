use des_ndl::*;
use proc_macro::{self, TokenStream};
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote;
use quote::ToTokens;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, FieldsNamed, FieldsUnnamed, Lit, Meta,
    MetaNameValue, Type,
};

///
/// #[derive(Module)]
///

#[proc_macro_derive(Module, attributes(ndl_workspace))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let mut static_gen = gen_static_module_core(ident.clone(), data);
    let dynamic_gen = gen_dynamic_module_core(ident.clone(), attrs);

    static_gen.extend(dynamic_gen.into_iter());

    static_gen
}

fn gen_static_module_core(ident: Ident, data: Data) -> TokenStream {
    let elem_ident = match data {
        syn::Data::Struct(s) => match s.fields {
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
        },
        _ => unimplemented!(),
    };

    if let Some((eident, idx)) = elem_ident {
        if let Some(eident) = eident {
            quote! {
                impl ::des_core::StaticModuleCore for #ident {
                    fn module_core(&self) -> &::des_core::ModuleCore {
                        &self.#eident
                    }

                    fn module_core_mut(&mut self) -> &mut ::des_core::ModuleCore {
                        &mut self.#eident
                    }
                }
            }
            .into()
        } else {
            let idx = syn::Index::from(idx);
            quote! {
                impl ::des_core::StaticModuleCore for #ident {
                    fn module_core(&self) -> &::des_core::ModuleCore {
                        &self.#idx
                    }

                    fn module_core_mut(&mut self) -> &mut ::des_core::ModuleCore {
                        &mut self.#idx
                    }
                }
            }
            .into()
        }
    } else {
        panic!("#[derive(Module)] -- No assosicated module core field found.")
    }
}

fn gen_dynamic_module_core(ident: Ident, attrs: Vec<Attribute>) -> TokenStream {
    if let Some(workspace) = parse_attr(attrs) {
        let mut resolver = NdlResolver::new(&workspace)
            .expect("#[derive(Module)] -- Failed because ndl_workspace is invalid");

        match resolver.run() {
            Ok((res, has_errors)) => {
                if has_errors {
                    panic!("#[derive(Module)] NDL resolver found erros while parsing")
                }

                let module = res
                    .modules
                    .into_iter()
                    .find(|m| ident == m.name)
                    .expect("#[derive(Module)] -- Failed to find NDL module with same name.");

                let mut token_stream = proc_macro2::TokenStream::new();

                // Submodule configuration

                for module in &module.submodules {
                    let ChildeModuleDef { descriptor, ty, .. } = module;
                    let ident = Ident::new(&format!("{}_smod", descriptor), Span::call_site());
                    let ty = Ident::new(ty, Span::call_site());
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: Box<#ty> = #ty::build_named_with_parent(#descriptor, &mut self, rt);
                    })
                }

                // Gate configuration

                for gate in &module.gates {
                    let GateDef { name, size, .. } = gate;
                    let ident = Ident::new(&format!("{}_gate", name), Span::call_site());
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let #ident: Vec<GateId> = self.create_gate_cluster(#name, #size);
                    })
                }

                // Connection configuration.

                for connection in &module.connections {
                    let ConDef {
                        from, channel, to, ..
                    } = connection;

                    let from_ident = ident_from_conident(&mut token_stream, from);
                    let to_ident = ident_from_conident(&mut token_stream, to);

                    if let Some(channel) = channel {
                        let LinkDef {
                            bitrate,
                            latency,
                            jitter,
                            ..
                        } = res
                            .links
                            .iter()
                            .find(|l| l.name == *channel)
                            .expect("unreachable");

                        token_stream.extend(quote! {
                            // assert_eq!(#from_ident.len(), #to_ident.len());
                            for i in 0..#from_ident.len() {
                                let channel = rt.create_channel(des_core::ChannelMetrics {
                                    bitrate: #bitrate,
                                    latency: des_core::SimTime::new(#latency),
                                    jitter: des_core::SimTime::new(#jitter),
                                });
                                #from_ident[i].set_next_gate(#to_ident[i].id());
                                #from_ident[i].set_channel(channel);
                            }
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

                // Add submodule to rt

                for module in &module.submodules {
                    let ChildeModuleDef { descriptor, .. } = module;
                    let ident = Ident::new(&format!("{}_smod", descriptor), Span::call_site());

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.create_module(#ident);
                    })
                }

                // Compile token stream

                let wrapped = WrappedTokenStream(token_stream);

                quote! {
                    impl ::des_core::NdlBuildableModule for #ident {
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
            impl ::des_core::NdlBuildableModule for #ident {}
        }
        .into()
    }
}

fn ident_from_conident(token_stream: &mut proc_macro2::TokenStream, ident: &ConNodeIdent) -> Ident {
    match ident {
        ConNodeIdent::Child { child, ident, .. } => {
            let submodule_ident = Ident::new(&format!("{}_smod", child), Span::call_site());
            let ident_token =
                Ident::new(&format!("{}_smod_{}_gate", child, ident), Span::call_site());

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident_token: Vec<&mut des_core::Gate> = #submodule_ident.gate_cluster_mut(#ident);
            });

            ident_token
        }
        ConNodeIdent::Local {
            ident: gate_name, ..
        } => {
            let ident = Ident::new(&format!("{}_gate_ref", ident), Span::call_site());

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident: Vec<&mut des_core::Gate> = self.gate_cluster_mut(#gate_name);
            });

            ident
        }
    }
}

fn parse_attr(attrs: Vec<Attribute>) -> Option<String> {
    for attr in attrs {
        match attr.parse_meta().unwrap() {
            Meta::NameValue(MetaNameValue {
                ref path, ref lit, ..
            }) => {
                if path.segments.last().unwrap().ident == "ndl_workspace" {
                    match lit {
                        Lit::Str(str) => return Some(str.value()),
                        _ => return None,
                    }
                }
            }
            _ => {}
        }
    }

    None
}

///
/// #[derive(Network)]
///

#[proc_macro_derive(Network, attributes(ndl_workspace))]
pub fn derive_network(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, attrs, .. } = parse_macro_input!(input);

    let attr = parse_attr(attrs).expect("#[derive(Network)] Missing NDL worspace");

    gen_network_main(ident, attr)
}

fn gen_network_main(ident: Ident, workspace: String) -> TokenStream {
    let mut resolver = NdlResolver::new(&workspace)
        .expect("#[derive(Network)] -- Failed because ndl_workspace is invalid");

    match resolver.run() {
        Ok((res, has_errors)) => {
            if has_errors {
                panic!("#[derive(Network)] NDL resolver found erros while parsing")
            }
            if let Some(network) = res.networks.iter().find(|n| ident == n.name) {
                let mut token_stream = proc_macro2::TokenStream::new();

                // Config nodes.

                for node in &network.nodes {
                    let ChildeModuleDef { descriptor, ty, .. } = node;
                    let ident = Ident::new(&format!("{}_node", descriptor), Span::call_site());
                    let ty = Ident::new(ty, Span::call_site());
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: Box<#ty> = #ty::build_named(#descriptor, rt);
                    })
                }

                // Create connections.

                for connection in &network.connections {
                    let ConDef {
                        from, channel, to, ..
                    } = connection;

                    let from_ident = ident_from_conident(&mut token_stream, from);
                    let to_ident = ident_from_conident(&mut token_stream, to);

                    if let Some(channel) = channel {
                        let LinkDef {
                            bitrate,
                            latency,
                            jitter,
                            ..
                        } = res
                            .links
                            .iter()
                            .find(|l| l.name == *channel)
                            .expect("unreachable");

                        token_stream.extend(quote! {
                            // assert_eq!(#from_ident.len(), #to_ident.len());
                            for i in 0..#from_ident.len() {
                                let channel = rt.create_channel(des_core::ChannelMetrics {
                                    bitrate: #bitrate,
                                    latency: des_core::SimTime::new(#latency),
                                    jitter: des_core::SimTime::new(#jitter),
                                });
                                #from_ident[i].set_next_gate(#to_ident[i].id());
                                #from_ident[i].set_channel(channel);
                            }
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
                    let ChildeModuleDef { descriptor, .. } = node;
                    let ident = Ident::new(&format!("{}_node", descriptor), Span::call_site());

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.create_module(#ident);
                    })
                }

                let token_stream = WrappedTokenStream(token_stream);

                quote! {
                    impl #ident {
                        pub fn run(self) -> (Self, des_core::SimTime) {
                            self.run_with_options(des_core::RuntimeOptions::default())
                        }

                        pub fn run_with_options(self, options: des_core::RuntimeOptions) -> (Self, des_core::SimTime) {
                            let net_rt = self.build_rt();
                            let rt = des_core::Runtime::new_with(net_rt, options);
                            let (net_rt, end_time) = rt.run().expect("RT exceeded itr limit.");
                            (net_rt.finish(), end_time)
                        }

                        pub fn build_rt(self) -> NetworkRuntime<Self> {
                            let mut runtime = des_core::NetworkRuntime::new(self);
                            let rt: &mut des_core::NetworkRuntime<Self> = &mut runtime;

                            #token_stream

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

///
/// #[derive(GlobalUID)]
/// will be moved to utils & refactored
/// #deprecated
///

#[proc_macro_derive(GlobalUID)]
pub fn derive_global_uid(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let mut name = ident.to_string().to_uppercase();
    name.push_str("_STATIC");
    let sident = Ident::new(&name, ident.span());

    let output = quote! {

        static mut #sident: #ident = #ident(0xff);

        impl #ident {
            fn gen() -> Self {
                unsafe {
                    let a = #sident;
                    #sident.0 += 1;
                    a
                }
            }
        }

        impl Clone for #ident {
            fn clone(&self) -> Self {
                Self(self.0)
            }
        }

        impl Copy for #ident  {}

        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.0, f)
            }
        }

        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::cmp::PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl std::cmp::Eq for #ident {}

        impl std::hash::Hash for #ident {
            fn hash<H>(&self, state: &mut H)
                where H: std::hash::Hasher {
                    self.0.hash(state)
                }
        }
    };
    output.into()
}

struct WrappedTokenStream(proc_macro2::TokenStream);

impl ToTokens for WrappedTokenStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<proc_macro2::TokenStream>(self.0.clone())
    }
}
