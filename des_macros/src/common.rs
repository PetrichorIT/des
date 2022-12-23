use lazy_static::lazy_static;
use ndl::{ChildNodeSpec, DesugaredResult};
use ndl::{ConSpecNodeIdent, NdlResolver};
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote;
use quote::ToTokens;
use syn::*;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{Diagnostic};

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
                    gtyctx,
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
            let submodule_ident = ident!(format!("{child_ident}_child"));
            let ident_token = ident!(format!("{child_ident}_child_{gate_ident}_gate{pos}"));

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident_token: ::des::net::gate::GateRef = #submodule_ident.gate(#gate_ident, #pos)
                    .expect(&format!("Internal macro err. Could not find child gate '{}[{}]'.", #gate_ident, #pos)).clone();
            });

            ident_token
        }
        ConSpecNodeIdent::Local {
            gate_ident, pos, ..
        } => {
            let ident = ident!(format!("{gate_ident}_gate{pos}_ref"));

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident: ::des::net::gate::GateRef = this.gate(#gate_ident, #pos)
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


        let mut segments_2 = Punctuated::new();
        segments_2.push(PathSegment { ident: ident!("des"), arguments: PathArguments::None });
        segments_2.push(PathSegment { ident: ident!("net"), arguments: PathArguments::None });
        segments_2.push(PathSegment { ident: ident!("__Buildable0"), arguments: PathArguments::None });

        let mut segments_3 = Punctuated::new();
        segments_3.push(PathSegment { ident: ident!("des"), arguments: PathArguments::None });
        segments_3.push(PathSegment { ident: ident!("net"), arguments: PathArguments::None });
        segments_3.push(PathSegment { ident: ident!("module"), arguments: PathArguments::None });
        segments_3.push(PathSegment { ident: ident!("Module"), arguments: PathArguments::None });

        let mut bounds = Punctuated::new();
      
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

    let build_trait = ident!(format!("__Buildable{proto_t_counter}"));


    quote! {
        impl ::des::net::#build_trait for #ident {
            fn build<#puntuated_a>(mut this: ::des::net::module::ModuleRef, ctx: &mut ::des::net::BuildContext<'_, A>) 
            {
                use des::net::*;
                use des::net::module::*;
                
                #wrapped
            }
        }
    }
    .into()
}






