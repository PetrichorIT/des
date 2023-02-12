use std::path::PathBuf;

use crate::attributes::Attr;
use crate::common::*;
use ndl::ChannelSpec;
use ndl::ChildNodeSpec;
use ndl::ConSpec;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Visibility;
use syn::{AttributeArgs, DeriveInput};

pub fn derive_impl(
    input: DeriveInput,
    attrs: AttributeArgs,
    path_tracker: fn(&[PathBuf]),
) -> Result<TokenStream> {
    let attr = Attr::from_args(attrs)?;
    let ident = input.ident.clone();

    // (0) Prepare token streams
    let mut derive_stream = TokenStream::new();

    // (1) Cast to data struct, This macro can only be applied to struct.
    // let data = match &mut input.data {
    //     Data::Struct(data) => data,
    //     _ => {
    //         return Err(Diagnostic::new(
    //             Level::Error,
    //             "Failed to find a field containing a module core.".to_string(),
    //         )
    //         .help(String::from("Try adding a module core to the struct.")))
    //     }
    // };

    // (2) Derive the deref impls / generate that approiated changes in the data struct.
    // derive_deref(ident.clone(), data, &mut derive_stream, "SubsystemCore")?;
    subsystem_main(
        input.vis.clone(),
        ident,
        attr,
        &mut derive_stream,
        path_tracker,
    )?;

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

fn subsystem_main(
    vis: Visibility,
    ident: Ident,
    attr: Attr,
    out: &mut TokenStream,
    path_tracker: fn(&[PathBuf]),
) -> Result<()> {
    let workspace = match &attr.workspace {
        Some(ref w) => w,
        None => return Ok(()),
    };
    match get_resolver(workspace, path_tracker) {
        Ok((res, _, par_files)) => {
            let (network, tyalias) = if let Some(ident) = attr.overwrite_ident {
                // TODO
                // Not yet possible since ndl_ident was not yet added to attributes
                // First implement mapping inside resolver.
                (res.subsystem(&ident), true)
            } else {
                (res.subsystem(&ident.to_string()), false)
            };

            if let Some(network) = network {
                let mut token_stream = TokenStream::new();

                // Import parameters

                for par_file in par_files {
                    let string_literal = par_file.to_str().unwrap();
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        ctx.include_par_file(#string_literal);
                    })
                }

                // Config nodes.

                for node in &network.nodes {
                    let ChildNodeSpec { descriptor, ty, .. } = node;
                    let ident = ident!(format!("{descriptor}_child"));
                    let ty = ident!(ty.unwrap());

                    if let Some(ref proto) = node.proto_impl {
                        let mut p = Punctuated::<Ident, Comma>::new();
                        for (_descr, ty) in &proto.values {
                            p.push(ident!(ty));
                        }

                        token_stream.extend::<proc_macro2::TokenStream>(quote! {
                            let mut #ident: ::des::net::module::ModuleRef = #ty::build_named::<SubsystemRef, #p>(#descriptor.parse().unwrap(), ctx);
                        });
                    } else {
                        token_stream.extend::<proc_macro2::TokenStream>(quote! {
                            let mut #ident: ::des::net::module::ModuleRef = #ty::build_named(#descriptor.parse().unwrap(), ctx);
                        })
                    }
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
                            cost,
                            queuesize,
                            ..
                        } = channel;

                        token_stream.extend(quote! {
                            let channel = ::des::net::channel::Channel::new(
                                ::des::net::ObjectPath::channel_with(
                                    &format!("{}->{}", #from_ident.name(), #to_ident.name()),
                                    &this_path
                                ),
                                ::des::net::channel::ChannelMetrics {
                                    bitrate: #bitrate,
                                    latency: ::des::time::Duration::from_secs_f64(#latency),
                                    jitter: ::des::time::Duration::from_secs_f64(#jitter),
                                    cost: #cost,
                                    queuesize: #queuesize,
                                }
                            );
                            #from_ident.set_next_gate(#to_ident);
                            #from_ident.set_channel(channel);
                        });
                    } else {
                        token_stream.extend(quote! {
                            // assert_eq!(#from_ident.len(), #to_ident.len());
                            #from_ident.set_next_gate(#to_ident);
                        });
                    }
                }
                // Add nodes to rt.

                for node in &network.nodes {
                    let ChildNodeSpec { descriptor, .. } = node;
                    let ident = ident!(format!("{descriptor}_child"));

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        ctx.create_module(#ident);
                    })
                }

                // Compile token stream

                let token_stream = WrappedTokenStream(token_stream);

                let ts = quote! {
                    impl #ident {
                        pub fn run(self) -> ::des::runtime::RuntimeResult<::des::net::subsystem::SubsystemRef> {
                            self.run_with_options(::des::runtime::RuntimeOptions::default())
                        }

                        pub fn run_with_options(self, options: ::des::runtime::RuntimeOptions) -> ::des::runtime::RuntimeResult<::des::net::subsystem::SubsystemRef> {
                            use ::des::runtime::Runtime;
                            use ::des::net::NetworkRuntime;

                            let net_rt = self.build_rt();
                            let rt = Runtime::new_with(net_rt, options);


                            rt.run().map_app(|network_app| network_app.finish())
                        }

                        pub fn build_rt(self) -> ::des::net::NetworkRuntime<::des::net::subsystem::SubsystemRef> {
                            let this = self;
                            let this_ref = ::des::net::subsystem::SubsystemRef::main(this);
                            let this_path = this_ref.path();

                            let mut runtime = ::des::net::NetworkRuntime::new(this_ref.clone());
                            let mut builder = ::des::net::BuildContext::new(&mut runtime);
                            builder.push_subsystem(this_ref.clone());
                            let ctx: &mut ::des::net::BuildContext<'_, SubsystemRef> = &mut builder;


                            use ::des::net::*;
                            #token_stream

                            // rt.finish_building()
                            runtime
                        }
                    }
                };

                out.extend::<TokenStream>(ts.into());
                if tyalias {
                    let alias = ident!(network.ident.raw());
                    out.extend::<TokenStream>(
                        quote! {
                            #vis type #alias = #ident;
                        }
                        .into(),
                    );
                }

                Ok(())
            } else {
                Err(Diagnostic::new(
                    Level::Error,
                    format!(
                        "#[derive(Network)] NDL resolver failed to find network called '{ident}'"
                    ),
                ))
            }
        }
        Err(e) => Err(Diagnostic::new(
            Level::Error,
            format!("#[derive(Network)] NDL resolver failed: {e}"),
        )),
    }
}