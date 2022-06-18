use crate::{
    error::*,
    parser::{ChildNodeDef, ConNodeIdent, GateDef, Ident},
    GateAnnotation,
};

use super::{
    ctx::{GlobalTyDefContext, TyComposeContext, TyDefContext},
    specs::*,
};

pub(crate) fn resolve_connection_ident(
    ident: &ConNodeIdent,
    local_gates: &[GateDef],
    child_modules: &[ChildNodeDef],
    tyctx: &TyDefContext<'_>,
    gtyctx: &GlobalTyDefContext<'_>,
    errors: &mut Vec<Error>,
    expected_type: GateAnnotation,
) -> Option<(usize, usize, Vec<ConSpecNodeIdent>)> {
    let global_loc = ident.loc();

    let mut result = Vec::new();
    match ident {
        ConNodeIdent::Local { loc, ident } => {
            let global_ident = ident;
            // Local can only reference a modules gate
            let gate = match local_gates
                .iter()
                .find(|gate| gate.name == ident.raw_ident())
            {
                Some(gate) => gate,
                None => {
                    errors.push(Error::new(
                        DsgConInvalidLocalGateIdent,
                        format!("No local gate cluster '{}' exists on this module.", ident),
                        *loc,
                        false,
                    ));
                    return None;
                }
            };

            if gate.size < 1 {
                // This should be catched by the [tychk]

                // errors.push(Error::new(
                //     DsgConInvalidGateSize,
                //     format!("Gate size 0 is invalid for gate cluster '{}'.", ident),
                //     *loc,
                //     false,
                // ));

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

            let submod_def = match child {
                // Can either describe a primitiv or a clustered group that
                // will not be indexed.
                Ident::Direct { ident } => {
                    child_modules.iter().find(|m| m.desc.descriptor == *ident)
                }
                Ident::Clustered { ident, index } => child_modules.iter().find(|m| {
                    m.desc.descriptor == *ident
                        && m.desc.cluster_bounds.is_some()
                        && m.desc.cluster_bounds_contain(*index)
                }),
            };

            if let Some(submod_def) = submod_def {
                // fetch module ty
                // this can be done outside the following if-else
                // since a cluster-definition shares the same ty
                let sub_module_ty = match tyctx.module_or_proto(&submod_def.ty) {
                    Some(sub_module) => sub_module,
                    None => {
                        errors.push(Error::new_ty_missing(
                            DsgConInvalidLocalGateIdent,
                            format!(
                                "No module '{}' exists as requested for child module '{}'.",
                                submod_def.ty, child
                            ),
                            *loc,
                            gtyctx.source_map(),
                            gtyctx
                                .module(submod_def.ty.inner())
                                .map(|module| module.loc),
                        ));
                        return None;
                    }
                };

                // fetch gate
                let gate_def = match ident {
                    Ident::Direct { ident } => sub_module_ty
                        .gates
                        .iter()
                        .find(|g| g.name == *ident)
                        .map(|g| (g, None)),
                    Ident::Clustered { ident, index } => sub_module_ty
                        .gates
                        .iter()
                        .find(|g| g.name == *ident)
                        .map(|g| (g, Some(*index))),
                };

                // fetch gate
                let (gate_def, gate_def_cindex) = match gate_def {
                    Some(gate_def) => gate_def,
                    None => {
                        errors.push(Error::new(
                            DsgConInvalidLocalGateIdent,
                            format!(
                                "No local gate cluster '{}' exists on module '{}'.",
                                ident,
                                sub_module_ty.ident.raw()
                            ),
                            *loc,
                            false,
                        ));
                        return None;
                    }
                };

                if gate_def.size < 1 {
                    // This should be catched by [tychk]

                    // errors.push(Error::new(
                    //     DsgConInvalidGateSize,
                    //     format!("Gate size 0 is invalid for gate cluster '{}'.", ident),
                    //     *loc,
                    //     false,
                    // ));

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
            } else {
                errors.push(Error::new(
                    DsgConInvalidField,
                    format!("Invalid field '{}'.", child),
                    *loc,
                    false,
                ));

                None
            }
        }
    }
}

pub(crate) fn resolve_connection_ident_compose(
    ident: &ConNodeIdent,

    child_modules: &[ChildNodeSpec],
    tyctx: &TyComposeContext<'_>,
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

            let submod_def = match child {
                // Can either describe a primitiv or a clustered group that
                // will not be indexed.
                Ident::Direct { ident } => child_modules.iter().find(|m| m.descriptor == *ident),
                Ident::Clustered { ident, index } => child_modules
                    .iter()
                    .find(|m| m.descriptor == format!("{}[{}]", ident, index)),
                /*ident
                && m.desc.cluster_bounds.is_some()
                && m.desc.cluster_bounds_contain(*index) */
            };

            if let Some(submod_def) = submod_def {
                // fetch module ty
                // this can be done outside the following if-else
                // since a cluster-definition shares the same ty

                let valid_ident = submod_def.ty.valid_ident();
                if valid_ident.is_none() {
                    todo!()
                }

                let sub_module_ty = match tyctx.module_or_proto(valid_ident.unwrap()) {
                    Some(sub_module) => sub_module,
                    None => {
                        errors.push(Error::new_ty_missing(
                            DsgConInvalidLocalGateIdent,
                            format!(
                                "No module '{}' exists as requested for child module '{}'.",
                                submod_def.ty, child
                            ),
                            *loc,
                            tyctx.source_map(),
                            tyctx
                                .module(submod_def.ty.valid_ident().unwrap())
                                .map(|module| module.loc),
                        ));
                        return None;
                    }
                };

                // fetch gate
                let gate_def = match ident {
                    Ident::Direct { ident } => sub_module_ty
                        .gates
                        .iter()
                        .find(|g| g.ident == *ident)
                        .map(|g| (g, None)),
                    Ident::Clustered { ident, index } => sub_module_ty
                        .gates
                        .iter()
                        .find(|g| g.ident == *ident)
                        .map(|g| (g, Some(*index))),
                };

                // fetch gate
                let (gate_def, gate_def_cindex) = match gate_def {
                    Some(gate_def) => gate_def,
                    None => {
                        errors.push(Error::new(
                            DsgConInvalidLocalGateIdent,
                            format!(
                                "No local gate cluster '{}' exists on module '{}'.",
                                ident,
                                sub_module_ty.ident.raw()
                            ),
                            *loc,
                            false,
                        ));
                        return None;
                    }
                };

                if gate_def.size < 1 {
                    // This should be catched by [tychk]

                    // errors.push(Error::new(
                    //     DsgConInvalidGateSize,
                    //     format!("Gate size 0 is invalid for gate cluster '{}'.", ident),
                    //     *loc,
                    //     false,
                    // ));

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
            } else {
                errors.push(Error::new(
                    DsgConInvalidField,
                    format!("Invalid field '{}'.", child),
                    *loc,
                    false,
                ));

                None
            }
        }
    }
}
