use proc_macro2::Span as Span2;
use proc_macro2::TokenStream;
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::token::Add;
use syn::*;

use crate::common::WrappedTokenStream;

pub fn derive_message_body(
    ident: Ident,
    data: Data,
    generics: Generics,
) -> crate::common::Result<TokenStream> {
    match data {
        Data::Struct(data_struct) => {
            let impl_ts = match data_struct.fields {
                Fields::Named(named_fields) => {
                    let mut ts = TokenStream::new();
                    for field in named_fields.named {
                        let ty = field.ty;
                        let fident = field.ident.unwrap();

                        ts.extend(quote! {
                            <#ty as ::des::net::message::MessageBody>::byte_len(&self.#fident) +
                        });
                    }
                    ts
                }
                Fields::Unnamed(unnamed_fields) => {
                    // Does this case ever happen
                    let mut ts = TokenStream::new();

                    for (i, field) in unnamed_fields.unnamed.into_iter().enumerate() {
                        let ty = field.ty;
                        let fident = Index::from(i);

                        ts.extend(quote! {
                            <#ty as ::des::net::message::MessageBody>::byte_len(&self.#fident) +
                        });
                    }

                    ts
                }
                Fields::Unit => TokenStream::new(),
            };

            let generics = generate_impl_generics(generics);
            let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

            let wrapped = WrappedTokenStream(impl_ts);
            Ok(quote! {
                    impl #impl_generics ::des::net::message::MessageBody for #ident #type_generics #where_clause {
                        fn byte_len(&self) -> usize {
                            #wrapped 0
                        }
                    }
                }.into()
            )
        }
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

            let generics = generate_impl_generics(generics);
            let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

            Ok(quote! {
                impl #impl_generics ::des::net::message::MessageBody for #ident #type_generics #where_clause {
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

fn generate_impl_generics(mut generics: Generics) -> Generics {
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(param) = param {
            if !param.bounds.trailing_punct() && !param.bounds.is_empty() {
                param.bounds.push_punct(Add {
                    spans: [proc_macro2::Span::call_site()],
                });
            }

            let input = quote::quote! { ::des::net::message::MessageBody };
            param
                .bounds
                .push_value(TypeParamBound::Trait(parse2(input).unwrap()))
        }
    }

    generics
}
