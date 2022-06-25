use crate::{
    attributes::Attributes,
    common::{get_resolver, ident_from_conident, WrappedTokenStream},
};
use ndl::ChildNodeSpec;
use ndl::ConSpec;
use ndl::GateAnnotation;
use ndl::GateSpec;
use ndl::{ChannelSpec, TySpec};
use proc_macro2::{Ident, Span};
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::Data;

use crate::common::*;
use proc_macro::TokenStream;

pub fn derive_module_impl(ident: Ident, data: Data, attrs: Attributes) -> Result<TokenStream> {
    let mut token_stream = TokenStream::new();

    generate_deref_impl(ident.clone(), data, "ModuleCore", &mut token_stream)?;
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
                    res.module(ident)
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name.")
                } else {
                    res.module(&ident.to_string())
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name.")
                };

                let mut token_stream = proc_macro2::TokenStream::new();

                // Submodule configuration

                let mut proto_t_counter = 0;
                for module in &module.submodules {
                    let ChildNodeSpec { descriptor, ty, .. } = module;

                    let ident = ident!(format!("{}_child", descriptor));
                    let ty = match ty {
                        TySpec::Static(s) => ident!(s.inner()),
                        TySpec::Dynamic(_) => {
                            let ident = ident!(format!("T{}", descriptor));
                            proto_t_counter += 1;
                            ident
                        }
                    };
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        let mut #ident: ::des::util::PtrMut<#ty> = #ty::build_named_with_parent(#descriptor, &mut this, rt);
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
                            cost,
                            ..
                        } = channel;

                        token_stream.extend(quote! {
                            let channel = ::des::net::Channel::new(::des::net::ChannelMetrics {
                                bitrate: #bitrate,
                                latency: ::des::time::Duration::from(#latency),
                                jitter: ::des::time::Duration::from(#jitter),
                                cost: #cost,
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
                    let ChildNodeSpec { descriptor, .. } = module;
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
