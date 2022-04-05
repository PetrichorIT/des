use std::collections::HashMap;

use crate::*;

use crate::desugar::first_pass::FstPassResult;
use crate::desugar::{DesugaredParsingResult, ScndPassGlobalTyCtx};
use crate::error::*;
use crate::parser::{ChildModuleDef, ModuleDef, TyDef};

mod tyctx;

pub use tyctx::*;

const PAR_TYPES: [&str; 15] = [
    "usize", "u8", "u16", "u32", "u64", "u128", "isize", "i8", "i16", "i32", "i64", "i128", "bool",
    "char", "String",
];

pub fn validate_module_ty(
    def: &ChildModuleDef,
    tyctx: &[&ModuleDef],
    gtyctx: &ScndPassGlobalTyCtx,
    smap: &SourceMap,
    errors: &mut Vec<Error>,
) {
    if !tyctx.iter().any(|m| m.name == def.ty.inner()) {
        // Ty missing
        let global_ty = gtyctx.module(def.ty.inner()).map(|m| m.loc);
        errors.push(Error::new_ty_missing(
            TycNetworkSubmoduleInvalidTy,
            format!("No module with name '{}' exists in the scope.", def.ty,),
            def.loc,
            smap,
            global_ty,
        ));
    }
}

///
/// Checks a given type-context for cyclic definitions and emits errors when one is found.
///
/// Note that edges must NOT point to a valid type, but invalid edges will just be ignored.
///
pub fn check_cyclic_types(all: &HashMap<String, FstPassResult>, errors: &mut Vec<Error>) {
    let modules = all
        .iter()
        .map(|(_k, v)| v.modules.iter().chain(v.prototypes.iter()))
        .flatten()
        .collect::<Vec<&ModuleDef>>();

    let mut edges: Vec<Vec<usize>> = Vec::new();

    for module in modules.iter() {
        let mut outgoing = Vec::new();
        for child in module.submodules.iter() {
            // Should there be a invalid type dsg will log this error
            // but we will only evaluate the valid part of the graph
            if let Some(idx) = match &child.ty {
                TyDef::Static(ty) => modules
                    .iter()
                    .enumerate()
                    .find(|(_, m)| m.name == *ty && !m.is_prototype),
                TyDef::Dynamic(ty) => modules
                    .iter()
                    .enumerate()
                    .find(|(_, m)| m.name == *ty && m.is_prototype),
            }
            .map(|t| t.0)
            {
                outgoing.push(idx)
            }
        }

        edges.push(outgoing);
    }

    // Depth first search

    fn dfs(
        start: usize,
        edges: &[Vec<usize>],
        visited: &mut Vec<bool>,
        // a stack straing vec of (ty_idx, submodule_idx)
        call_path: &mut Vec<(usize, usize)>,
    ) -> bool {
        let (node, _) = *call_path.last().unwrap();
        if visited[node] {
            if node == start {
                return true;
            } else {
                return false;
            }
        }

        visited[node] = true;
        for (submod_idx, edge) in edges[node].iter().enumerate() {
            call_path.push((*edge, submod_idx));
            let cycle = dfs(start, edges, visited, call_path);
            if cycle {
                return true;
            }
            call_path.pop();
        }

        false
    }

    for (idx, module) in modules.iter().enumerate() {
        let mut visited = vec![false; modules.len()];
        let mut call_path = Vec::with_capacity(modules.len());
        call_path.push((idx, usize::MAX));

        let c = dfs(idx, &edges, &mut visited, &mut call_path);

        if c {
            // generate path
            let mut path = String::new();
            let mut current_ty = module;
            for (ty, submod) in call_path.iter().skip(1) {
                path.push_str(&format!("{}/", current_ty.submodules[*submod].desc));
                current_ty = &modules[*ty];
            }

            path.pop().unwrap();

            errors.push(Error::new(
                TycModuleSubmoduleRecrusiveTyDefinition,
                format!(
                    "Cannot create cyclic definition for type '{}' via path '{}'.",
                    module.name, path
                ),
                module.loc,
                false,
            ));
        }
    }
}

///
/// Validates the given an internal DesugaredParsingResult 'unit' using the resovler
/// as parameters.
/// Returns all sematic errors that were encountered.
///
pub fn validate(unit: &DesugaredParsingResult, resolver: &NdlResolver) -> Vec<Error> {
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
                    // errors.push(Error::new(
                    //     TycModuleAllreadyDefined,
                    //     format!("Module '{}' was allready defined.", self_ty),
                    //     module.loc,
                    //     false,

                    // ))
                } else {
                    module_names.push(self_ty)
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
                        // TODO: Defer to dsg else UB
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
                    // errors.push(Error::new(
                    //     TycNetworkAllreadyDefined,
                    //     format!("Network '{}' was allready defined.", self_ty),
                    //     network.loc,
                    //     false,

                    // ))
                } else {
                    network_names.push(self_ty)
                }

                if network.nodes.is_empty() {
                    errors.push(Error::new(
                        TycNetworkEmptyNetwork,
                        format!("Network '{}' does not contain any nodes.", self_ty),
                        network.loc,
                        false,
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
                // This should have been checked beforehand

                // errors.push(Error::new(
                //     TycIncludeInvalidAlias,
                //     format!(
                //         "Include '{}' cannot be resolved. No such file exists. {:?}",
                //         include.path, include.loc
                //     ),
                //     include.loc,
                //     false,
                // ))
            }
        }
    }
}
