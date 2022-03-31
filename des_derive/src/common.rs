use lazy_static::lazy_static;
use ndl::ChildModuleSpec;
use ndl::{ConSpecNodeIdent, NdlResolver, OwnedTySpecContext};
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote;
use quote::ToTokens;
use syn::Path;
use syn::PathArguments;
use syn::PathSegment;
use syn::TraitBound;
use syn::TraitBoundModifier;
use syn::TypeParam;
use syn::TypeParamBound;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::GenericParam;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::Diagnostic;

pub type Result<T> = std::result::Result<T, Diagnostic>;

lazy_static! {
    static ref RESOLVERS: Mutex<HashMap<String, NdlResolver>> = Mutex::new(HashMap::new());
}

pub fn get_resolver(
    workspace: &str,
) -> std::result::Result<(OwnedTySpecContext, bool, Vec<PathBuf>), &'static str> {
    let mut resolvers = RESOLVERS.lock().unwrap();

    if !resolvers.contains_key(workspace) {
        resolvers.insert(
            workspace.to_owned(),
            NdlResolver::new(workspace).expect("#[derive(Module)] Invalid NDL workspace."),
        );
    }
    let (t, errs, p) =
        resolvers
            .get_mut(workspace)
            .unwrap()
            .run_cached()
            .map(|(gtyctx, errs, pars)| {
                (
                    gtyctx.to_owned(),
                    errs.cloned().collect::<Vec<ndl::Error>>(),
                    pars,
                )
            })?;

    let has_errs = !errs.is_empty();

    Ok((t, has_errs, p))
}

//

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

pub fn build_impl_from(ident: Ident, wrapped: WrappedTokenStream, submodules: &[ChildModuleSpec], proto_t_counter: usize) -> TokenStream {
    
    let param_a = GenericParam::Type(TypeParam {
        attrs: Vec::new(),
        ident: Ident::new("A", Span::call_site()),
        colon_token: None,
        bounds: Punctuated::new(),
        default: None,
        eq_token: None,
    });
   
    let mut puntuated_a: Punctuated<GenericParam, Comma> = Punctuated::new();   
    puntuated_a.push(param_a.clone());

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
            fn build<#puntuated_a>(mut this: ::des::util::Mrc<Self>, rt: &mut ::des::net::NetworkRuntime<A>) 
            -> ::des::util::Mrc<Self> {
                #wrapped
                this
            }
        }
    }
    .into()
}

// fn build_named<A>(path: ModulePath, rt: &mut NetworkRuntime<A>) -> MrcS<Self, Mutable>
//     where
//         Self: NameableModule + Sized,
//     {
//         let core = ModuleCore::new_with(path, rt.globals());
//         let mut this = MrcS::new(Self::named(core));

//         // Attach self to module core
//         let clone = MrcS::clone(&this);
//         this.deref_mut().self_ref = Some(UntypedMrc::new(clone));

//         Self::build(this, rt)
//     }

//     fn build_named_with_parent<A, T>(
//         name: &str,
//         parent: &mut MrcS<T, Mutable>,
//         rt: &mut NetworkRuntime<A>,
//     ) -> MrcS<Self, Mutable>
//     where
//         T: NameableModule,
//         Self: NameableModule + Sized,
//     {
//         let obj = Self::named_with_parent(name, parent);
//         // parent.add_child(&mut (*obj));
//         Self::build(obj, rt)
//     }
