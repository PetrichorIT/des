use crate::{
    common::{OIdent, OType},
    error::*,
    parser::{ChildNodeDef, ConNodeIdent, GateDef, Ident},
    utils::{edit_distance, TyResolveError, TyResolveResult},
    GateAnnotation,
};

use super::{
    ctx::{TyComposeContext, TyDefContext, MAX_ERROR_EDIT_DISTANCE},
    specs::*,
};

pub fn gate_from_set_by_ident<'a>(
    set: &'a [GateDef],
    ident: &Ident,
) -> TyResolveResult<(&'a GateDef, Option<usize>)> {
    match ident {
        Ident::Direct { ident } => gate_from_set(set, ident)
            .map(|v| (v, None))
            .map_err(|e| e.map(|i| (i, None))),
        Ident::Clustered { ident, index } => gate_from_set(set, ident)
            .map(|v| (v, Some(*index)))
            .map_err(|e| e.map(|i| (i, Some(*index)))),
    }
}

pub fn gate_from_set<'a>(set: &'a [GateDef], raw_ident: &str) -> TyResolveResult<&'a GateDef> {
    match set.iter().find(|g| g.name == raw_ident) {
        Some(g) => Ok(g),
        None => {
            // Find best lookalike
            let lookalike = set
                .iter()
                .map(|g| (g, edit_distance(&g.name, raw_ident)))
                // NOTE: Relaxe filter since gate names are usually short
                .filter(|(g, d)| *d < MAX_ERROR_EDIT_DISTANCE && g.name.len() >= 2 * d)
                .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

            match lookalike {
                Some((lookalike, distance)) => {
                    Err(TyResolveError::FoundLookalike(lookalike, distance))
                }
                None => Err(TyResolveError::NoneFound),
            }
        }
    }
}

fn child_modules_from_set<'a>(
    set: &'a [ChildNodeDef],
    child: &Ident,
) -> TyResolveResult<&'a ChildNodeDef> {
    match child {
        // Can either describe a primitiv or a clustered group that
        // will not be indexed.
        Ident::Direct { ident } => {
            // Its an easy naming so we cann allow results
            match set.iter().find(|m| m.desc.descriptor == *ident) {
                Some(c) => Ok(c),
                None => {
                    // Find best lookalike
                    let lookalike = set
                        .iter()
                        .map(|g| (g, edit_distance(&g.desc.descriptor, ident)))
                        .filter(|(g, d)| {
                            *d < MAX_ERROR_EDIT_DISTANCE && g.desc.descriptor.len() > 2 * d
                        })
                        .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                    match lookalike {
                        Some((lookalike, distance)) => {
                            Err(TyResolveError::FoundLookalike(lookalike, distance))
                        }
                        None => Err(TyResolveError::NoneFound),
                    }
                }
            }
        }
        Ident::Clustered { ident, index } => {
            match set.iter().find(|m| {
                m.desc.descriptor == *ident
                    && m.desc.cluster_bounds.is_some()
                    && m.desc.cluster_bounds_contain(*index)
            }) {
                Some(c) => Ok(c),
                None => {
                    // Find best lookalike
                    let lookalike = set
                        .iter()
                        .map(|c| (c, edit_distance(&c.desc.descriptor, ident)))
                        .filter(|(c, d)| {
                            *d < MAX_ERROR_EDIT_DISTANCE
                                && c.desc.descriptor.len() > 2 * d
                                && c.desc.cluster_bounds.is_some()
                                && c.desc.cluster_bounds_contain(*index)
                        })
                        .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                    match lookalike {
                        Some((lookalike, distance)) => {
                            Err(TyResolveError::FoundLookalike(lookalike, distance))
                        }
                        None => Err(TyResolveError::NoneFound),
                    }
                }
            }
        }
    }
}

pub fn gatespec_from_set_by_ident(
    set: &[GateSpec],
    ident: &Ident,
) -> TyResolveResult<(GateSpec, Option<usize>)> {
    match ident {
        Ident::Direct { ident } => gatespec_from_set(set, ident)
            .map(|v| (v, None))
            .map_err(|e| e.map(|i| (i, None))),
        Ident::Clustered { ident, index } => gatespec_from_set(set, ident)
            .map(|v| (v, Some(*index)))
            .map_err(|e| e.map(|i| (i, Some(*index)))),
    }
}

pub fn gatespec_from_set(set: &[GateSpec], raw_ident: &str) -> TyResolveResult<GateSpec> {
    match set.iter().find(|g| g.ident == raw_ident) {
        Some(g) => Ok(g.clone()),
        None => {
            // Find best lookalike
            let lookalike = set
                .iter()
                .map(|g| (g, edit_distance(&g.ident, raw_ident)))
                // NOTE: Relaxe filter since gate names are usually short
                .filter(|(g, d)| *d < MAX_ERROR_EDIT_DISTANCE && g.ident.len() >= 2 * d)
                .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

            match lookalike {
                Some((lookalike, distance)) => {
                    Err(TyResolveError::FoundLookalike(lookalike.clone(), distance))
                }
                None => Err(TyResolveError::NoneFound),
            }
        }
    }
}

/*let submod_def = match child {
    // Can either describe a primitiv or a clustered group that
    // will not be indexed.
    Ident::Direct { ident } => child_modules.iter().find(|m| m.descriptor == *ident),
    Ident::Clustered { ident, index } => child_modules
        .iter()
        .find(|m| m.descriptor == format!("{}[{}]", ident, index)),
    /*ident
    && m.desc.cluster_bounds.is_some()
    && m.desc.cluster_bounds_contain(*index) */
}; */

fn childspec_modules_from_set<'a>(
    set: &'a [ChildNodeSpec],
    child: &Ident,
) -> TyResolveResult<&'a ChildNodeSpec> {
    match child {
        // Can either describe a primitiv or a clustered group that
        // will not be indexed.
        Ident::Direct { ident } => match set.iter().find(|m| m.descriptor == *ident) {
            Some(v) => Ok(v),
            None => {
                // Find best lookalike
                let lookalike = set
                    .iter()
                    .map(|g| (g, edit_distance(&g.descriptor, ident)))
                    .filter(|(g, d)| *d < MAX_ERROR_EDIT_DISTANCE && g.descriptor.len() > 2 * d)
                    .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                match lookalike {
                    Some((lookalike, distance)) => {
                        Err(TyResolveError::FoundLookalike(lookalike, distance))
                    }
                    None => Err(TyResolveError::NoneFound),
                }
            }
        },
        Ident::Clustered { ident, index } => {
            let full_ident = format!("{}[{}]", ident, index);
            match set.iter().find(|m| m.descriptor == full_ident) {
                Some(v) => Ok(v),
                None => {
                    // Find best lookalike
                    let lookalike = set
                        .iter()
                        .map(|g| (g, edit_distance(&g.descriptor, &full_ident)))
                        .filter(|(g, d)| *d < MAX_ERROR_EDIT_DISTANCE && g.descriptor.len() > 2 * d)
                        .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                    match lookalike {
                        Some((lookalike, distance)) => {
                            Err(TyResolveError::FoundLookalike(lookalike, distance))
                        }
                        None => Err(TyResolveError::NoneFound),
                    }
                }
            }
        } /*ident
          && m.desc.cluster_bounds.is_some()
          && m.desc.cluster_bounds_contain(*index) */
    }
}

pub(crate) fn resolve_connection_ident(
    ident: &ConNodeIdent,
    local_gates: &[GateDef],
    child_modules: &[ChildNodeDef],

    supertype: &OIdent,
    tyctx: &TyDefContext<'_>,
    errors: &mut Vec<Error>,
    expected_type: GateAnnotation,
) -> Option<(usize, usize, Vec<ConSpecNodeIdent>)> {
    let global_loc = ident.loc();

    let mut result = Vec::new();
    match ident {
        ConNodeIdent::Local { loc, ident } => {
            let global_ident = ident;
            // Local can only reference a modules gate
            let gate = match gate_from_set(local_gates, ident.raw_ident()) {
                Ok(gate) => gate,
                Err(e) => {
                    errors.push(Error::new_with_lookalike(
                        DsgConInvalidLocalGateIdent,
                        format!("No local gate cluster '{}' exists on this module.", ident),
                        *loc,
                        false,
                        e.lookalike().map(|g| (&g.0.name[..], g.0.loc)),
                    ));
                    return None;
                }
            };

            if gate.size < 1 {
                // This should be catched by the [tychk]
                return None;
            }

            if !(gate.annotation == expected_type || gate.annotation == GateAnnotation::Unknown) {
                // invalid annotation connection
                errors.push(Error::new_with_solution(
                    DsgGateConnectionViolatesAnnotation,
                    format!(
                        "Gate '{}' cannot be used as {} of a connection since it is defined as {}.",
                        global_ident,
                        if matches!(expected_type, GateAnnotation::Input) {
                            "end"
                        } else {
                            "start"
                        },
                        gate.annotation
                    ),
                    global_loc,
                    false,
                    ErrorSolution::new(
                        format!(
                            "Define gate '{}' as {}",
                            global_ident,
                            if matches!(expected_type, GateAnnotation::Input) {
                                "@input"
                            } else {
                                "@output"
                            }
                        ),
                        gate.loc,
                    ),
                ));
                // Continue either way since annotations violations
                // are not transient
            }

            match ident {
                Ident::Direct { .. } => {
                    // maybe add gate.size for debug message creation later
                    for pos in 0..gate.size {
                        result.push(ConSpecNodeIdent::Local {
                            loc: *loc,
                            gate_ident: ident.raw_ident().to_string(),
                            pos,
                        });
                    }
                    Some((1, gate.size, result))
                }
                Ident::Clustered { index, .. } => {
                    result.push(ConSpecNodeIdent::Local {
                        loc: *loc,
                        gate_ident: ident.raw_ident().to_string(),
                        pos: *index,
                    });
                    Some((1, 1, result))
                }
            }
        }
        ConNodeIdent::Child { loc, child, ident } => {
            let global_ident = ident;

            // maybe referces clustered submodules.
            let submod_def = child_modules_from_set(child_modules, child);

            match submod_def {
                Ok(submod_def) => {
                    // fetch module ty
                    // this can be done outside the following if-else
                    // since a cluster-definition shares the same ty
                    let sub_module_ty = match tyctx.module_or_proto(&submod_def.ty) {
                        Some(sub_module) => sub_module,
                        None => {
                            panic!("This should have been checked when submodules were validated");
                        }
                    };

                    // fetch gate
                    let gate_def = gate_from_set_by_ident(&sub_module_ty.gates, ident);

                    // fetch gate
                    let (gate_def, gate_def_cindex) = match gate_def {
                        Ok(gate_def) => gate_def,
                        Err(e) => {
                            errors.push(Error::new_with_lookalike(
                                DsgConInvalidLocalGateIdent,
                                format!(
                                    "No local gate cluster '{}' exists on module '{}'.",
                                    ident,
                                    sub_module_ty.ident.raw()
                                ),
                                *loc,
                                false,
                                e.lookalike().map(|l| (&l.0 .0.name[..], l.0 .0.loc)),
                            ));
                            return None;
                        }
                    };

                    if gate_def.size < 1 {
                        // This should be catched by [tychk]
                        return None;
                    }

                    if !(gate_def.annotation == expected_type
                        || gate_def.annotation == GateAnnotation::Unknown)
                    {
                        // invalid annotation connection
                        errors.push(Error::new_with_solution(
                        DsgGateConnectionViolatesAnnotation,
                        format!(
                            "Gate '{}' cannot be used as {} of a connection since it is defined as {}.",
                            global_ident,
                            if matches!(expected_type, GateAnnotation::Input) {
                                "end"
                            } else {
                                "start"
                            },
                            gate_def.annotation
                        ),
                        global_loc,
                        false,
                        ErrorSolution::new(
                            format!(
                                "Define gate '{}' as {}",
                                global_ident,
                                if matches!(expected_type, GateAnnotation::Input) {
                                    "@input"
                                } else {
                                    "@output"
                                }
                            ),
                            gate_def.loc,
                        ),
                    ));
                        // Continue either way since annotations violations
                        // are not transient
                    }

                    let gate_ident = ident.raw_ident();

                    if let Some((from_id, to_id)) = submod_def.desc.cluster_bounds {
                        // referenced child is clustered

                        if let Ident::Clustered { ident, index } = child {
                            // cluster is resolved through indexing
                            let child_ident = format!("{}[{}]", ident, index);

                            if let Some(gate_def_cindex) = gate_def_cindex {
                                result.push(ConSpecNodeIdent::Child {
                                    loc: *loc,
                                    child_ident,
                                    gate_ident: gate_ident.to_string(),
                                    pos: gate_def_cindex,
                                })
                            } else {
                                for pos in 0..gate_def.size {
                                    result.push(ConSpecNodeIdent::Child {
                                        loc: *loc,
                                        child_ident: child_ident.clone(),
                                        gate_ident: gate_ident.to_string(),
                                        pos,
                                    });
                                }
                            }
                            Some((1, gate_def.size, result))
                        } else {
                            // Make clustersed spec
                            for id in from_id..=to_id {
                                let child_ident = format!("{}[{}]", child, id);

                                // maybe add gate.size for debug message creation later
                                if let Some(gate_def_cindex) = gate_def_cindex {
                                    result.push(ConSpecNodeIdent::Child {
                                        loc: *loc,
                                        child_ident: child_ident.clone(),
                                        gate_ident: gate_ident.to_string(),
                                        pos: gate_def_cindex,
                                    })
                                } else {
                                    for pos in 0..gate_def.size {
                                        result.push(ConSpecNodeIdent::Child {
                                            loc: *loc,
                                            child_ident: child_ident.clone(),
                                            gate_ident: gate_ident.to_string(),
                                            pos,
                                        });
                                    }
                                }
                            }
                            Some((to_id + 1 - from_id, gate_def.size, result))
                        }
                    } else {
                        // make normal spec.

                        // maybe add gate.size for debug message creation later
                        if let Some(gate_def_cindex) = gate_def_cindex {
                            result.push(ConSpecNodeIdent::Child {
                                loc: *loc,
                                child_ident: child.to_string(),
                                gate_ident: gate_ident.to_string(),
                                pos: gate_def_cindex,
                            })
                        } else {
                            for pos in 0..gate_def.size {
                                result.push(ConSpecNodeIdent::Child {
                                    loc: *loc,
                                    child_ident: child.to_string(),
                                    gate_ident: gate_ident.to_string(),
                                    pos,
                                });
                            }
                        }

                        Some((1, gate_def.size, result))
                    }
                }
                Err(e) => {
                    errors.push(Error::new_with_lookalike(
                        DsgConInvalidField,
                        format!(
                            "Field '{}' was not defined on module '{}'.",
                            child,
                            supertype.raw()
                        ),
                        *loc,
                        false,
                        e.lookalike().map(|c| (&c.0.desc.descriptor[..], c.0.loc)),
                    ));

                    None
                }
            }
        }
    }
}

pub(crate) fn resolve_connection_ident_compose(
    ident: &ConNodeIdent,
    child_modules: &[ChildNodeSpec],
    tyctx: &TyComposeContext<'_>,

    supertype: &OIdent,
    errors: &mut Vec<Error>,
    expected_type: GateAnnotation,
) -> Option<(usize, usize, Vec<ConSpecNodeIdent>)> {
    let global_loc = ident.loc();

    let mut result = Vec::new();
    match ident {
        // There are no local gates
        ConNodeIdent::Local { .. } => None,
        ConNodeIdent::Child { loc, child, ident } => {
            let global_ident = ident;

            // maybe referces clustered submodules.

            let submod_def = childspec_modules_from_set(child_modules, child);

            match submod_def {
                Ok(submod_def) => {
                    // fetch module ty
                    // this can be done outside the following if-else
                    // since a cluster-definition shares the same ty

                    let valid_ident = match submod_def.ty.valid_ident() {
                        Some(v) => v,
                        None => todo!(),
                    };

                    // Want to fetch (gate: &GateSpec, Option<usize>)
                    let gate_def = match tyctx.module_or_proto(valid_ident) {
                        Some(sub_module) => {
                            // Its a module
                            gatespec_from_set_by_ident(&sub_module.gates, ident)
                        }
                        // TODO: Support lookalikes in this case aswell
                        None => match tyctx.composed_subsystem_gate(valid_ident, ident) {
                            Some(r) => match r {
                                Some(v) => Ok(v),
                                None => Err(TyResolveError::NoneFound),
                            },
                            None => {
                                // TODO: Support lookalikes aswell
                                errors.push(Error::new_ty_missing(
                                    DsgConInvalidLocalGateIdent,
                                    format!(
                                        "No module or subsystem '{}' exists as requested for child module '{}'.",
                                        valid_ident.raw(),
                                        child
                                    ),
                                    *loc,
                                    tyctx.source_map(),
                                    tyctx
                                        .module(submod_def.ty.valid_ident().unwrap())
                                        .map(|module| module.loc),
                                ));
                                return None;
                            }
                        },
                    };

                    // fetch gate
                    let (gate_def, gate_def_cindex) = match gate_def {
                        Ok(gate_def) => gate_def,
                        Err(e) => {
                            errors.push(Error::new_with_lookalike(
                                DsgConInvalidLocalGateIdent,
                                format!(
                                    "No local gate cluster '{}' exists on {} '{}'.",
                                    ident,
                                    if valid_ident.typ() == OType::Subsystem {
                                        "subsystem"
                                    } else {
                                        "module"
                                    },
                                    valid_ident.raw()
                                ),
                                *loc,
                                false,
                                e.lookalike().map(|d| (&d.0 .0.ident[..], d.0 .0.loc)),
                            ));
                            return None;
                        }
                    };

                    if gate_def.size < 1 {
                        // This should be catched by [tychk]
                        return None;
                    }

                    if !(gate_def.annotation == expected_type
                        || gate_def.annotation == GateAnnotation::Unknown)
                    {
                        // invalid annotation connection
                        errors.push(Error::new_with_solution(
                            DsgGateConnectionViolatesAnnotation,
                            format!(
                                "Gate '{}' cannot be used as {} of a connection since it is defined as {}.",
                                global_ident,
                                if matches!(expected_type, GateAnnotation::Input) {
                                    "end"
                                } else {
                                    "start"
                                },
                                gate_def.annotation
                            ),
                            global_loc,
                            false,
                            ErrorSolution::new(
                                format!(
                                    "Define gate '{}' as {}",
                                    global_ident,
                                    if matches!(expected_type, GateAnnotation::Input) {
                                        "@input"
                                    } else {
                                        "@output"
                                    }
                                ),
                                gate_def.loc,
                            ),
                        ));
                        // Continue either way since annotations violations
                        // are not transient
                    }

                    let gate_ident = ident.raw_ident();

                    // IF some cluster bounds is not possible
                    {
                        // make normal spec.

                        // maybe add gate.size for debug message creation later
                        if let Some(gate_def_cindex) = gate_def_cindex {
                            result.push(ConSpecNodeIdent::Child {
                                loc: *loc,
                                child_ident: child.to_string(),
                                gate_ident: gate_ident.to_string(),
                                pos: gate_def_cindex,
                            })
                        } else {
                            for pos in 0..gate_def.size {
                                result.push(ConSpecNodeIdent::Child {
                                    loc: *loc,
                                    child_ident: child.to_string(),
                                    gate_ident: gate_ident.to_string(),
                                    pos,
                                });
                            }
                        }

                        Some((1, gate_def.size, result))
                    }
                }
                Err(e) => {
                    errors.push(Error::new_with_lookalike(
                        DsgConInvalidField,
                        format!(
                            "Field '{}' was not defined on subsystem '{}'.",
                            child,
                            supertype.raw()
                        ),
                        *loc,
                        false,
                        e.lookalike().map(|v| (&v.0.descriptor[..], v.0.loc)),
                    ));

                    None
                }
            }
        }
    }
}
