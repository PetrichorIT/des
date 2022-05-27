use crate::common::*;
use crate::{
    attributes::Attributes,
    common::{get_resolver, ident_from_conident, WrappedTokenStream},
};
use ndl::ChannelSpec;
use ndl::ChildModuleSpec;
use ndl::ConSpec;
use proc_macro2::Ident;
use proc_macro_error::{Diagnostic, Level};
use quote::quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::punctuated::Punctuated;
use syn::token::Comma;

pub fn derive_network_impl(ident: Ident, attrs: Attributes) -> Result<TokenStream> {
    gen_network_main(ident, attrs)
}

macro_rules! ident {
    ($e:expr) => {
        proc_macro2::Ident::new(
            &$e.replace("[", "").replace("]", ""),
            proc_macro2::Span::call_site(),
        )
    };
}

fn gen_network_main(ident: Ident, attrs: Attributes) -> Result<TokenStream> {
    match get_resolver(
        &attrs
            .workspace
            .expect("#[derive(Network)] Missing NDL worspace"),
    ) {
        Ok((res, _, par_files)) => {
            let network = if let Some(ident) = attrs.ident {
                // TODO
                // Not yet possible since ndl_ident was not yet added to attributes
                // First implement mapping inside resolver.
                res.network(ident)
            } else {
                res.network(ident.clone())
            };

            if let Some(network) = network {
                let mut token_stream = TokenStream2::new();

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
                    let ident = ident!(format!("{}_child", descriptor));
                    let ty = ident!(ty.inner());

                    if let Some(ref proto) = node.proto_impl {
                        let mut p = Punctuated::<Ident, Comma>::new();
                        for (_descr, ty) in &proto.values {
                            p.push(ident!(ty));
                        }

                        token_stream.extend::<proc_macro2::TokenStream>(quote! {
                            let mut #ident: ::des::util::PtrMut<#ty> = #ty::build_named::<Self, #p>(#descriptor.parse().unwrap(), rt);
                        });
                    } else {
                        token_stream.extend::<proc_macro2::TokenStream>(quote! {
                            let mut #ident: ::des::util::PtrMut<#ty> = #ty::build_named(#descriptor.parse().unwrap(), rt);
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
                            // assert_eq!(#from_ident.len(), #to_ident.len());
                            #from_ident.set_next_gate(#to_ident);
                        });
                    }
                }
                // Add nodes to rt.

                for node in &network.nodes {
                    let ChildModuleSpec { descriptor, .. } = node;
                    let ident = ident!(format!("{}_child", descriptor));

                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        rt.create_module(#ident);
                    })
                }

                // Compile token stream

                let token_stream = WrappedTokenStream(token_stream);

                Ok(quote! {
                    impl #ident {
                        pub fn run(self) -> ::des::runtime::RuntimeResult<Self> {
                            self.run_with_options(::des::runtime::RuntimeOptions::default())
                        }

                        pub fn run_with_options(self, options: ::des::runtime::RuntimeOptions) -> ::des::runtime::RuntimeResult<Self> {
                            use ::des::runtime::Runtime;
                            use ::des::net::NetworkRuntime;

                            let net_rt = self.build_rt();
                            let rt = Runtime::<NetworkRuntime<Self>>::new_with(net_rt, options);


                            rt.run().map_app(|network_app| network_app.finish())
                        }

                        pub fn build_rt(self) -> ::des::net::NetworkRuntime<Self> {
                            let mut runtime = ::des::net::NetworkRuntime::new(self);
                            let rt: &mut ::des::net::NetworkRuntime<Self> = &mut runtime;

                            use ::des::net::*;
                            #token_stream

                            // rt.finish_building()
                            runtime
                        }
                    }
                }.into())
            } else {
                return Err(Diagnostic::new(
                    Level::Error,
                    format!(
                        "#[derive(Network)] NDL resolver failed to find network called '{}'",
                        ident,
                    ),
                ));
            }
        }
        Err(e) => Err(Diagnostic::new(
            Level::Error,
            format!("#[derive(Network)] NDL resolver failed: {}", e),
        )),
    }
}
