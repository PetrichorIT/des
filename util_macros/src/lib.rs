use proc_macro::TokenStream;
use quote::quote;
use syn::*;

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
