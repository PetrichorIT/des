use crate::{
    attributes::Attributes,
    common::{get_resolver, ident_from_conident, WrappedTokenStream},
};
use ndl::ChildModuleSpec;
use ndl::ConSpec;
use ndl::GateAnnotation;
use ndl::GateSpec;
use ndl::{ChannelSpec, TySpec};
use proc_macro2::{Ident, Span};
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::{Data, DataStruct, FieldsNamed, FieldsUnnamed, Type};

use crate::common::*;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

pub fn derive_module_impl(ident: Ident, data: Data, attrs: Attributes) -> Result<TokenStream> {
    let mut token_stream = TokenStream::new();

    generate_static_implementations(ident.clone(), data, &mut token_stream)?;
    generate_dynamic_builder(ident, &attrs, &mut token_stream)?;

    // final check for ndl errors
    if let Some(workspace) = attrs.workspace {
        if let Ok((_, has_err, _)) = get_resolver(&workspace) {
            if has_err {
                Diagnostic::new(Level::Error, String::from("Some NDL error occured")).emit();
            }
        }
    }

    Ok(token_stream)
}

fn generate_static_implementations(ident: Ident, data: Data, out: &mut TokenStream) -> Result<()> {
    let token_stream: TokenStream2;

    let elem_ident = match &data {
        syn::Data::Struct(s) => {
            token_stream = gen_named_object(ident.clone(), s)?;

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
        _ => {
            return Err(Diagnostic::new(
                Level::Error,
                "Modules are currently only supported on structs.".into(),
            ))
        }
    };

    if let Some((eident, idx)) = elem_ident {
        out.extend::<TokenStream>(token_stream.into());

        if let Some(eident) = eident {
            out.extend::<TokenStream>(
                quote! {
                    impl ::std::ops::Deref for #ident {
                        type Target = ::des::net::ModuleCore;
                        fn deref(&self) -> &Self::Target {
                            &self.#eident
                        }
                    }
                    impl ::std::ops::DerefMut for #ident {
                        fn deref_mut(&mut self) -> &mut Self::Target {
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
                    impl ::std::ops::Deref for #ident {
                        type Target = ::des::net::ModuleCore;
                        fn deref(&self) -> &Self::Target {
                            &self.#idx
                        }
                    }
                    impl ::std::ops::DerefMut for #ident {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.#idx
                        }
                    }
                }
                .into(),
            );
        }
    } else {
        return Err(Diagnostic::new(
            Level::Error,
            "Failed to find a field containing a module core.".to_string(),
        )
        .help(String::from("Try adding a module core to the struct.")));
    }

    Ok(())
}

fn gen_named_object(ident: Ident, data: &DataStruct) -> Result<TokenStream2> {
    match &data.fields {
        syn::Fields::Named(FieldsNamed { named, .. }) => {
            if named.len() == 1 {
                let field = named.first().unwrap().ident.clone().unwrap();
                Ok(quote! {
                    impl ::des::net::NameableModule for #ident {
                        fn named(core: ::des::net::ModuleCore) -> Self {
                            Self { #field: core }
                        }
                    }
                })
            } else {
                Ok(TokenStream2::new())
            }
        }
        syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            if unnamed.len() == 1 {
                Ok(quote! {
                    impl ::des::net::NameableModule for #ident {
                        fn named(core: ::des::net::ModuleCore) -> Self {
                            Self(core)
                        }
                    }
                })
            } else {
                Ok(TokenStream2::new())
            }
        }
        _ => Ok(TokenStream2::new()),
    }
}

macro_rules! ident {
    ($e:expr) => {
        proc_macro2::Ident::new(
            &$e.replace("[", "").replace("]", ""),
            proc_macro2::Span::call_site(),
        )
    };
}

fn generate_dynamic_builder(ident: Ident, attrs: &Attributes, out: &mut TokenStream) -> Result<()> {
    if let Some(workspace) = &attrs.workspace {
        match get_resolver(workspace) {
            Ok((res, _, _)) => {
                let module = if let Some(ident) = &attrs.ident {
                    // TODO
                    // Not yet possible since ndl_ident was not yet added to attributes
                    // First implement mapping inside resolver.
                    res.module(ident.to_string())
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name.")
                } else {
                    res.module(ident.clone())
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name.")
                };

                let mut token_stream = proc_macro2::TokenStream::new();

                // Submodule configuration

                let mut proto_t_counter = 0;
                for module in &module.submodules {
                    let ChildModuleSpec { descriptor, ty, .. } = module;

                    let ident = ident!(format!("{}_child", descriptor));
                    let ty = match ty {
                        TySpec::Static(s) => ident!(s),
                        TySpec::Dynamic(_) => {
                            let ident = ident!(format!("T{}", descriptor));
                            proto_t_counter += 1;
                            ident
                        }
                    };
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: ::des::util::Mrc<#ty> = #ty::build_named_with_parent(#descriptor, &mut this, rt);
                    })
                }

                // Gate configuration

                for gate in &module.gates {
                    let GateSpec { ident, size, .. } = gate;
                    let typ = Ident::new(
                        match gate.annotation {
                            GateAnnotation::Input => "Input",
                            GateAnnotation::Output => "Output",
                            GateAnnotation::Unknown => "Undefined",
                        },
                        Span::call_site(),
                    );
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let _ = this.create_gate_cluster(#ident, #size, ::des::net::GateServiceType::#typ, rt);
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
                            #from_ident.set_next_gate(#to_ident.make_readonly());
                            #from_ident.set_channel(channel);
                        });
                    } else {
                        token_stream.extend(quote! {
                                #from_ident.set_next_gate(#to_ident.make_readonly());
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

                out.extend::<TokenStream>(build_impl_from(
                    ident,
                    wrapped,
                    &module.submodules,
                    proto_t_counter,
                ));

                Ok(())
            }
            Err(_) => Err(Diagnostic::new(
                Level::Error,
                "#[derive(Module)] -- Failed to parse NDl file.".into(),
            )),
        }
    } else {
        Ok(())
    }
}
