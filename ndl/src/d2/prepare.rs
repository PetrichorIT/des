use super::{
    ctx::TyDefContext,
    specs::{
        ChannelSpec, ChildNodeSpec, ConSpec, GateSpec, IncludeSpec, ModuleSpec, ProtoImplSpec,
        SubsystemSpec, TyPath, TySpec,
    },
};
use crate::{
    error::*,
    parser::{
        AliasDef, ChildNodeDef, ConDef, ConNodeIdent, ExportDef, GateDef, LinkDef, ModuleDef,
        ParsingResult, SubsystemDef, TyDef,
    },
    AssetDescriptor, Error, Loc, NdlResolver,
};
use std::fmt::Display;

pub type PreparedInclude = IncludeSpec;
pub type PreparedLink = LinkDef;
pub type PreparedPrototype = ModuleSpec<ConNodeIdent>;
pub type PrepareModule = ModuleSpec<ConNodeIdent>;
pub type PrepareSubsystem = SubsystemSpec<ConNodeIdent>;

#[derive(Debug)]
pub struct PreparedUnit {
    pub asset: AssetDescriptor,
    pub loc: Loc,

    pub includes: Vec<PreparedInclude>,
    pub links: Vec<PreparedLink>,
    pub prototypes: Vec<PreparedPrototype>,
    pub modules: Vec<PrepareModule>,
    pub subsystems: Vec<PrepareSubsystem>,

    pub errors: Vec<Error>,
}

impl Display for PreparedUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PreparedUnit {{")?;

        if !self.includes.is_empty() {
            writeln!(f, "<< includes >>")?;
            for include in &self.includes {
                writeln!(f, "- {}", include)?;
            }
        }

        if !self.links.is_empty() {
            writeln!(f, "<< links >>")?;
            for link in &self.links {
                writeln!(f, "- {}", link)?;
            }
        }

        if !self.prototypes.is_empty() {
            writeln!(f, "<< prototypes >>")?;
            for prototype in &self.prototypes {
                writeln!(f, "{}", prototype)?
            }
        }

        if !self.modules.is_empty() {
            writeln!(f, "<< modules >>")?;
            for module in &self.modules {
                writeln!(f, "{}", module)?
            }
        }

        if !self.subsystems.is_empty() {
            writeln!(f, "<< networks >>")?;
            for network in &self.subsystems {
                writeln!(f, "{}", network)?
            }
        }

        writeln!(f, "<< errors >>")?;
        writeln!(f, "-> {} errors", self.errors.len())?;

        write!(f, "}}")
    }
}

///
/// This function will return the an object guranteeing the following things:
///
/// (1) - All includes are validated or removed by emiting an error
/// (2) - All links have sensable values (or emit an error) & no local name collision
/// (3) - All modules, prototypes and subsystems have no local name collisions
/// (4) - All [gates] have sensable size and no local name collision
/// (5) - All [nodes] / [submodules] have sensable bounds and no local name colliosn
/// (6) - All [nodes] / [submodules] with static types have types that are either in the local
///        type context or an error is emitted. If neither a local ty nor a global ty
///        exists the corresponding ChildNodeSpec is flaged with a ty `None``
/// (7) - All [nodes] / [submodules] with dynamic types are checked to have
///        the corresponding prototypes in scope. If not they are flagged aswell
/// (8) - All [connections] have valid channels or dummys which emit an error.
/// (9) - All [exports] are check to point to a valid child. Gate sizes cannot be infered yet.
/// (10) - All aliases are replaced by their prototype impl, or an error is emitted.
///
/// ty: enum { Static(Option<String>), Dynamic(Option<String>) }
///
pub fn prepare(unit: &ParsingResult, resolver: &NdlResolver) -> PreparedUnit {
    let ParsingResult {
        asset,
        loc,
        links,
        modules,
        prototypes,
        subsystems,
        aliases,
        ..
    } = unit;

    let mut r = PreparedUnit {
        asset: asset.clone(),
        loc: *loc,

        includes: Vec::new(),
        links: Vec::new(),
        prototypes: Vec::new(),
        modules: Vec::new(),
        subsystems: Vec::new(),

        errors: Vec::new(),
    };

    let tyctx = TyDefContext::new_for(unit, resolver, &mut r.errors);
    let gtyctx = resolver.gtyctx_def();

    // (1) All includes are valid and thus sensable
    r.includes = tyctx
        .included
        .iter()
        .map(|(asset, loc)| IncludeSpec {
            path: asset.clone(),
            loc: *loc,
        })
        .collect();

    // (2) All links have valid names an no name collisions
    let mut local_link_namespace: Vec<&LinkDef> = Vec::with_capacity(links.len());
    for link in links {
        if let Some(other) = local_link_namespace.iter().find(|l| (*l).name == link.name) {
            r.errors.push(Error::new(
                DsgLinkNamespaceCollision,
                format!(
                    "Namespace collsion. Allready defined a link with name '{}'.",
                    other.name
                ),
                link.loc,
                false,
            ));
            // This link can be ignored since an error has been emitted by the
            // an another link under the same name ensures that callls to this link
            // are still valid
            continue;
        }

        local_link_namespace.push(link);

        // Check values
        // Emit error on stupid values, but keep the link eitherway to prevent transient errors
        if link.jitter < 0.0 {
            r.errors.push(Error::new(
                DsgLinkInvalidJitter,
                format!(
                    "Invalid jitter value '{}' on link '{}'. Jitter must be positive or null.",
                    link.jitter, link.name
                ),
                link.loc,
                false,
            ))
        }
        if link.latency < 0.0 {
            r.errors.push(Error::new(
                DsgLinkInvalidLatency,
                format!(
                    "Invalid latency value '{}' on link '{}'. Latency must be positive or null.",
                    link.latency, link.name
                ),
                link.loc,
                false,
            ))
        }
        if link.bitrate == 0 {
            r.errors.push(Error::new(
                DsgLinkInvalidBitrate,
                format!(
                    "Invalid birate value '{}' on link '{}'. Birrate must be real positive.",
                    link.bitrate, link.name
                ),
                link.loc,
                false,
            ))
        }

        r.links.push(link.clone())
    }

    // Evaluate aliases (as defs)
    let aliases = aliases
        .into_iter()
        .map(|alias| {
            let AliasDef {
                loc,
                name,
                prototype,
            } = alias;

            // search for proto
            let proto = tyctx.prototypes.iter().find(|p| p.name == *prototype);

            if let Some(proto) = proto {
                let mut proto: ModuleDef = (*proto).clone();

                proto.is_prototype = false;
                proto.loc = *loc;
                proto.name = name.clone();
                proto.derived_from = Some(prototype.to_string());

                Some(proto)
            } else {
                let g_proto = gtyctx.prototype(prototype).map(|m| m.loc);
                let g_module_or_proto = gtyctx.module(prototype).map(|m| m.loc).is_some();

                let module_as_proto = g_module_or_proto && g_proto.is_none();

                r.errors.push(Error::new_ty_missing(
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

                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    // Iterate over the modules to fulfill (3)...(8) and perpare (10)
    let mut local_module_namespace: Vec<&ModuleDef> = Vec::new();
    let mut local_prototype_namespace: Vec<&ModuleDef> = Vec::new();
    for module in modules.iter().chain(aliases.iter()).chain(prototypes) {
        // Fetch namespace based on is_prototype
        let namespace = if module.is_prototype {
            &mut local_prototype_namespace
        } else {
            &mut local_module_namespace
        };

        // (3) Check collison.
        if let Some(other) = namespace.iter().find(|m| (*m).name == module.name) {
            r.errors.push(Error::new(
                DsgModuleNamespaceCollision,
                format!(
                    "Namespace collsion. Allready defined a module with name '{}'.",
                    other.name
                ),
                module.loc,
                false,
            ));

            continue;
        }

        namespace.push(module);

        let ModuleDef {
            loc,
            name,
            submodules,
            gates,
            connections,
            is_prototype,
            ..
        } = module;

        let mut spec = PrepareModule::new(module, asset.clone());

        // (4) Check that gates make sense
        let mut local_gate_namespace: Vec<&GateDef> = Vec::new();
        for gate in gates {
            if let Some(other) = local_gate_namespace.iter().find(|g| (*g).name == gate.name) {
                r.errors.push(Error::new(
                    DsgGateNamespaceCollision,
                    format!(
                        "Namespace collision. Allready defined a gate with name '{}' on module '{}'.",
                        other.name, name
                    ),
                    gate.loc,
                    false,
                ));

                // This may lead to problems but that cant be solved.
                continue;
            }

            local_gate_namespace.push(gate);

            if gate.size == 0 {
                r.errors.push(Error::new(
                    DsgGateNullSize,
                    format!("Cannot create gate '{}' with size 0.", gate.name),
                    gate.loc,
                    false,
                ));
                // TODO: Maybe push further along with marker
                // could be nice
                continue;
            }

            spec.gates.push(GateSpec::new(gate))
        }
        drop(local_gate_namespace);

        // (5..7) Work on local nodes.
        let mut local_submod_namespace: Vec<&ChildNodeDef> = Vec::new();
        for submodule in submodules {
            // (5) Namespace checks
            if let Some(other) = local_submod_namespace.iter().find(|s| {
                (*s).desc.descriptor == submodule.desc.descriptor
                    && s.desc.cluster_bounds_overlap(&submodule.desc)
            }) {
                r.errors.push(Error::new(
                    DsgSubmoduleNamespaceCollision,
                    format!(
                        "Namespace collision. Allready defined a submodule with name '{}' on module '{}'.",
                        other.desc, name
                    ),
                    submodule.loc,
                    false,
                ));

                // This may lead to problems but that cant be solved.
                continue;
            }

            local_submod_namespace.push(submodule);
            let ChildNodeDef {
                loc: submod_loc,
                ty,
                desc,
                proto_impl,
            } = submodule;

            // (5) Checks descriptor bounds
            if let Some((from, to)) = desc.cluster_bounds {
                if from >= to {
                    r.errors.push(Error::new(
                        DsgSubmoduleInvalidBound,
                        format!(
                            "Cannot define submodule '{}' with invalid bound {}..{}",
                            desc.descriptor, from, to
                        ),
                        *submod_loc,
                        false,
                    ));
                    // The child object does not event exist.
                    continue;
                }
            }

            // (6) and (7) resolve type imports and get path
            let ty_spec = match ty {
                // (6) Static types
                TyDef::Static(ref s) => {
                    let exists = tyctx.modules.iter().any(|m| &m.name == s);
                    if !exists {
                        let gty = gtyctx.module(s).map(|g| g.loc);

                        r.errors.push(Error::new_ty_missing(
                            DsgSubmoduleMissingTy,
                            format!("No type '{}' found.", s),
                            *submod_loc,
                            &resolver.source_map,
                            gty,
                        ));

                        if let Some(gty) = gty {
                            TySpec::Static(TyPath::OutOfScope(s.clone(), gty))
                        } else {
                            TySpec::Static(TyPath::InScope(s.clone()))
                        }
                    } else {
                        TySpec::Static(TyPath::InScope(s.clone()))
                    }
                }
                // (7) Dynamic types
                TyDef::Dynamic(ref s) => {
                    if *is_prototype {
                        // Cannot yet support nested proto
                        //i think, maybe we can but there are no tests yet
                        r.errors.push(Error::new(
                            DsgSubmoduleNestedProto,
                            format!("Field '{}' cannot be generic over '{}' since '{}' is already a prototype.", desc.descriptor, s, name),
                            *submod_loc,
                            false
                        ));
                        continue;
                    }

                    let exists = tyctx.prototypes.iter().any(|p| &p.name == s);
                    if !exists {
                        let g_proto = gtyctx.prototype(s).map(|g| g.loc);
                        let g_module = gtyctx.module(s).map(|m| m.loc).is_some();

                        let module_as_proto = g_module && g_proto.is_none();

                        r.errors.push(Error::new_ty_missing(
                            DsgInvalidPrototypeAtSome,
                            if module_as_proto {
                                format!(
                                    "No prototype called '{0}' found. Module '{0}' is no prototype.",
                                    s
                                )
                            } else {
                                format!("No prototype called '{}' found.", s)
                            },
                            submodule.loc,
                            &resolver.source_map,
                            g_proto,
                        ));

                        if let Some(g_proto) = g_proto {
                            TySpec::Dynamic(TyPath::OutOfScope(s.clone(), g_proto))
                        } else {
                            TySpec::Dynamic(TyPath::InScope(s.clone()))
                        }
                    } else {
                        TySpec::Dynamic(TyPath::InScope(s.clone()))
                    }
                }
            };

            // NOTE
            // .. that connections are not checked here.
            // .. this will be done in a graph later.

            // Generate actual specs
            if let Some((from_id, to_id)) = desc.cluster_bounds {
                // Desugar macro
                for id in from_id..=to_id {
                    spec.submodules.push(ChildNodeSpec {
                        loc: *loc,
                        descriptor: format!("{}[{}]", desc.descriptor, id),
                        ty: ty_spec.clone(),
                        proto_impl: proto_impl.as_ref().map(ProtoImplSpec::new),
                    })
                }
            } else {
                // CopyPaste
                spec.submodules.push(ChildNodeSpec {
                    loc: *loc,
                    descriptor: desc.descriptor.clone(),
                    ty: ty_spec,
                    proto_impl: proto_impl.as_ref().map(ProtoImplSpec::new),
                })
            }
        }
        drop(local_submod_namespace);

        // (8) Connection with valid channel
        for con in connections {
            let ConDef {
                loc,
                channel,
                from,
                to,
            } = con;

            // (8) Check channels
            let channel_spec = if let Some(ref channel) = channel {
                match tyctx.links.iter().find(|l| *l.name == *channel) {
                    Some(link) => Some(ChannelSpec::new(link)),
                    None => {
                        // Emit error
                        let glink = gtyctx.link(channel).map(|l| l.loc);
                        r.errors.push(Error::new_ty_missing(
                            DsgConInvalidChannel,
                            format!("Could not find link '{}' in scope.", channel),
                            *loc,
                            &resolver.source_map,
                            glink,
                        ));
                        Some(ChannelSpec::dummy())
                    }
                }
            } else {
                None
            };

            let con = ConSpec {
                loc: *loc,
                source: from.clone(),
                target: to.clone(),
                channel: channel_spec,
            };

            spec.connections.push(con)
        }

        if *is_prototype {
            r.modules.push(spec)
        } else {
            r.modules.push(spec)
        }
    }

    // Iterate over the subsystems to fulfill (3)...(9)
    let mut local_subsystem_namespace: Vec<&SubsystemDef> = Vec::new();
    for subsystem in subsystems {
        // Fetch namespace based on is_prototype
        let namespace = &mut local_subsystem_namespace;

        // (3) Check collison.
        if let Some(other) = namespace.iter().find(|s| (*s).name == subsystem.name) {
            r.errors.push(Error::new(
                DsgModuleNamespaceCollision,
                format!(
                    "Namespace collsion. Allready defined a subsystem with name '{}'.",
                    other.name
                ),
                subsystem.loc,
                false,
            ));

            continue;
        }

        namespace.push(subsystem);

        let SubsystemDef {
            loc,
            name,
            nodes,
            connections,
            exports,
            ..
        } = subsystem;

        let mut spec = PrepareSubsystem::new(subsystem);

        // Work on local nodes
        let mut local_nodes_namespace: Vec<&ChildNodeDef> = Vec::new();
        for node in nodes {
            // (5) Namespace checks
            if let Some(other) = local_nodes_namespace.iter().find(|s| {
                (*s).desc.descriptor == node.desc.descriptor
                    && s.desc.cluster_bounds_overlap(&node.desc)
            }) {
                r.errors.push(Error::new(
                    DsgSubmoduleNamespaceCollision,
                    format!(
                        "Namespace collision. Allready defined a node with name '{}' on subsystem '{}'.",
                        other.desc, name
                    ),
                    node.loc,
                    false,
                ));

                // This may lead to problems but that cant be solved.
                continue;
            }

            local_nodes_namespace.push(node);
            let ChildNodeDef {
                loc: submod_loc,
                ty,
                desc,
                proto_impl,
            } = node;

            // (5) Checks descriptor bounds
            if let Some((from, to)) = desc.cluster_bounds {
                if from >= to {
                    r.errors.push(Error::new(
                        DsgSubmoduleInvalidBound,
                        format!(
                            "Cannot define node '{}' with invalid bound {}..{}",
                            desc.descriptor, from, to
                        ),
                        *submod_loc,
                        false,
                    ));
                    // The child object does not event exist.
                    continue;
                }
            }

            // (6) and (7) resolve type imports and get path
            let ty_spec = match ty {
                // (6) Static types
                TyDef::Static(ref s) => {
                    let exists = tyctx.modules.iter().any(|m| &m.name == s)
                        || tyctx.subsystems.iter().any(|sys| &sys.name == s);
                    if !exists {
                        let gty = gtyctx
                            .module(s)
                            .map(|g| (g.loc))
                            .or(gtyctx.subsystem(s).map(|s| s.loc));

                        r.errors.push(Error::new_ty_missing(
                            DsgSubmoduleMissingTy,
                            format!("No type '{}' found.", s),
                            *submod_loc,
                            &resolver.source_map,
                            gty,
                        ));

                        if let Some(gty) = gty {
                            TySpec::Static(TyPath::OutOfScope(s.clone(), gty))
                        } else {
                            TySpec::Static(TyPath::InScope(s.clone()))
                        }
                    } else {
                        TySpec::Static(TyPath::InScope(s.clone()))
                    }
                }
                // (7) Dynamic types
                TyDef::Dynamic(ref s) => {
                    // TODO:
                    // Does is make sense to allow proto on subsystem
                    // subsys A { nodes: some Watcher }
                    // Could be unfunny bc derive.
                    let exists = tyctx.prototypes.iter().any(|p| &p.name == s);
                    if !exists {
                        let g_proto = gtyctx.prototype(s).map(|g| g.loc);
                        let g_module = gtyctx.module(s).map(|m| m.loc).is_some();

                        let module_as_proto = g_module && g_proto.is_none();

                        r.errors.push(Error::new_ty_missing(
                            DsgInvalidPrototypeAtSome,
                            if module_as_proto {
                                format!(
                                    "No prototype called '{0}' found. Module '{0}' is no prototype.",
                                    s
                                )
                            } else {
                                format!("No prototype called '{}' found.", s)
                            },
                            node.loc,
                            &resolver.source_map,
                            g_proto,
                        ));

                        if let Some(g_proto) = g_proto {
                            TySpec::Dynamic(TyPath::OutOfScope(s.clone(), g_proto))
                        } else {
                            TySpec::Dynamic(TyPath::InScope(s.clone()))
                        }
                    } else {
                        TySpec::Dynamic(TyPath::InScope(s.clone()))
                    }
                }
            };

            // NOTE
            // .. that connections are not checked here.
            // .. this will be done in a graph later.

            // Generate actual specs
            if let Some((from_id, to_id)) = desc.cluster_bounds {
                // Desugar macro
                for id in from_id..=to_id {
                    spec.nodes.push(ChildNodeSpec {
                        loc: *loc,
                        descriptor: format!("{}[{}]", desc.descriptor, id),
                        ty: ty_spec.clone(),
                        proto_impl: proto_impl.as_ref().map(ProtoImplSpec::new),
                    })
                }
            } else {
                // CopyPaste
                spec.nodes.push(ChildNodeSpec {
                    loc: *loc,
                    descriptor: desc.descriptor.clone(),
                    ty: ty_spec,
                    proto_impl: proto_impl.as_ref().map(ProtoImplSpec::new),
                })
            }
        }
        drop(local_nodes_namespace);

        // (8) Connections with valid channels
        for con in connections {
            let ConDef {
                loc,
                channel,
                from,
                to,
            } = con;

            // (8) Check channels
            let channel_spec = if let Some(ref channel) = channel {
                match tyctx.links.iter().find(|l| *l.name == *channel) {
                    Some(link) => Some(ChannelSpec::new(link)),
                    None => {
                        // Emit error
                        let glink = gtyctx.link(channel).map(|l| l.loc);
                        r.errors.push(Error::new_ty_missing(
                            DsgConInvalidChannel,
                            format!("Could not find link '{}' in scope.", channel),
                            *loc,
                            &resolver.source_map,
                            glink,
                        ));
                        Some(ChannelSpec::dummy())
                    }
                }
            } else {
                None
            };

            let con = ConSpec {
                loc: *loc,
                source: from.clone(),
                target: to.clone(),
                channel: channel_spec,
            };

            spec.connections.push(con)
        }

        let mut local_export_namespace: Vec<&ExportDef> = Vec::new();
        for export in exports {
            if let Some(other) = local_export_namespace
                .iter()
                .find(|e| e.gate == export.gate)
            {
                r.errors.push(Error::new(
                    DsgExportNamespaceCollision,
                    format!("Name collision. Gate '{}' cannot be exported, since '{}' has been exported earlier.", export, other),
                    export.loc,
                    false
                ));
                // Could make problems but i dont care
                continue;
            }

            local_export_namespace.push(export);
        }
        drop(local_export_namespace);

        r.subsystems.push(spec)
    }

    r
}
