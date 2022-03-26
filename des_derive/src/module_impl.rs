use crate::{
    attributes::Attributes,
    common::{get_resolver, ident_from_conident, WrappedTokenStream},
};
use ndl::ChannelSpec;
use ndl::ChildModuleSpec;
use ndl::ConSpec;
use ndl::GateSpec;
use proc_macro2::Ident;
use quote::quote;
use syn::{Data, DataStruct, FieldsNamed, FieldsUnnamed, Type};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

pub fn derive_module_impl(ident: Ident, data: Data, attrs: Attributes) -> TokenStream {
    let mut token_stream = TokenStream::new();

    generate_static_implementations(ident.clone(), data, &mut token_stream);
    generate_dynamic_builder(ident, attrs, &mut token_stream);

    token_stream
}

fn generate_static_implementations(ident: Ident, data: Data, out: &mut TokenStream) {
    let token_stream: TokenStream2;

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
        out.extend::<TokenStream>(token_stream.into());

        if let Some(eident) = eident {
            out.extend::<TokenStream>(
                quote! {
                    impl ::des::net::StaticModuleCore for #ident {
                        fn module_core(&self) -> &::des::net::ModuleCore {
                            &self.#eident
                        }

                        fn module_core_mut(&mut self) -> &mut ::des::net::ModuleCore {
                            &mut self.#eident
                        }
                    }
                }
                .into(),
            );
        } else {
            let idx = syn::Index::from(idx);
            out.extend::<TokenStream>(
                quote! {
                    impl ::des::net::StaticModuleCore for #ident {
                        fn module_core(&self) -> &::des::net::ModuleCore {
                            &self.#idx
                        }

                        fn module_core_mut(&mut self) -> &mut ::des::net::ModuleCore {
                            &mut self.#idx
                        }
                    }
                }
                .into(),
            );
        }
    } else {
        panic!("#[derive(Module)] -- No assosicated module core field found.")
    }
}

fn gen_named_object(ident: Ident, data: &DataStruct) -> TokenStream2 {
    match &data.fields {
        syn::Fields::Named(FieldsNamed { named, .. }) => {
            if named.len() == 1 {
                let field = named.first().unwrap().ident.clone().unwrap();
                quote! {
                    impl ::des::net::NameableModule for #ident {
                        fn named(core: ::des::net::ModuleCore) -> Self {
                            Self { #field: core }
                        }
                    }
                }
            } else {
                TokenStream2::new()
            }
        }
        syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            if unnamed.len() == 1 {
                quote! {
                    impl ::des::net::NameableModule for #ident {
                        fn named(core: ::des::net::ModuleCore) -> Self {
                            Self(core)
                        }
                    }
                }
            } else {
                TokenStream2::new()
            }
        }
        _ => TokenStream2::new(),
    }
}

macro_rules! ident {
    ($e:expr) => {
        proc_macro2::Ident::new(
            &$e.as_str().replace("[", "").replace("]", ""),
            proc_macro2::Span::call_site(),
        )
    };
}

fn generate_dynamic_builder(ident: Ident, attrs: Attributes, out: &mut TokenStream) {
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

                    let ident = ident!(format!("{}_child", descriptor));
                    let ty = ident!(ty);
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: ::des::util::Mrc<#ty> = #ty::build_named_with_parent(#descriptor, &mut this, rt);
                    })
                }

                // Gate configuration

                for gate in &module.gates {
                    let GateSpec { ident, size, .. } = gate;
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let _ = this.create_gate_cluster(#ident, #size, rt);
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
                            let channel = ::des::net::Channel::new(::des::net::ChannelMetrics {
                                bitrate: #bitrate,
                                latency: ::des::core::SimTime::from(#latency),
                                jitter: ::des::core::SimTime::from(#jitter),
                            });
                            #from_ident.set_next_gate(#to_ident);
                            #from_ident.set_channel(channel);
                        });
                    } else {
                        token_stream.extend(quote! {
                                #from_ident.set_next_gate(#to_ident);
                        });
                    }
                }

                // Add submodule to rt

                for module in &module.submodules {
                    let ChildModuleSpec { descriptor, .. } = module;
                    let ident = ident!(format!("{}_child", descriptor));

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.create_module(#ident);
                    })
                }

                // Compile token stream

                let wrapped = WrappedTokenStream(token_stream);

                out.extend::<TokenStream>(quote! {
                    impl ::des::net::BuildableModule for #ident {
                        fn build<A>(mut this: ::des::util::Mrc<Self>, rt: &mut ::des::net::NetworkRuntime<A>) -> ::des::util::Mrc<Self> {
                            #wrapped
                            this
                        }
                    }
                }
                .into())
            }
            Err(_) => panic!("#[derive(Module)] -- Failed to parse NDl file."),
        }
    } else {
        out.extend::<TokenStream>(
            quote! {
                impl ::des::net::BuildableModule for #ident {}
            }
            .into(),
        );
    }
}
