use crate::{
    error::*,
    loc::Loc,
    NdlResolver, DesugaredParsingResult,
};

mod tests;
mod tyctx;

pub use tyctx::*;

const PAR_TYPES: [&str; 15] = [
    "usize", "u8", "u16", "u32", "u64", "u128", "isize", "i8", "i16", "i32", "i64", "i128", "bool",
    "char", "String",
];

///
/// Validates the given [DesugaredParsingResult] 'unit' using the resovler
/// as parameters.
/// Returns all sematic errors that were encountered.
///
#[allow(unused)]
pub fn validate(
    unit: &DesugaredParsingResult,
    resolver: &NdlResolver,
) -> Vec<Error> {
    let mut tyctx = TySpecContext::new();
    let mut errors = Vec::new();

    let global_tyctx = resolver.gtyctx_spec();

    let asset = resolver.source_map.get_asset(&unit.asset.alias).unwrap();

    resolve_includes(resolver, unit, &mut tyctx, &mut errors);

    match tyctx.check_name_collision() {
        Ok(()) => {
            //
            // === Module check ===
            //

            let mut module_names = Vec::new();

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

                // Check submodule namespaces and types
                let mut descriptors = Vec::new();

                for submodule in &module.submodules {
                    if descriptors.contains(&&submodule.descriptor) {
                        errors.push(Error::new(
                            TycModuleSubmoduleFieldAlreadyDeclared,
                            format!("Field '{}' was already declared.", submodule),
                            submodule.loc,
                            false,
                        
                        ));
                    }
                    descriptors.push(&submodule.descriptor);

                    let ty_valid = tyctx.modules.iter().any(|&m| m.ident == *submodule.ty);

                    if submodule.ty == *self_ty {
                        errors.push(Error::new(
                            TycModuleSubmoduleRecrusiveTyDefinition,
                            format!("Module '{0}' has a required submodule of type '{0}'. Cannot create cyclic definitions.", submodule.ty),
                            submodule.loc,
                            false,
                      
                        ))
                    } else if !ty_valid {
                        let gty = global_tyctx.module(&submodule.ty[..]);

                        errors.push(Error::new_ty_missing(
                            TycModuleSubmoduleInvalidTy,
                            format!(
                                "No module with name '{}' exists in the scope of module '{}'.",
                                submodule.ty, module.ident
                            ),
                            submodule.loc,
                            asset.source_map(),
                            gty.map(|ty| ty.loc),
                        ));
                    }
                }

                //
                // === Gate check ===
                //

                let mut self_gates = Vec::new();
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
                // === Connection check ===
                //

                // for connection in &module.connections {
                //     // check peers
                //     let peers = [&connection.from, &connection.to].map(|peer| {
                //         match peer {
                //             ConNodeIdent::Child { loc, child, ident} => {
                //                 // Referencing subvalue
                //                 let peer_ident_valid = descriptors.contains(&&child);
                //                 if !peer_ident_valid {
                //                     errors.push(Error::new(
                //                         TycModuleConUnknownIdentSymbol,
                //                         format!(
                //                             "No submodule '{}' exists on module '{}'",
                //                             child, module.name
                //                         ),
                //                         *loc,
                //                         false,
                //                     ));

                //                     return None;
                //                 }

                //                 let submod = module
                //                     .submodules
                //                     .iter()
                //                     .find(|&m| m.desc.descriptor == *child)
                //                     .unwrap();

                //                 let mod_def = tyctx.modules.iter().find(|m| m.name == submod.ty);

                //                 // if referenced submodule has invalid ty
                //                 // this error was already handled
                //                 mod_def?;

                //                 let mod_def = mod_def.unwrap();

                //                 let peer_subident_valid =
                //                     mod_def.gates.iter().find(|g| g.name == *ident);

                //                 if peer_subident_valid.is_none() {
                //                     errors.push(Error::new(
                //                         TycModuleConUnknownIdentSymbol,
                //                         format!(
                //                             "No gate '{}' exists on submodule '{}' of type '{}'",
                //                             ident, child, mod_def.name
                //                         ),
                //                         *loc,
                //                         false,
                                    
                //                     ));

                //                     return None;
                //                 }

                //                 peer_subident_valid
                //             },
                //             ConNodeIdent::Local { loc, ident} => {
                //                 // referencing direct value
                //                 let peer_valid = module.gates.iter().find(|g| g.name == *ident);
                //                 if peer_valid.is_none() {
                //                     errors.push(Error::new(
                //                         TycModuleConUnknownIdentSymbol,
                //                         format!(
                //                             "No gate '{}' exists on module '{}'",
                //                             ident, module.name
                //                         ),
                //                         *loc,
                //                         false,
                                    
                //                     ));

                //                     return None;
                //                 }

                //                 peer_valid
                //             }
                //         }
                //     });

                //     if let Some(from) = peers[0] {
                //         if let Some(to) = peers[1] {
                //             if from.size != to.size {
                //                 // This could only be a warning once handeling procedures
                //                 // are implemented

                //                 errors.push(
                //                     Error::new(
                //                     TycModuleConNonMatchingGateSizes,
                //                     format!("Gates '{}' and '{}' cannot be connected since they have different sizes.", from, to),
                //                     connection.loc,
                //                     false,
                         
                //                 ))
                //             }
                //         }
                //     }
                // }

                //
                // === Par check ===
                //

                let mut par_names = Vec::new();

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

            let mut network_names = Vec::new();

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

                // Check submodule namespaces and types
                let mut descriptors = Vec::new();

                for node in &network.nodes {
                    if descriptors.contains(&&node.descriptor) {
                        errors.push(Error::new(
                            TycnetworkSubmoduleFieldAlreadyDeclared,
                            format!("Field '{}' was already declared.", node.descriptor),
                            node.loc,
                            false,
                        
                        ));
                    }
                    descriptors.push(&node.descriptor);

                    let ty_valid = tyctx.modules.iter().any(|&m| m.ident == *node.ty);

                    // Cyclic definition is not possible since submoudles are modules while
                    // networks are top-level only definitions.

                    if !ty_valid {
                        let gty = global_tyctx.module(&node.ty[..]);

                        errors.push(Error::new_ty_missing(
                            TycNetworkSubmoduleInvalidTy,
                            format!(
                                "No module with name '{}' exists in the scope of network '{}'.",
                                node.ty, network.ident
                            ),
                            node.loc,
                            asset.source_map(),
                            gty.map(|ty| ty.loc),
                        ));
                    }
                }

                //
                // === Connection check ===
                //

                // for connection in &network.connections {

                //     // check peers
                //     let peers = [&connection.from, &connection.to].map(|peer| {
                //         match peer {
                //             ConNodeIdent::Child { loc, child, ident} => {
                //                 // Referencing subvalue
                //                 let peer_ident_valid = descriptors.contains(&&child);
                //                 if !peer_ident_valid {
                //                     errors.push(Error::new(
                //                         TycModuleConUnknownIdentSymbol,
                //                         format!(
                //                             "No submodule '{}' exists on module '{}'",
                //                             child, network.name
                //                         ),
                //                         *loc,
                //                         false,
                //                     ));

                //                     return None;
                //                 }

                //                 let submod = network
                //                     .nodes
                //                     .iter()
                //                     .find(|&m| m.desc.descriptor == *child)
                //                     .unwrap();

                //                 let mod_def = tyctx.modules.iter().find(|m| m.name == submod.ty);

                //                 // if referenced submodule has invalid ty
                //                 // this error was already handled
                //                 mod_def?;

                //                 let mod_def = mod_def.unwrap();

                //                 let peer_subident_valid =
                //                     mod_def.gates.iter().find(|g| g.name == *ident);

                //                 if peer_subident_valid.is_none() {
                //                     errors.push(Error::new(
                //                         TycNetworkConUnknownIdentSymbol,
                //                         format!(
                //                             "No gate '{}' exists on submodule '{}' of type '{}'",
                //                             ident, child, mod_def.name
                //                         ),
                //                         *loc,
                //                         false,
                                    
                //                     ));

                //                     return None;
                //                 }

                //                 peer_subident_valid
                //             },
                //             ConNodeIdent::Local { loc, ident} => {
                //                // Cannot reference local gates since no local gates
                //                // exist for network.
                //                 errors.push(Error::new(
                //                     TycNetworkConIllegalLocalNodeIdent,
                //                     format!("Cannot refernce local gate '{}' if no local gates exist on network.", ident),
                //                     *loc,
                //                     false
                //                 ));

                //                None
                //             }
                //         }
                //     });

                //     if let Some(from) = peers[0] {
                //         if let Some(to) = peers[1] {
                //             if from.size != to.size {
                //                 // This could only be a warning once handeling procedures
                //                 // are implemented

                //                 errors.push(
                //                     Error::new(
                //                         TycNetworkConNonMatchingGateSizes,
                //                     format!("Gates '{}' and '{}' cannot be connected since they have different sizes.", from, to),
                //                     connection.loc,
                //                     false,
                         
                //                 ))
                //             }
                //         }
                //     }
                // }

                //
                // === Par check ===
                //

                let mut par_names = Vec::new();

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

fn resolve_includes<'a>(
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

