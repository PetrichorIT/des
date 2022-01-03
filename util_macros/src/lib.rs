use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::*;

#[proc_macro_derive(EventSet)]
pub fn derive_event_superstucture(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let gident = ident;

    match data {
        Data::Enum(data_enum) => {
            let mut token_stream = proc_macro2::TokenStream::new();
            let mut from_stream = proc_macro2::TokenStream::new();
            let mut where_stream = proc_macro2::TokenStream::new();

            for v in data_enum.variants {
                let Variant { ident, fields, .. } = v;
                match fields {
                    Fields::Unnamed(fields) => {
                        assert!(
                            fields.unnamed.len() == 1,
                            "Expected enum variant '{}' to have one unnamed field.",
                            ident
                        );

                        // get ty
                        let ty = match &fields.unnamed[0].ty {
                            Type::Path(path) => path.path.get_ident().expect("Unspported type def"),
                            _ => panic!("Unsupported type def"),
                        };

                        assert!(
                            ident == *ty,
                            "Expected enum variant '{0}', to have on unnamed filed of type '{0}'",
                            ident
                        );

                        token_stream.extend(quote! {
                            Self::#ident(event) => event.handle(rt),
                        });

                        where_stream.extend(quote! {
                            #ty: Event<A>,
                        });

                        from_stream.extend(quote! {
                            impl std::convert::From<#ty> for #gident {
                                fn from(variant: #ty) -> Self {
                                    Self::#ty(variant)
                                }
                            }
                        })
                    }
                    _ => unimplemented!(),
                }
            }

            let token_stream = WrappedTokenStream(token_stream);
            let where_stream = WrappedTokenStream(where_stream);

            let mut final_stream = quote! {
                impl<A: Application<EventSet = Self>> EventSet<A> for #gident
                    where #where_stream {
                    fn handle(self, rt: &mut Runtime<A>) {
                        match self {
                            #token_stream
                        }
                    }
                }
            };

            final_stream.extend(from_stream);
            final_stream.into()
        }
        _ => panic!("#[derive(EventSuperstructure)] is only supported for enums."),
    }
}

struct WrappedTokenStream(proc_macro2::TokenStream);

impl ToTokens for WrappedTokenStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<proc_macro2::TokenStream>(self.0.clone())
    }
}
