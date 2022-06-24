use std::collections::HashMap;

use crate::{
    common::*,
    error::*,
    parser::{AliasDef, ModuleDef, ParsingResult},
    NdlResolver,
};

use super::ctx::TyDefContext;

///
/// Resolves alias definitions:
/// 
pub fn prepare(resolver: &mut NdlResolver) -> Vec<Error> {
    // Move out
    let mut errors = Vec::new();
    let mut units: HashMap<String, ParsingResult> = resolver.units.clone();

    for (_asset, unit) in &mut units {
        let aliases = unit.aliases.drain(..).collect::<Vec<_>>();

        let tyctx = TyDefContext::new_for(unit, resolver, &mut errors);
        let gtyctx = resolver.gtyctx_def();

        let mut aliases = aliases
            .into_iter()
            .map(|alias| {
                let AliasDef {
                    loc,
                    ident: name,
                    prototype,
                } = alias;

                // search for proto
                let proto = tyctx.prototype(&prototype);

                match proto {
                    Ok(proto) => {
                        let mut proto: ModuleDef = (*proto).clone();
    
                        proto.is_prototype = false;
                        proto.loc = loc;
                        proto.ident = name.cast_type(OType::Module);
    
                        proto.derived_from = Some(prototype.to_string());
    
                        Some(proto)
                    },
                    Err(e) => {
                        let g_proto = gtyctx.prototype(&prototype).map(|m| m.loc);
                        let g_module_or_proto = gtyctx.module(&prototype).map(|m| m.loc).is_some();
    
                        let module_as_proto = g_module_or_proto && g_proto.is_none();
    
                        errors.push(Error::new_ty_missing_or_lookalike(
                            DsgInvalidPrototypeAtAlias,
                            if module_as_proto {
                                format!(
                                    "No prototype called '{0}' found for alias '{1}'. Module '{0}' is no prototype.",
                                    prototype, name.raw()
                                )
                            } else {
                                format!("No prototype called '{}' found for alias '{}'.", prototype,name.raw())
                            },
                            loc,
                            &resolver.source_map,
                            g_proto,
                            e.lookalike().map(|(p, _)| (&p.ident, p.loc))
                        ));
    
                        None
                    }
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        unit.modules.append(&mut aliases);
    }

    // SWAP in
    std::mem::swap(&mut units, &mut resolver.units);
    errors
}
