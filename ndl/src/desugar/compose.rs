use super::{
    ctx::TyComposeContext,
    expand::ExpandedUnit,
    specs::{ConSpec, ConSpecNodeIdent, ExportSpec, GateSpec, ModuleSpec, SubsystemSpec},
};
use crate::{common::OType, error::*, DesugaredResult, Error, GateAnnotation, NdlResolver};
use std::{cell::RefCell, collections::HashMap};

///
/// This function takes the global context and does the following:
///
/// (1) - All [exports] on subsystems are evauted and their size is determined.
/// (2) - All [connections] on subsystems are checked, based on the size information.
/// (3) - All [protpimpls] are checked
///
pub fn compose(
    units: &mut HashMap<String, ExpandedUnit>,
    resolver: &mut NdlResolver,
) -> DesugaredResult {
    let mut tyctx = TyComposeContext::new(units, resolver);
    let mut errors = Vec::new();

    // TODO:
    // Make callstack frame based for full tree struture

    // Fetch all subsystems
    // Since typechecking has allready commenced and all ty are OIdent a global scale can be applied
    // If the TyPath is invalid just ignore
    let subsystems = units
        .iter()
        .map(|unit| unit.1.subsystems.iter())
        .flatten()
        .collect::<Vec<_>>();

    // Build a AJL or the graph
    // This may have circles ! be aware
    let mut deps = vec![Vec::new(); subsystems.len()];
    for idx in 0..subsystems.len() {
        let buf = &mut deps[idx];
        for node in &subsystems[idx].nodes {
            if let Some(ident) = node.ty.valid_ident() {
                if ident.typ() == OType::Subsystem {
                    // Child found, now get index

                    let (idx, _) = subsystems
                        .iter()
                        .enumerate()
                        .find(|(_, e)| e.ident == *ident)
                        .expect("Type checking has allready succeded and this is either InScope or OutOfScope");
                    buf.push(idx);
                }
            }
        }

        buf.dedup()
    }

    // A set of processed steps
    let mut done: Vec<Option<ComposedSubsystem>> = vec![None; subsystems.len()];

    // Vector of indices that must be evaluated first.

    for i in 0..subsystems.len() {
        if done[i].is_some() {
            continue;
        }

        let mut callstack = Vec::new();
        callstack.push(i);

        while let Some(idx) = callstack.pop() {
            // Skip redundand calls
            if done[idx].is_some() {
                continue;
            }

            let subsys = subsystems[idx];
            let dep = &deps[idx];

            // Check all deps first
            let mut missing = Vec::new();
            for d in dep {
                if done[*d].is_none() {
                    missing.push(*d);
                }
            }

            if !missing.is_empty() {
                callstack.push(idx);
                callstack.append(&mut missing);

                continue;
            }

            // Compose the element, all deps are ready
            let mut composed = ComposedSubsystem::from_spec(subsys);

            for export in &subsys.exports {
                let ExportSpec {
                    loc,
                    node_ident,
                    node_ty,
                    gate_ident,
                } = export;

                // Note that gate_ident has a invalid size / loc / annotation
                // only ident is valid

                if let Some(oident) = node_ty.valid_ident() {
                    let (loc, size, annotation) = match oident.typ() {
                        OType::Module | OType::Prototype => {
                            let sp = tyctx.module_or_proto(oident).unwrap();
                            if let Some(g) = sp.gates.iter().find(|g| g.ident == gate_ident.ident) {
                                (g.loc, g.size, g.annotation)
                            } else {
                                errors.push(Error::new(
                                    DsgExportInvalidGateIdent,
                                    format!(
                                        "Cannot export gate '{}' since module '{}' has no such gate.",
                                        gate_ident,
                                        oident.raw()
                                    ),
                                    *loc,
                                    false,
                                ));
                                continue;
                            }
                        }
                        OType::Subsystem => {
                            // get i
                            let (i, _) = subsystems
                                .iter()
                                .enumerate()
                                .find(|(_, s)| s.ident == *oident)
                                .unwrap();

                            let csys = done[i].as_ref().unwrap();
                            if let Some(g) = csys
                                .exports
                                .iter()
                                .find(|g| g.gate_ident.ident == gate_ident.ident)
                            {
                                (g.loc, g.gate_ident.size, g.gate_ident.annotation)
                            } else {
                                errors.push(Error::new(
                                    DsgExportInvalidGateIdent,
                                    format!(
                                        "Cannot export gate '{}' since subsystem '{}' exports no such gate.",
                                        gate_ident,
                                        oident.raw()
                                    ),
                                    *loc,
                                    false,
                                ));
                                continue;
                            }
                        }
                        _ => unreachable!(),
                    };

                    composed.exports.push(ExportSpec {
                        loc: export.loc,
                        node_ident: node_ident.clone(),
                        node_ty: node_ty.clone(),
                        gate_ident: GateSpec {
                            loc,
                            size,
                            ident: gate_ident.ident.clone(),
                            annotation,
                        },
                    })
                }
            }

            done[idx] = Some(composed);
        }
    }

    debug_assert!(done.iter().all(|v| v.is_some()));
    let done = RefCell::new(done.into_iter().map(Option::unwrap).collect());

    tyctx.attach(&done);

    // C check
    for i in 0..subsystems.len() {
        let subsys = subsystems[i];
        // Check connections
        for connection in &subsys.connections {
            let ConSpec {
                loc,
                source: from,
                channel,
                target: to,
            } = connection;

            let (f_nodes_len, f_gate_size, from_idents) =
                match super::connections::resolve_connection_ident_compose(
                    from,
                    &subsys.nodes,
                    &tyctx,
                    &subsys.ident,
                    &mut errors,
                    GateAnnotation::Output,
                ) {
                    Some(v) => v,
                    None => continue,
                };
            let (t_nodes_len, t_gate_size, to_idents) =
                match super::connections::resolve_connection_ident_compose(
                    to,
                    &&subsys.nodes,
                    &tyctx,
                    &subsys.ident,
                    &mut errors,
                    GateAnnotation::Input,
                ) {
                    Some(v) => v,
                    None => continue,
                };

            // Gurantee that count(from) <= count(to)
            // This allows partial targeting of later gates.
            if from_idents.len() != to_idents.len() {
                errors.push(Error::new(
                    DsgConGateSizedToNotMatch,
                    format!(
                        "Connection gate cluster sizes do not match ({}*{} != {}*{}).",
                        f_nodes_len, f_gate_size, t_nodes_len, t_gate_size
                    ),
                    *loc,
                    false,
                ));

                // Continue anyway will be aborted nonetheless
            }

            // Resolve the channel desc once,
            // the reuse the same desc.

            for (source, target) in from_idents.into_iter().zip(to_idents.into_iter()) {
                done.borrow_mut()[i].connections.push(ConSpec {
                    loc: *loc,

                    source,
                    target,
                    channel: channel.clone(),
                })
            }
        }
    }

    DesugaredResult {
        modules: units
            .iter()
            .map(|u| u.1.modules.iter())
            .flatten()
            .cloned()
            .collect::<Vec<_>>(),
        subsystems: done.into_inner(),
        errors,
    }
}

pub type ComposedModule = ModuleSpec<ConSpecNodeIdent>;
pub type ComposedSubsystem = SubsystemSpec<ConSpecNodeIdent>;
