use proc_macro2::Span as Span2;
use proc_macro2::TokenStream;
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use quote::ToTokens;
use syn::token::Plus;
use syn::{parse2, Data, Fields, GenericParam, Generics, Ident, Index, TypeParamBound};

type Result<T> = std::result::Result<T, Diagnostic>;

struct WrappedTokenStream(TokenStream);

impl ToTokens for WrappedTokenStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<proc_macro2::TokenStream>(self.0.clone());
    }
}

/// Returns the derived token stream.
///
/// # Errors
///
/// Internal.
///
/// # Panics
///
/// Internal.
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn derive_impl(ident: Ident, data: Data, generics: Generics) -> Result<TokenStream> {
    match data {
        Data::Struct(data_struct) => {
            let impl_ts = match data_struct.fields {
                Fields::Named(named_fields) => {
                    let mut ts = TokenStream::new();
                    for field in named_fields.named {
                        let ty = field.ty;
                        let field_ident = field.ident.unwrap();

                        ts.extend(quote! {
                            <#ty as ::des::net::message::MessageBody>::byte_len(&self.#field_ident) +
                        });
                    }
                    ts
                }
                Fields::Unnamed(unnamed_fields) => {
                    // Does this case ever happen
                    let mut ts = TokenStream::new();

                    for (i, field) in unnamed_fields.unnamed.into_iter().enumerate() {
                        let ty = field.ty;
                        let field_ident = Index::from(i);

                        ts.extend(quote! {
                            <#ty as ::des::net::message::MessageBody>::byte_len(&self.#field_ident) +
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
            })
        }
        Data::Enum(data_enum) => {
            let mut gts = TokenStream::new();
            if data_enum.variants.is_empty() {
                return Ok(quote! {
                    impl ::des::net::message::MessageBody for #ident {
                        fn byte_len(&self) -> usize { 0 }
                    }
                });
            }

            for variant in data_enum.variants {
                let variant_ident = variant.ident;

                let ts = match variant.fields {
                    Fields::Named(named_fields) => {
                        let mut prop_ts = TokenStream::new();
                        let mut ts = TokenStream::new();

                        for field in named_fields.named {
                            let ty = field.ty;
                            let field_ident = field.ident.unwrap();

                            prop_ts.extend(quote! { ref #field_ident, });
                            ts.extend(quote! {
                                <#ty as ::des::net::message::MessageBody>::byte_len(#field_ident) +
                            });
                        }

                        let wrapped = WrappedTokenStream(ts);
                        quote! {
                            #ident::#variant_ident { #prop_ts } => #wrapped 0
                        }
                    }
                    Fields::Unnamed(unnamed_fields) => {
                        // Does this case ever happen
                        let mut property_ts = TokenStream::new();
                        let mut ts = TokenStream::new();

                        for (i, field) in unnamed_fields.unnamed.into_iter().enumerate() {
                            let ty = field.ty;
                            let field_ident = Ident::new(&format!("v{i}"), Span2::call_site());

                            property_ts.extend(quote! { #field_ident,  });
                            ts.extend(quote! {
                                <#ty as ::des::net::message::MessageBody>::byte_len(#field_ident) +
                            });
                        }

                        let wrapped = WrappedTokenStream(ts);
                        quote! { #ident::#variant_ident(#property_ts) => #wrapped 0 }
                    }
                    Fields::Unit => {
                        quote! {
                            #ident::#variant_ident => 0
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
            })
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
                param.bounds.push_punct(Plus {
                    spans: [proc_macro2::Span::call_site()],
                });
            }

            let input = quote::quote! { ::des::net::message::MessageBody };
            param
                .bounds
                .push_value(TypeParamBound::Trait(parse2(input).unwrap()));
        }
    }

    generics
}
