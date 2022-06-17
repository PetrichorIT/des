use std::collections::HashMap;

use crate::desugar::{DesugaredParsingResult, ProtoImplSpec, ScndPassTyCtx};
use crate::error::ErrorCode::*;
use crate::parser::*;
use crate::*;

use super::ctx::ScndPassGlobalTyCtx;
use super::first_pass::FstPassResult;

pub fn second_pass<'a>(
    unit: &'a FstPassResult,
    all: &'a HashMap<String, FstPassResult>,
    resolver: &NdlResolver,
) -> DesugaredParsingResult {
    let mut errors = Vec::new();
    let tyctx = ScndPassTyCtx::new_for(unit, all, &mut errors);
    let gtyctx = ScndPassGlobalTyCtx::new(all, &resolver.source_map);

    // Assume that no name collision occured, else dont proceed thing will get funky
    tyctx.check_for_name_collisions(&mut errors);

    let mut result = DesugaredParsingResult {
        asset: unit.asset.clone(),
        includes: Vec::new(),
        modules: Vec::new(),
        networks: Vec::new(),

        errors: Vec::new(),
    };

    // fwd errors
    errors.append(&mut unit.errors.clone());

    //
    // === Map includes ===
    //

    // This mapping may be irrelevant in the future if the generated
    // TySpecContext stabalizes, thus no includes must be resolved by the TYC
    for include in &unit.includes {
        result.includes.push(IncludeSpec {
            loc: include.loc,
            path: include.path.clone(),
        })
    }

    //
    // === Map Modules ===
    //

    for module in &unit.modules {
        let mut module_spec = ModuleSpec::new(module);

        // Resolve ChildModuleDef to ChildModuleSpec
        let mut occupied_c_namespace = Vec::<&LocalDescriptorDef>::new();
        for child in &module.submodules {
            // Issue (001)
            // Added type checking in desugar to prevent redundand checks
            // on expanded macro types.
            if matches!(child.ty, TyDef::Static(_)) {
                // Can ingore dyn types since they are checked later anyway
                let exists = validate_module_ty(
                    child,
                    &tyctx.modules,
                    &gtyctx,
                    &resolver.source_map,
                    &mut errors,
                );

                if !exists {
                    continue;
                }
            }

            let ChildNodeDef {
                loc,
                ty,
                desc,
                proto_impl,
            } = child;

            if let Some(col) = occupied_c_namespace
                .iter()
                .find(|n| n.descriptor == desc.descriptor && n.cluster_bounds_overlap(desc))
            {
                // naming collision.
                errors.push(Error::new(
                    DsgModuleSubmoduleFieldAlreadyDeclared,
                    format!(
                        "Naming collision. Namespaces of '{}' and '{}' collide.",
                        col, desc
                    ),
                    desc.loc,
                    false,
                ));
            }

            occupied_c_namespace.push(desc);

            if let Some((from_id, to_id)) = desc.cluster_bounds {
                // Desugar macro
                for id in from_id..=to_id {
                    module_spec.submodules.push(ChildNodeSpec {
                        loc: *loc,
                        descriptor: format!("{}[{}]", desc.descriptor, id),
                        ty: TySpec::new(ty),
                        proto_impl: proto_impl.as_ref().map(ProtoImplSpec::new),
                    })
                }
            } else {
                // CopyPaste
                module_spec.submodules.push(ChildNodeSpec {
                    loc: *loc,
                    descriptor: desc.descriptor.clone(),
                    ty: TySpec::new(ty),
                    proto_impl: proto_impl.as_ref().map(ProtoImplSpec::new),
                })
            }
        }

        // Resolve connections
        for connection in &module.connections {
            let ConDef {
                loc,
                from,
                channel,
                to,
            } = connection;

            let (f_nodes_len, f_gate_size, from_idents) = match resolve_connection_ident(
                from,
                &module.gates,
                &module.submodules,
                &tyctx,
                &gtyctx,
                &mut errors,
                GateAnnotation::Output,
            ) {
                Some(v) => v,
                None => continue,
            };
            let (t_nodes_len, t_gate_size, to_idents) = match resolve_connection_ident(
                to,
                &module.gates,
                &module.submodules,
                &tyctx,
                &gtyctx,
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
            let channel_spec = match channel {
                Some(channel_ident) => {
                    let link_def = match tyctx.links.iter().find(|link| link.name == *channel_ident)
                    {
                        Some(link_def) => link_def,
                        None => {
                            errors.push(Error::new_ty_missing(
                                DsgConInvalidChannel,
                                format!("No link called '{}' found.", channel_ident),
                                connection.loc,
                                &resolver.source_map,
                                gtyctx.link(channel_ident).map(|link| link.loc),
                            ));

                            continue;
                        }
                    };

                    Some(ChannelSpec::new(link_def))
                }
                None => None,
            };

            for (source, target) in from_idents.into_iter().zip(to_idents.into_iter()) {
                module_spec.connections.push(ConSpec {
                    loc: *loc,

                    source,
                    target,
                    channel: channel_spec.clone(),
                })
            }
        }

        result.modules.push(module_spec);
    }

    //
    // === Network spec ===
    //

    for subsys_def in &unit.subsystems {
        let mut subsystem_spec = SubsystemSpec::new(subsys_def);

        // Issue (001)
        // Defines that tycheck should be done on unexpanded macros

        // Namespace collission
        let occupied_namespaces = Vec::<&LocalDescriptorDef>::new();
        for ChildNodeDef { desc, .. } in &subsys_def.nodes {
            // check collisions.
            if let Some(col) = occupied_namespaces
                .iter()
                .find(|n| n.descriptor == desc.descriptor && n.cluster_bounds_overlap(desc))
            {
                // naming collision.
                errors.push(Error::new(
                    DsgModuleSubmoduleFieldAlreadyDeclared,
                    format!(
                        "Naming collision. Namespaces of '{}' and '{}' collide.",
                        col, desc
                    ),
                    desc.loc,
                    false,
                ));
            }
        }

        // Resolve ChildModuleDef to ChildModuleSpec
        for child in &subsys_def.nodes {
            let (exists, gty) = if !tyctx.modules.iter().any(|m| m.name == child.ty.inner()) {
                // Ty missing
                let global_ty = gtyctx.module(child.ty.inner()).map(|m| m.loc);

                if !tyctx.subsystems.iter().any(|n| n.name == child.ty.inner()) {
                    let global_ty2 = gtyctx.subsystem(child.ty.inner()).map(|m| m.loc);

                    (false, global_ty.or(global_ty2))
                } else {
                    (true, None)
                }
            } else {
                (true, None)
            };

            if !exists {
                errors.push(Error::new_ty_missing(
                    TycNetworkSubmoduleInvalidTy,
                    format!("No module with name '{}' exists in the scope.", child.ty,),
                    child.loc,
                    &resolver.source_map,
                    gty,
                ));

                continue;
            }

            let ChildNodeDef {
                loc,
                ty,
                desc,
                proto_impl,
            } = child;

            // Solved issue:
            // Proto-impl operand order must match child definition order
            // to ensure safe generics.
            let proto_impl = proto_impl
                .as_ref()
                .map(ProtoImplSpec::new)
                .map(|pimpl| pimpl.sorted_according_to(gtyctx.module(ty.inner())));

            if let Some((from_id, to_id)) = desc.cluster_bounds {
                // Desugar macro
                for id in from_id..=to_id {
                    subsystem_spec.nodes.push(ChildNodeSpec {
                        loc: *loc,
                        descriptor: format!("{}[{}]", desc.descriptor, id),
                        ty: TySpec::new(ty),
                        proto_impl: proto_impl.clone(),
                    })
                }
            } else {
                // CopyPaste
                subsystem_spec.nodes.push(ChildNodeSpec {
                    loc: *loc,
                    descriptor: desc.descriptor.clone(),
                    ty: TySpec::new(ty),
                    proto_impl,
                })
            }
        }

        let vec_new = Vec::new();

        // Resolve connections
        for connection in &subsys_def.connections {
            let ConDef {
                loc,
                from,
                channel,
                to,
            } = connection;

            let (f_nodes_len, f_gate_size, from_idents) = match resolve_connection_ident(
                from,
                &vec_new,
                &subsys_def.nodes,
                &tyctx,
                &gtyctx,
                &mut errors,
                GateAnnotation::Output,
            ) {
                Some(v) => v,
                None => continue,
            };
            let (t_nodes_len, t_gate_size, to_idents) = match resolve_connection_ident(
                to,
                &vec_new,
                &subsys_def.nodes,
                &tyctx,
                &gtyctx,
                &mut errors,
                GateAnnotation::Input,
            ) {
                Some(v) => v,
                None => continue,
            };

            // Gurantee that count(from) = count(to)
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
            let channel_spec = match channel {
                Some(channel_ident) => {
                    let link_def = match tyctx.links.iter().find(|link| link.name == *channel_ident)
                    {
                        Some(link_def) => link_def,
                        None => {
                            errors.push(Error::new_ty_missing(
                                DsgConInvalidChannel,
                                format!("No link called '{}' found.", channel_ident),
                                connection.loc,
                                &resolver.source_map,
                                gtyctx.link(channel_ident).map(|link| link.loc),
                            ));
                            continue;
                        }
                    };

                    Some(ChannelSpec::new(link_def))
                }
                None => None,
            };

            for (source, target) in from_idents.into_iter().zip(to_idents.into_iter()) {
                subsystem_spec.connections.push(ConSpec {
                    loc: *loc,

                    source,
                    target,
                    channel: channel_spec.clone(),
                })
            }
        }

        // Resolve export interface

        // for export in &subsys_def.exports {
        //     let ExportDef { loc, module, gate } = export;

        //     // First check gate name collision
        //     if let Some(v) = subsystem_spec
        //         .exports
        //         .iter()
        //         .find(|v| v.gate_ident.ident == *gate)
        //     {
        //         errors.push(Error::new(
        //             DsgExportNamespaceCollision,
        //             format!(
        //                 "Gate naming collision. Export namespace '{}/{}' and '{}' collide.",
        //                 v.node_ident, v.gate_ident.ident, export
        //             ),
        //             *loc,
        //             false,
        //         ));
        //         continue;
        //     }

        //     // Ensure the module handle is correct.
        //     // Note that cluster definitions are forbidden since this will lead to namespace collisions
        //     // thereby this operation can be performed on the specs direcly.

        //     // proto or non proto should no really matter

        //     let mod_spec = subsystem_spec
        //         .nodes
        //         .iter()
        //         .find(|n| n.descriptor == *module);

        //     if let Some(mod_spec) = mod_spec {
        //         // Get child ty (can be done via a gtyctx since the typcoherence is checked seperatoly)
        //         // TODO: Support subsystem export as well (using poll based structure)
        //         let mod_ty = gtyctx.module(&mod_spec.ty.inner());
        //         if let Some(mod_ty) = mod_ty {
        //             // Extract gate by name
        //             // Firstly we will only support export of full clusters.
        //             let gate_def = mod_ty.gates.iter().find(|g| g.name == *gate);
        //             if let Some(gate_def) = gate_def {
        //                 subsystem_spec.exports.push(ExportSpec {
        //                     loc: *loc,
        //                     node_ident: module.clone(),
        //                     gate_ident: GateSpec::new(gate_def),
        //                 });
        //             } else {
        //                 errors.push(Error::new(
        //                     DsgExportInvalidGateIdent,
        //                     format!(
        //                         "Unknown gate ident '{}' referenced for export on module '{}'.",
        //                         module, mod_ty.name
        //                     ),
        //                     *loc,
        //                     false,
        //                 ))
        //             }
        //         }
        //     } else {
        //         errors.push(Error::new(
        //             DsgExportInvalidNodeIdent,
        //             format!("Unknown module '{}' referenced for export.", module),
        //             *loc,
        //             false,
        //         ))
        //     }
        // }

        result.networks.push(subsystem_spec);
    }

    result.errors = errors;
    result
}
fn resolve_connection_ident(
    ident: &ConNodeIdent,
    local_gates: &[GateDef],
    child_modules: &[ChildNodeDef],
    tyctx: &ScndPassTyCtx<'_>,
    gtyctx: &ScndPassGlobalTyCtx<'_>,
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
                                ident, sub_module_ty.name
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
