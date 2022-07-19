use crate::attributes::Attr;
use crate::common::*;
use ndl::ChannelSpec;
use ndl::ChildNodeSpec;
use ndl::ConSpec;
use ndl::GateAnnotation;
use ndl::GateSpec;
use ndl::TySpec;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::Visibility;
use syn::{AttributeArgs, Data, DeriveInput, Ident};

pub fn derive_impl(mut input: DeriveInput, attrs: AttributeArgs) -> Result<TokenStream> {
    let attr = Attr::from_args(attrs)?;
    let ident = input.ident.clone();

    // (0) Prepare token streams
    let mut derive_stream = TokenStream::new();

    // (1) Cast to data struct, This macro can only be applied to struct.
    let data = match &mut input.data {
        Data::Struct(data) => data,
        _ => {
            return Err(Diagnostic::new(
                Level::Error,
                "Failed to find a field containing a module core.".to_string(),
            )
            .help(String::from("Try adding a module core to the struct.")))
        }
    };

    // (2) Derive the deref impls / generate that approiated changes in the data struct.
    derive_deref(ident.clone(), data, &mut derive_stream, "ModuleCore")?;
    generate_dynamic_builder(input.vis.clone(), ident, attr, &mut derive_stream)?;

    let mut structdef_stream: TokenStream = quote! {
        #input
    }
    .into();
    structdef_stream.extend::<TokenStream>(derive_stream);

    Ok(structdef_stream)
}

macro_rules! ident {
    ($e:expr) => {
        proc_macro2::Ident::new(
            &$e.replace("[", "").replace("]", ""),
            proc_macro2::Span::call_site(),
        )
    };
}

fn generate_dynamic_builder(
    vis: Visibility,
    ident: Ident,
    attr: Attr,
    out: &mut TokenStream,
) -> Result<()> {
    let workspace = match &attr.workspace {
        Some(ref w) => w,
        None => return Ok(()),
    };

    match get_resolver(workspace) {
        Ok((res, _, _)) => {
            let (module, typalias) = if let Some(ident) = attr.overwrite_ident {
                (
                    res.module(&ident)
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name."),
                    true,
                )
            } else {
                (
                    res.module(&ident.to_string())
                        .expect("#[derive(Module)] -- Failed to find NDL module with same name."),
                    false,
                )
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
                        let mut #ident: ::des::util::PtrMut<#ty> = #ty::build_named_with_parent(#descriptor, &mut this, ctx);
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
                        let _ = this.create_gate_cluster(#ident, #size, ::des::net::GateServiceType::#typ, ctx.rt());
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
                        let channel = ::des::net::Channel::new(
                            ::des::net::ObjectPath::channel_with(
                                &format!("{}->{}", #from_ident.name(), #to_ident.name()),
                                this.path()
                            ),
                            ::des::net::ChannelMetrics {
                                bitrate: #bitrate,
                                latency: ::des::time::Duration::from_secs_f64(#latency),
                                jitter: ::des::time::Duration::from_secs_f64(#jitter),
                                cost: #cost,
                            }
                        );
                        ctx.create_channel(channel.clone());
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
                    ctx.create_module(#ident);
                })
            }

            // Compile token stream

            let wrapped = WrappedTokenStream(token_stream);

            out.extend::<TokenStream>(build_impl_from(
                ident.clone(),
                wrapped,
                &module.submodules,
                proto_t_counter,
            ));

            if typalias {
                let alias = ident!(module.ident.raw());
                out.extend::<TokenStream>(
                    quote! {
                        #vis type #alias = #ident;
                    }
                    .into(),
                );
            }

            Ok(())
        }
        Err(_) => Err(Diagnostic::new(
            Level::Error,
            "#[derive(Module)] -- Failed to parse NDl file.".into(),
        )),
    }
}
