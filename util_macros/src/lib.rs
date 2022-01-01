use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::*;

#[proc_macro_derive(EventSuperstructure)]
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
                impl<A: Application<EventSuperstructure = Self>> EventSuperstructure<A> for #gident
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

///
/// A macro to provude a global UID like service on a tuple struct.
///
/// DEPRECATION PLANNED
///
#[proc_macro_derive(GlobalUID)]
pub fn derive_global_uid(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let mut name = ident.to_string().to_uppercase();
    name.push_str("_STATIC");
    let sident = Ident::new(&name, ident.span());

    let output = quote! {
        static mut #sident: #ident = #ident(0xff);

        impl #ident {
            fn gen() -> Self {
                unsafe {
                    let a = #sident;
                    #sident.0 += 1;
                    a
                }
            }
        }

        impl Clone for #ident {
            fn clone(&self) -> Self {
                Self(self.0)
            }
        }

        impl Copy for #ident  {}

        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.0, f)
            }
        }

        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::cmp::PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }


        impl std::cmp::Eq for #ident {}

        impl std::hash::Hash for #ident {
            fn hash<H>(&self, state: &mut H)
                where H: std::hash::Hasher {
                    self.0.hash(state)
                }
        }
    };
    output.into()
}

struct WrappedTokenStream(proc_macro2::TokenStream);

impl ToTokens for WrappedTokenStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<proc_macro2::TokenStream>(self.0.clone())
    }
}
