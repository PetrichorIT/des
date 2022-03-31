use crate::*;

use crate::desugar::{TyDefContext, GlobalTyDefContext, DesugaredParsingResult};
use crate::error::*;
use crate::parser::ChildModuleDef;

mod tyctx;

pub use tyctx::*;

const PAR_TYPES: [&str; 15] = [
    "usize", "u8", "u16", "u32", "u64", "u128", "isize", "i8", "i16", "i32", "i64", "i128", "bool",
    "char", "String",
];

pub fn validate_module_ty(def: &ChildModuleDef, tyctx: &TyDefContext<'_>, gtyctx: &GlobalTyDefContext, smap: &SourceMap, errors: &mut Vec<Error>) {
    if !tyctx.modules_and_prototypes.iter().any(|m| m.name == def.ty.inner()) {
        // Ty missing 
        let global_ty = gtyctx.module(&def.ty.inner()).map(|m| m.loc);
        errors.push(Error::new_ty_missing(
            TycNetworkSubmoduleInvalidTy,
            format!(
                "No module with name '{}' exists in the scope.",
                def.ty, 
            ),
            def.loc,
            smap,
            global_ty,
        ));
    }
}

///
/// Validates the given an internal DesugaredParsingResult 'unit' using the resovler
/// as parameters.
/// Returns all sematic errors that were encountered.
///
pub fn validate(
    unit: &DesugaredParsingResult,
    resolver: &NdlResolver,
) -> Vec<Error> {

    let mut tyctx = TySpecContext::new();
    let mut errors = Vec::new();

    resolve_includes(resolver, unit, &mut tyctx, &mut errors);

    let name_collision = tyctx.check_name_collision();


    match name_collision {
        Ok(()) => {
            //
            // === Module check ===
            //
            //            

            let mut module_names = Vec::with_capacity(unit.modules.len());

            for module in &unit.modules {
                let self_ty = &module.ident;

                if module_names.contains(&self_ty) {
                    errors.push(Error::new(
                        TycModuleAllreadyDefined,
                        format!("Module '{}' was allready defined.", self_ty),
                        module.loc,
                        false,
                    
                    ))
                } else {
                    module_names.push(self_ty)
                }

                //
                // === Submodule check
                // 

                for submodule in &module.submodules {
                    if submodule.ty.inner() == *self_ty {
                        errors.push(Error::new(
                            TycModuleSubmoduleRecrusiveTyDefinition,
                            format!("Module '{0}' has a required submodule of type '{0}'. Cannot create cyclic definitions.", submodule.ty),
                            submodule.loc,
                            false,
                      
                        ))
                    } 
                }

                //
                // === Gate check ===
                //

                let mut self_gates = Vec::with_capacity(module.gates.len());
                for gate in &module.gates {
                    if gate.size == 0 {
                        errors.push(Error::new(
                            TycGateInvalidNullGate,
                            String::from("Cannot create gate of size 0."),
                            gate.loc,
                            false,
                        ))
                        // Still hold the descriptor to prevent transient errors
                    }

                    if self_gates.iter().any(|&n| n == &gate.ident) {
                        errors.push(Error::new(
                            TycGateFieldDuplication,
                            format!("Gate '{}' was allready defined.", gate.ident),
                            gate.loc,
                            false,
                     
                        ))
                    } else {
                        self_gates.push(&gate.ident);
                    }
                }

                //
                // === Par check ===
                //

                let mut par_names = Vec::with_capacity(module.params.len());

                for par in &module.params {
                    // Check ty
                    if !PAR_TYPES.contains(&&par.ty[..]) {
                        errors.push(Error::new(
                            TycParInvalidType,
                            format!("Parameter type '{}' does not exist.", par.ty),
                            par.loc,
                            false,
                   
                        ));
                        continue;
                    }

                    if par_names.contains(&&par.ident) {
                        errors.push(Error::new(
                            TycParAllreadyDefined,
                            format!("Parameter '{}' was already defined.", par.ident),
                            par.loc,
                            false,
                       
                        ));
                        continue;
                    } else {
                        par_names.push(&par.ident);
                    }
                }
            }

            // 
            // === Network check ===
            //


            let mut network_names = Vec::with_capacity(unit.networks.len());

            for network in &unit.networks {
                let self_ty = &network.ident;

                if network_names.contains(&self_ty) {
                    errors.push(Error::new(
                        TycNetworkAllreadyDefined,
                        format!("Network '{}' was allready defined.", self_ty),
                        network.loc,
                        false,
                    
                    ))
                } else {
                    network_names.push(self_ty)
                }

                if network.nodes.is_empty() {
                    errors.push(Error::new(
                        TycNetworkEmptyNetwork, 
                        format!("Network '{}' does not contain any nodes.",  
                        self_ty), 
                        network.loc, false
                    ))
                }

                // //
                // // === Par check ===
                // //

                let mut par_names = Vec::with_capacity(network.params.len());

                for par in &network.params {
                    // Check ty
                    if !PAR_TYPES.contains(&&par.ty[..]) {
                        errors.push(Error::new(
                            TycParInvalidType,
                            format!("Parameter type '{}' does not exist.", par.ty),
                            par.loc,
                            false,
                   
                        ));
                        continue;
                    }

                    if par_names.contains(&&par.ident) {
                        errors.push(Error::new(
                            TycParAllreadyDefined,
                            format!("Parameter '{}' was already defined.", par.ident),
                            par.loc,
                            false,
                       
                        ));
                        continue;
                    } else {
                        par_names.push(&par.ident);
                    }
                }
            }
        }
        Err(_e) => errors.push(Error::new(
            TycDefNameCollission,
            format!("Name collision in '{}'", unit.asset.alias),
            Loc::new(0, 1, 1),
            false,
     
        )),
    }

    errors
}

pub fn resolve_includes<'a>(
    resolver: &'a NdlResolver,
    unit: &'a DesugaredParsingResult,
    tyctx: &mut TySpecContext<'a>,
    errors: &mut Vec<Error>,
) {
    let new_unit = tyctx.include(unit);
    if new_unit {
        // resolve meta imports.
        for include in &unit.includes {
            if let Some(unit) = resolver.desugared_units.get(&include.path) {
                resolve_includes(resolver, unit, tyctx, errors);
            } else {
                errors.push(Error::new(
                    TycIncludeInvalidAlias,
                    format!(
                        "Include '{}' cannot be resolved. No such file exists. {:?}",
                        include.path, include.loc
                    ),
                    include.loc,
                    false,
                ))
            }
        }
    }
}

