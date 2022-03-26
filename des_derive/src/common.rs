use lazy_static::lazy_static;
use ndl::{ConSpecNodeIdent, NdlResolver, OwnedTySpecContext};
use proc_macro2::Ident;
use quote::quote;
use quote::ToTokens;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use proc_macro2::TokenStream as TokenStream2;

lazy_static! {
    static ref RESOLVERS: Mutex<HashMap<String, NdlResolver>> = Mutex::new(HashMap::new());
}

pub fn get_resolver(
    workspace: &str,
) -> Result<(OwnedTySpecContext, bool, Vec<PathBuf>), &'static str> {
    let mut resolvers = RESOLVERS.lock().unwrap();

    if !resolvers.contains_key(workspace) {
        resolvers.insert(
            workspace.to_owned(),
            NdlResolver::new(workspace).expect("#[derive(Module)] Invalid NDL workspace."),
        );
    }
    resolvers
        .get_mut(workspace)
        .unwrap()
        .run_cached()
        .map(|(gtyctx, has_err, pars)| (gtyctx.to_owned(), has_err, pars))
}

//

macro_rules! ident {
    ($e:expr) => {
        proc_macro2::Ident::new(
            &$e.as_str().replace("[", "").replace("]", ""),
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
                let mut #ident_token: ::des::net::GateRef = #submodule_ident.gate_mut(#gate_ident, #pos)
                    .expect("Internal macro err.").clone();
            });

            ident_token
        }
        ConSpecNodeIdent::Local {
            gate_ident, pos, ..
        } => {
            let ident = ident!(format!("{}_gate{}_ref", gate_ident, pos));

            token_stream.extend::<proc_macro2::TokenStream>(quote! {
                let mut #ident: ::des::net::GateRef = this.gate_mut(#gate_ident, #pos)
                    .expect("Internal macro err.").clone();
            });

            ident
        }
    }
}
