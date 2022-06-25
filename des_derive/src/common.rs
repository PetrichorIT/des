use lazy_static::lazy_static;
use ndl::{ChildNodeSpec, DesugaredResult};
use ndl::{ConSpecNodeIdent, NdlResolver};
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote;
use quote::ToTokens;
use syn::{Path, DataStruct, FieldsNamed, FieldsUnnamed, Data, Type, Visibility, Field, NestedMeta, Lit, AttributeArgs, TypePath};
use syn::PathArguments;
use syn::PathSegment;
use syn::TraitBound;
use syn::TraitBoundModifier;
use syn::TypeParam;
use syn::TypeParamBound;
use syn::punctuated::Punctuated;
use syn::token::{Comma, Colon2};
use syn::GenericParam;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{Diagnostic, Level};

pub type Result<T> = std::result::Result<T, Diagnostic>;

lazy_static! {
    static ref RESOLVERS: Mutex<HashMap<String, NdlResolver>> = Mutex::new(HashMap::new());
}

pub fn get_resolver(
    workspace: &str,
) -> std::result::Result<(DesugaredResult, bool, Vec<PathBuf>), &'static str> {
    let mut resolvers = RESOLVERS.lock().unwrap();

    if !resolvers.contains_key(workspace) {
        resolvers.insert(
            workspace.to_owned(),
            NdlResolver::new(workspace).expect("#[derive(Module)] Invalid NDL workspace."),
        );
    }
    let (t, errs, p) = {
        let resolver = resolvers
            .get_mut(workspace)
            .unwrap();
        

        let result = resolver
            .run_cached()
            .map(|(gtyctx, errs, pars)| {
                (
                    gtyctx.to_owned(),
                    errs.cloned().collect::<Vec<ndl::Error>>(),
                    pars,
                )
        })?;
        
        // Parse successful from a access persepective
        setup_path_tracking(&resolver.scopes);
        result
    };
        

    let has_errs = !errs.is_empty();

    Ok((t, has_errs, p))
}

pub fn setup_path_tracking(paths: &[PathBuf]) {
    for path in paths {
        proc_macro::tracked_path::path(path.to_string_lossy())
    }
   
}

macro_rules! ident {
    ($e:expr) => {
        proc_macro2::Ident::new(
            &$e.replace("[", "").replace("]", ""),
            proc_macro2::Span::call_site(),
        )
    };
}



pub struct WrappedTokenStream(pub TokenStream2);

impl ToTokens for WrappedTokenStream {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<proc_macro2::TokenStream>(self.0.clone())
    }
}

//

/// Resolve a concreate conident to the associated gate clusters
pub fn ident_from_conident(
    token_stream: &mut proc_macro2::TokenStream,
    ident: &ConSpecNodeIdent,
) -> Ident {
    match ident {
        ConSpecNodeIdent::Child {
            child_ident,
            gate_ident,
            pos,
            ..
        } => {
            let submodule_ident = ident!(format!("{}_child", child_ident));
            let ident_token = ident!(format!("{}_child_{}_gate{}", child_ident, gate_ident, pos));

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident_token: ::des::net::GateRefMut = #submodule_ident.gate_mut(#gate_ident, #pos)
                    .expect(&format!("Internal macro err. Could not find child gate '{}[{}]'.", #gate_ident, #pos)).clone();
            });

            ident_token
        }
        ConSpecNodeIdent::Local {
            gate_ident, pos, ..
        } => {
            let ident = ident!(format!("{}_gate{}_ref", gate_ident, pos));

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident: ::des::net::GateRefMut = this.gate_mut(#gate_ident, #pos)
                    .expect(&format!("Internal macro err. Could not find local gate '{}[{}]'.", #gate_ident, #pos)).clone();
            });

            ident
        }
    }
}

type TokenStream = proc_macro::TokenStream;

pub fn build_impl_from(ident: Ident, wrapped: WrappedTokenStream, submodules: &[ChildNodeSpec], proto_t_counter: usize) -> TokenStream {
    
    let param_a = GenericParam::Type(TypeParam {
        attrs: Vec::new(),
        ident: Ident::new("A", Span::call_site()),
        colon_token: None,
        bounds: Punctuated::new(),
        default: None,
        eq_token: None,
    });
   
    let mut puntuated_a: Punctuated<GenericParam, Comma> = Punctuated::new();   
    puntuated_a.push(param_a);

    for ty in submodules.iter().filter_map(|c| 
        if c.ty.is_dynamic() {
            Some(ident!(format!("T{}", c.descriptor)))
        } else { 
            None 
        }) {

    // }
    // for ty in  (0..proto_t_counter).map(|i| ident!(format!("T{}", i))) {

        let mut segments = Punctuated::new();
        segments.push(PathSegment { ident: ident!("des"), arguments: PathArguments::None });
        segments.push(PathSegment { ident: ident!("net"), arguments: PathArguments::None });
        segments.push(PathSegment { ident: ident!("NameableModule"), arguments: PathArguments::None });

        let mut segments_2 = Punctuated::new();
        segments_2.push(PathSegment { ident: ident!("des"), arguments: PathArguments::None });
        segments_2.push(PathSegment { ident: ident!("net"), arguments: PathArguments::None });
        segments_2.push(PathSegment { ident: ident!("__Buildable0"), arguments: PathArguments::None });

        let mut segments_3 = Punctuated::new();
        segments_3.push(PathSegment { ident: ident!("des"), arguments: PathArguments::None });
        segments_3.push(PathSegment { ident: ident!("net"), arguments: PathArguments::None });
        segments_3.push(PathSegment { ident: ident!("Module"), arguments: PathArguments::None });

        let mut bounds = Punctuated::new();
        bounds.push(TypeParamBound::Trait(TraitBound {
            paren_token: None,
            modifier: TraitBoundModifier::None,
            lifetimes: None,
            path: Path {
                leading_colon: None,
                segments,
            },
        }));
        bounds.push(TypeParamBound::Trait(TraitBound {
            paren_token: None,
            modifier: TraitBoundModifier::None,
            lifetimes: None,
            path: Path {
                leading_colon: None,
                segments: segments_2,
            },
        }));
        bounds.push(TypeParamBound::Trait(TraitBound {
            paren_token: None,
            modifier: TraitBoundModifier::None,
            lifetimes: None,
            path: Path {
                leading_colon: None,
                segments: segments_3,
            },
        }));

        let param = GenericParam::Type(TypeParam {
            ident: ty,
            attrs: Vec::new(),
            colon_token: None,
            default: None,
            eq_token: None,
            bounds,
        });

        puntuated_a.push(param.clone());
     
    }

    let build_trait = ident!(format!("__Buildable{}", proto_t_counter));


    quote! {
        impl ::des::net::#build_trait for #ident {
            fn build<#puntuated_a>(mut this: ::des::util::PtrMut<Self>, rt: &mut ::des::net::NetworkRuntime<A>) 
            -> ::des::util::PtrMut<Self> {
                use des::net::*;
                
                #wrapped
                this
            }
        }
    }
    .into()
}


pub fn gen_named_object(ident: Ident, data: &DataStruct) -> Result<TokenStream2> {

    match &data.fields {
        syn::Fields::Named(FieldsNamed { named, .. }) => {
            if named.len() == 1 {
                let field = named.first().unwrap().ident.clone().unwrap();
                Ok(quote! {
                    impl ::des::net::NameableModule for #ident {
                        fn named(core: ::des::net::ModuleCore) -> Self {
                            Self { #field: core }
                        }
                    }
                })
            } else {
                Ok(TokenStream2::new())
            }
        }
        syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            if unnamed.len() == 1 {
                Ok(quote! {
                    impl ::des::net::NameableModule for #ident {
                        fn named(core: ::des::net::ModuleCore) -> Self {
                            Self(core)
                        }
                    }
                })
            } else {
                Ok(TokenStream2::new())
            }
        }
        _ => Ok(TokenStream2::new()),
    }
}

pub fn generate_deref_impl(
    ident: Ident,
    data: Data,
    searched_ty: &str,
    out: &mut TokenStream,
) -> Result<()> {
    let token_stream: TokenStream2;

    let elem_ident = match &data {
        syn::Data::Struct(s) => {
            token_stream = gen_named_object(ident.clone(), s)?;

            match &s.fields {
                syn::Fields::Named(FieldsNamed { named, .. }) => named
                    .iter()
                    .find(|f| {
                        if let Type::Path(ty) = &f.ty {
                            ty.path.segments.last().unwrap().ident
                                == Ident::new(
                                    searched_ty,
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
                                    searched_ty,
                                    ty.path.segments.last().unwrap().ident.span(),
                                )
                        } else {
                            false
                        }
                    })
                    .map(|(idx, _)| (None, idx)),
                syn::Fields::Unit => None,
            }
        }
        _ => {
            return Err(Diagnostic::new(
                Level::Error,
                "Modules are currently only supported on structs.".into(),
            ))
        }
    };

    if let Some((eident, idx)) = elem_ident {
        out.extend::<TokenStream>(token_stream.into());

        if let Some(eident) = eident {
            out.extend::<TokenStream>(
                quote! {
                    impl ::std::ops::Deref for #ident {
                        type Target = ::des::net::ModuleCore;
                        fn deref(&self) -> &Self::Target {
                            &self.#eident
                        }
                    }
                    impl ::std::ops::DerefMut for #ident {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.#eident
                        }
                    }
                }
                .into(),
            );
        } else {
            let idx = syn::Index::from(idx);
            out.extend::<TokenStream>(
                quote! {
                    impl ::std::ops::Deref for #ident {
                        type Target = ::des::net::ModuleCore;
                        fn deref(&self) -> &Self::Target {
                            &self.#idx
                        }
                    }
                    impl ::std::ops::DerefMut for #ident {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.#idx
                        }
                    }
                }
                .into(),
            );
        }
    } else {
        return Err(Diagnostic::new(
            Level::Error,
            "Failed to find a field containing a module core.".to_string(),
        )
        .help(String::from("Try adding a module core to the struct.")));
    }

    Ok(())
}

pub fn resolve_attrs(attrs: AttributeArgs) -> Result<String> {
    let workspace = match attrs.first() {
        Some(v) => v,
        None => {
            return Err(Diagnostic::new(
                Level::Error,
                "Missing attribute 'workspace' at macro invokation.".to_string(),
            ))
        }
    };
    let workspace = match workspace {
        NestedMeta::Lit(lit) => {
            if let Lit::Str(lit) = lit {
                lit.value()
            } else {
                return Err(Diagnostic::new(
                    Level::Error,
                    "Missing attribute 'workspace' at macro invokation. Must be a string."
                        .to_string(),
                ));
            }
        }
        NestedMeta::Meta(_) => unimplemented!(),
    };

    Ok(workspace)
}

// macro_rules! typath {
//     ($($ty:ident)::+) => {
//         {
//             let mut segments = Punctuated::new();
//             $(
//                 segments.push(PathSegment {
//                     ident: Ident::new(stringify!($ty), Span::call_site()),
//                     arguments: PathArguments::None,
//                 });
//             )+

//             Type::Path(TypePath {
//                 qself: None,
//                 path: Path {
//                     leading_colon: Some(Colon2::default()),
//                     segments,
//                 },
//             })
//         }
//     }
// }

pub fn derive_deref(
    ident: Ident,
    data: &mut DataStruct,
    out: &mut TokenStream,
    searched_ty: &str,
) -> Result<()> {
    let token_stream: TokenStream2;
    let ty_path = {
        let mut segments = Punctuated::new();
        segments.push(PathSegment {
            ident: Ident::new("des", Span::call_site()),
            arguments: PathArguments::None,
        });
        segments.push(PathSegment {
            ident: Ident::new("net", Span::call_site()),
            arguments: PathArguments::None,
        });
        segments.push(PathSegment {
            ident: Ident::new(searched_ty, Span::call_site()),
            arguments: PathArguments::None,
        });

        Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: Some(Colon2::default()),
                segments,
            },
        })
    };

    let elem_ident = {
        

        match &data.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => named
                .iter()
                .find(|f| {
                    if let Type::Path(ty) = &f.ty {
                        ty.path.segments.last().unwrap().ident
                            == Ident::new(
                                searched_ty,
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
                                searched_ty,
                                ty.path.segments.last().unwrap().ident.span(),
                            )
                    } else {
                        false
                    }
                })
                .map(|(idx, _)| (None, idx)),
            syn::Fields::Unit => None,
        }
    };

    let (eident, idx) = match elem_ident {
        Some(v) => v,
        None => match &mut data.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                // build sgements

                named.push(Field {
                    attrs: Vec::new(),
                    vis: Visibility::Inherited,
                    ident: Some(Ident::new("__core", Span::call_site())),
                    colon_token: None,
                    ty: ty_path.clone(),
                });

                (Some(Ident::new("__core", Span::call_site())), 0)
            }
            _ => todo!(),
        },
    };

    if searched_ty == "ModuleCore" { 
        token_stream = gen_named_object(ident.clone(), &data)?; 
        out.extend::<TokenStream>(token_stream.into());
    }
    

    if let Some(eident) = eident {
        out.extend::<TokenStream>(
            quote! {
                impl ::std::ops::Deref for #ident {
                    type Target = #ty_path;
                    fn deref(&self) -> &Self::Target {
                        &self.#eident
                    }
                }
                impl ::std::ops::DerefMut for #ident {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#eident
                    }
                }
            }
            .into(),
        );
    } else {
        let idx = syn::Index::from(idx);
        out.extend::<TokenStream>(
            quote! {
                impl ::std::ops::Deref for #ident {
                    type Target = #ty_path;
                    fn deref(&self) -> &Self::Target {
                        &self.#idx
                    }
                }
                impl ::std::ops::DerefMut for #ident {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#idx
                    }
                }
            }
            .into(),
        );
    }

    Ok(())
}
