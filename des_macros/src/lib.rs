use des_ndl::*;
use proc_macro::{self, TokenStream};
use proc_macro2::Ident;
use quote::quote;
use quote::ToTokens;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, FieldsNamed, FieldsUnnamed, Lit, Meta,
    MetaNameValue, Type,
};

#[proc_macro_derive(Module, attributes(ndl_workspace))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);

    let mut static_gen = gen_static_module_core(ident.clone(), data);
    let dynamic_gen = gen_dynamic_module_core(ident.clone(), attrs);

    static_gen.extend(dynamic_gen.into_iter());

    static_gen
}

fn gen_static_module_core(ident: Ident, data: Data) -> TokenStream {
    let elem_ident = match data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => named
                .iter()
                .find(|f| {
                    if let Type::Path(ty) = &f.ty {
                        ty.path.segments.last().unwrap().ident
                            == Ident::new(
                                "ModuleCore",
                                ty.path.segments.last().unwrap().ident.span(),
                            )
                    } else {
                        false
                    }
                })
                .map(|field| (Some(field.ident.clone().unwrap()), 0)),
            syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => unnamed
                .iter()
                .enumerate()
                .find(|(_, f)| {
                    if let Type::Path(ty) = &f.ty {
                        ty.path.segments.last().unwrap().ident
                            == Ident::new(
                                "ModuleCore",
                                ty.path.segments.last().unwrap().ident.span(),
                            )
                    } else {
                        false
                    }
                })
                .map(|(idx, _)| (None, idx)),
            syn::Fields::Unit => None,
        },
        _ => unimplemented!(),
    };

    if let Some((eident, idx)) = elem_ident {
        if let Some(eident) = eident {
            quote! {
                impl ::des_core::StaticModuleCore for #ident {
                    fn module_core(&self) -> &::des_core::ModuleCore {
                        &self.#eident
                    }

                    fn module_core_mut(&mut self) -> &mut ::des_core::ModuleCore {
                        &mut self.#eident
                    }
                }
            }
            .into()
        } else {
            let idx = syn::Index::from(idx);
            quote! {
                impl ::des_core::StaticModuleCore for #ident {
                    fn module_core(&self) -> &::des_core::ModuleCore {
                        &self.#idx
                    }

                    fn module_core_mut(&mut self) -> &mut ::des_core::ModuleCore {
                        &mut self.#idx
                    }
                }
            }
            .into()
        }
    } else {
        panic!("#[derive(Module)] -- No assosicated module core field found.")
    }
}

fn gen_dynamic_module_core(ident: Ident, attrs: Vec<Attribute>) -> TokenStream {
    if let Some(workspace) = parse_attr(attrs) {
        let mut resolver = NdlResolver::new(&workspace)
            .expect("#[derive(Module)] -- Failed because ndl_workspace is invalid");

        match resolver.run() {
            Ok(res) => {
                let module = res
                    .modules
                    .into_iter()
                    .find(|m| ident == m.name)
                    .expect("#[derive(Module)] -- Failed to find NDL module with same name.");

                let mut token_stream = proc_macro2::TokenStream::new();

                // Submodule configuration
                // TODO

                // Gate configuration
                for gate in &module.gates {
                    let GateDef { name, size, .. } = gate;
                    token_stream.extend::<proc_macro2::TokenStream>(quote! {
                        self.create_gate_cluster(#name, #size);
                    })
                }

                // Connection configuration.

                let wrapped = WrappedTokenStream(token_stream);

                quote! {
                    impl ::des_core::DynamicModuleCore for #ident {
                        fn build(mut self: Box<Self>) -> Box<Self> {
                            #wrapped
                            self
                        }
                    }
                }
                .into()
            }
            Err(_) => panic!("#[derive(Module)] -- Failed to parse NDl file."),
        }
    } else {
        quote! {
            impl ::des_core::DynamicModuleCore for #ident {}
        }
        .into()
    }
}

fn parse_attr(attrs: Vec<Attribute>) -> Option<String> {
    for attr in attrs {
        match attr.parse_meta().unwrap() {
            Meta::NameValue(MetaNameValue {
                ref path, ref lit, ..
            }) => {
                if path.segments.last().unwrap().ident == "ndl_workspace" {
                    match lit {
                        Lit::Str(str) => return Some(str.value()),
                        _ => return None,
                    }
                }
            }
            _ => {}
        }
    }

    None
}

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
