use std::fmt::Display;

use crate::desugar::TyDefContext;
use crate::error::ErrorCode::*;
use crate::parser::*;
use crate::*;

///
/// Pre unit result
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FstPassResult {
    pub asset: AssetDescriptor,
    pub loc: Loc,

    pub includes: Vec<IncludeDef>,
    pub links: Vec<LinkDef>,
    pub modules: Vec<ModuleDef>,
    pub subsystems: Vec<SubsystemDef>,

    pub prototypes: Vec<ModuleDef>,

    pub errors: Vec<Error>,
}

impl Display for FstPassResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not yet implemented")
    }
}

///
/// Removes AliasDef & PrototypeDef.
///
pub fn first_pass(unit: &ParsingResult, resolver: &NdlResolver) -> FstPassResult {
    let mut errors = Vec::new();
    let tyctx = TyDefContext::new_for(unit, resolver, &mut errors);
    let gtyctx = resolver.gtyctx_def();
    // resolve aliases

    let ParsingResult {
        asset,
        loc,

        includes,
        links,
        modules,
        subsystems: networks,

        prototypes,
        aliases,
        ..
    } = unit;

    let mut modules = modules.clone();

    //
    // Since prototypes are removed check some definitions first.
    //

    // do this before aliasing and check prototypes too

    for module in modules.iter().chain(prototypes) {
        for child in &module.submodules {
            // PROTO DEF
            if let TyDef::Dynamic(ref s) = child.ty {
                // check whether s is a existing prototype.

                let exists = tyctx.prototypes.iter().any(|p| &p.name == s);
                if !exists {
                    let g_proto = gtyctx.prototype(s).map(|m| m.loc);
                    let g_module = gtyctx.module(s).map(|m| m.loc).is_some();

                    let module_as_proto = g_module && g_proto.is_none();

                    errors.push(Error::new_ty_missing(
                        DsgInvalidPrototypeAtSome,
                        if module_as_proto {
                            format!(
                                "No prototype called '{0}' found. Module '{0}' is no prototype.",
                                s
                            )
                        } else {
                            format!("No prototype called '{}' found.", s)
                        },
                        child.loc,
                        &resolver.source_map,
                        g_proto,
                    ))
                }
            }
        }
    }

    //
    // Remove aliases from ctx.
    //
    for AliasDef {
        loc,
        name,
        prototype,
    } in aliases
    {
        // search for proto
        let proto = tyctx.prototypes.iter().find(|p| p.name == *prototype);

        if let Some(proto) = proto {
            let mut proto: ModuleDef = proto.to_owned().to_owned();

            proto.loc = *loc;
            proto.name = name.clone();
            proto.derived_from = Some(prototype.to_string());

            modules.push(proto);
        } else {
            let g_proto = gtyctx.prototype(prototype).map(|m| m.loc);
            let g_module_or_proto = gtyctx.module(prototype).map(|m| m.loc).is_some();

            let module_as_proto = g_module_or_proto && g_proto.is_none();

            errors.push(Error::new_ty_missing(
                DsgInvalidPrototypeAtAlias,
                if module_as_proto {
                    format!(
                        "No prototype called '{0}' found. Module '{0}' is no prototype.",
                        prototype
                    )
                } else {
                    format!("No prototype called '{}' found.", prototype)
                },
                *loc,
                &resolver.source_map,
                g_proto,
            ));
        }
    }

    FstPassResult {
        asset: asset.clone(),
        loc: *loc,

        includes: includes.clone(),
        links: links.clone(),
        modules,
        subsystems: networks.clone(),
        prototypes: prototypes.clone(),

        errors,
    }
}
