use proc_macro2::Span as Span2;
use proc_macro2::TokenStream;
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::*;

use crate::common::WrappedTokenStream;

pub fn derive_message_body(ident: Ident, data: Data) -> crate::common::Result<TokenStream> {
    match data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(named_fields) => {
                let mut ts = TokenStream::new();
                for field in named_fields.named {
                    let ty = field.ty;
                    let fident = field.ident.unwrap();

                    ts.extend(quote! {
                        <#ty as ::des::net::message::MessageBody>::byte_len(&self.#fident) +
                    });
                }

                let wrapped = WrappedTokenStream(ts);
                Ok(quote! {
                    impl ::des::net::message::MessageBody for #ident {
                        fn byte_len(&self) -> usize {
                            #wrapped 0
                        }
                    }
                }
                .into())
            }
            Fields::Unnamed(unnamed_fields) => {
                // Does this case ever happen
                let mut ts = TokenStream::new();

                for (i, field) in unnamed_fields.unnamed.into_iter().enumerate() {
                    let ty = field.ty;
                    let fident = Index::from(i);

                    ts.extend(quote! {
                        <#ty as des::net::message::MessageBody>::byte_len(&self.#fident) +
                    });
                }

                let wrapped = WrappedTokenStream(ts);
                Ok(quote! {
                    impl des::net::message::MessageBody for #ident {
                        fn byte_len(&self) -> usize {
                            #wrapped 0
                        }
                    }
                }
                .into())
            }
            Fields::Unit => Ok(quote! {
                impl ::des::net::message::MessageBody for #ident {
                    fn byte_len(&self) -> usize { 0 }
                }
            }
            .into()),
        },
        Data::Enum(data_enum) => {
            let mut gts = TokenStream::new();
            if data_enum.variants.is_empty() {
                return Ok(quote! {
                    impl ::des::net::message::MessageBody for #ident {
                        fn byte_len(&self) -> usize { 0 }
                    }
                }
                .into());
            }

            for variant in data_enum.variants {
                let vident = variant.ident;

                let ts = match variant.fields {
                    Fields::Named(named_fields) => {
                        let mut pts = TokenStream::new();
                        let mut ts = TokenStream::new();

                        for field in named_fields.named {
                            let ty = field.ty;
                            let fident = field.ident.unwrap();

                            pts.extend(quote! { ref #fident, });
                            ts.extend(quote! {
                                <#ty as ::des::net::message::MessageBody>::byte_len(#fident) +
                            });
                        }

                        let wrapped = WrappedTokenStream(ts);
                        quote! {
                            #ident::#vident { #pts } => #wrapped 0
                        }
                    }
                    Fields::Unnamed(unnamed_fields) => {
                        // Does this case ever happen
                        let mut pts = TokenStream::new();
                        let mut ts = TokenStream::new();

                        for (i, field) in unnamed_fields.unnamed.into_iter().enumerate() {
                            let ty = field.ty;
                            let fident = Ident::new(&format!("v{i}"), Span2::call_site());

                            pts.extend(quote! { #fident,  });
                            ts.extend(quote! {
                                <#ty as ::des::net::message::MessageBody>::byte_len(#fident) +
                            });
                        }

                        let wrapped = WrappedTokenStream(ts);
                        quote! { #ident::#vident(#pts) => #wrapped 0 }
                    }
                    Fields::Unit => {
                        quote! {
                            #ident::#vident => 0
                        }
                    }
                };

                gts.extend(quote! {
                    #ts,
                });
            }

            Ok(quote! {
                impl ::des::net::message::MessageBody for #ident {
                    fn byte_len(&self) -> usize {
                        match self {
                            #gts
                        }
                    }
                }
            }
            .into())
        }
        Data::Union(_) => Err(Diagnostic::new(
            Level::Error,
            "#[derive(MessageBody)] -- Macro does not support unions".into(),
        )),
    }
}
