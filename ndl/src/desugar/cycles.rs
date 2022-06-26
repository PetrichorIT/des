use super::expand::ExpandedUnit;
use super::specs::{SubsystemSpec, TySpec};
use crate::parser::ConNodeIdent;
use crate::{error::*, NdlResolver};
use std::collections::HashMap;

/// Returns whether cycles were found.
pub fn check_for_cycles(
    units: &mut HashMap<String, ExpandedUnit>,
    resolver: &mut NdlResolver,
) -> bool {
    check_for_module_cycles(units, resolver) || check_for_subsystem_cycles(units, resolver)
}

fn check_for_module_cycles(
    units: &mut HashMap<String, ExpandedUnit>,
    _resolver: &mut NdlResolver,
) -> bool {
    // Module, Asset, errors
    // (&ModuleSpec, &String, &mut Vec<Error>)
    let modules_and_prototypes: Vec<_> = units
        .iter()
        .flat_map(|(asset, unit)| {
            unit.modules
                .iter()
                // Filter out alias since they all behave the same
                // so working with one instance of P is better
                .filter(|m| m.derived_from.is_none())
                .chain(unit.prototypes.iter())
                .map(move |m| (m, asset))
        })
        .collect();

    let mut edges: Vec<Vec<usize>> = Vec::new();

    for (module, _asset) in modules_and_prototypes.iter() {
        let mut outgoing = Vec::new();
        for child in module.submodules.iter() {
            // Should there be a invalid type dsg will log this error
            // but we will only evaluate the valid part of the graph
            if let Some(idx) = match &child.ty {
                TySpec::Static(ty) => modules_and_prototypes.iter().enumerate().find(|(_, m)| {
                    m.0.ident.raw() == ty.inner() && ty.exists() && !m.0.is_prototype
                }),
                TySpec::Dynamic(ty) => modules_and_prototypes.iter().enumerate().find(|(_, m)| {
                    m.0.ident.raw() == ty.inner() && ty.exists() && m.0.is_prototype
                }),
            }
            .map(|t| t.0)
            {
                outgoing.push(idx)
            }
        }

        edges.push(outgoing);
    }

    // ident, vec<submodules>, asset
    // (String, Loc, Vec<String>, String)
    let modules_and_prototypes = modules_and_prototypes
        .into_iter()
        .map(|(m, a)| {
            (
                m.ident.clone(),
                m.loc,
                m.submodules
                    .iter()
                    .map(|s| s.descriptor.clone())
                    .collect::<Vec<_>>(),
                a.clone(),
            )
        })
        .collect::<Vec<_>>();

    let mut found = false;
    for (idx, module) in modules_and_prototypes.iter().enumerate() {
        let mut visited = vec![false; modules_and_prototypes.len()];
        let mut call_path = Vec::with_capacity(modules_and_prototypes.len());
        call_path.push((idx, usize::MAX));

        let c = dfs(idx, &edges, &mut visited, &mut call_path);

        if c {
            found = true;
            // generate path
            let mut path = String::new();
            let mut current_ty = module;
            for (ty, submod) in call_path.iter().skip(1) {
                path.push_str(&format!("{}/", current_ty.2[*submod]));
                current_ty = &modules_and_prototypes[*ty];
            }

            path.pop().unwrap();

            units.get_mut(&module.3).unwrap().errors.push(Error::new(
                TycModuleSubmoduleRecrusiveTyDefinition,
                format!(
                    "Cannot create cyclic definition for type '{}' via path '{}'.",
                    module.0.raw(),
                    path
                ),
                module.1,
                false,
            ));
        }
    }
    found
}

fn check_for_subsystem_cycles(
    units: &mut HashMap<String, ExpandedUnit>,
    _resolver: &mut NdlResolver,
) -> bool {
    // Module, Asset, errors
    // (&ModuleSpec, &String, &mut Vec<Error>)
    let subsystems: Vec<(&SubsystemSpec<ConNodeIdent>, &String)> = units
        .iter()
        .flat_map(|(asset, unit)| unit.subsystems.iter().map(move |m| (m, asset)))
        .collect();

    let mut edges: Vec<Vec<usize>> = Vec::new();

    for (module, _asset) in subsystems.iter() {
        let mut outgoing = Vec::new();
        for child in module.nodes.iter() {
            // Only follow subsystem paths, modules are already tested
            // an internally cannot contain subystems.
            if let Some(idx) = match &child.ty {
                // This may also be called with child that are modules
                // but since there is no namespace problems any valid name IS a rt.
                TySpec::Static(ty) => subsystems
                    .iter()
                    .enumerate()
                    .find(|(_, m)| m.0.ident.raw() == ty.inner() && ty.exists()),
                TySpec::Dynamic(_) => None,
            }
            .map(|t| t.0)
            {
                outgoing.push(idx)
            }
        }

        edges.push(outgoing);
    }

    // ident, vec<submodules>, asset
    // (String, Loc, Vec<String>, String)
    let subsystems = subsystems
        .into_iter()
        .map(|(m, a)| {
            (
                m.ident.clone(),
                m.loc,
                m.nodes
                    .iter()
                    .map(|s| s.descriptor.clone())
                    .collect::<Vec<_>>(),
                a.clone(),
            )
        })
        .collect::<Vec<_>>();

    let mut found = false;

    for (idx, module) in subsystems.iter().enumerate() {
        let mut visited = vec![false; subsystems.len()];
        let mut call_path = Vec::with_capacity(subsystems.len());
        call_path.push((idx, usize::MAX));

        let c = dfs(idx, &edges, &mut visited, &mut call_path);

        if c {
            found = true;
            // generate path
            let mut path = String::new();
            let mut current_ty = module;
            for (ty, submod) in call_path.iter().skip(1) {
                path.push_str(&format!("{}/", current_ty.2[*submod]));
                current_ty = &subsystems[*ty];
            }

            path.pop().unwrap();

            units.get_mut(&module.3).unwrap().errors.push(Error::new(
                TycModuleSubmoduleRecrusiveTyDefinition,
                format!(
                    "Cannot create cyclic definition for type '{}' via path '{}'.",
                    module.0.raw(),
                    path
                ),
                module.1,
                false,
            ));
        }
    }

    found
}

fn dfs(
    start: usize,
    edges: &[Vec<usize>],
    visited: &mut Vec<bool>,
    // a stack straing vec of (ty_idx, submodule_idx)
    call_path: &mut Vec<(usize, usize)>,
) -> bool {
    let (node, _) = *call_path.last().unwrap();
    if visited[node] {
        return node == start;
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
