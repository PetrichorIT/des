use fxhash::{FxHashMap, FxHashSet};
use std::iter::once;

pub mod def;
pub mod error;
pub mod tree;

use def::{
    ConnectionDef, ConnectionEndpointDef, Def, FieldDef, GateDef, Kardinality, ModuleDef,
    ModuleGenericsDef, TypClause,
};
use error::{Error, ErrorKind, Result};
use tree::{
    Connection, ConnectionEndpoint, ConnectionEndpointAccessor, Gate, Link, Network, Node,
    Submodule, Symbol,
};

/// Transforms the network definition into a concrete node tree.
///
/// (0)
/// Therefore all modules are parsed in order of dependencies. This means, that
/// the a module is only transformed if all its dependencies are already parsed.
/// The common dependencies are:
/// - the 'inherit' symbol, if exsitent
/// - all submodule types, including their generic replacements, except those defined by generics
/// - bounds for the generics
///
/// Failure to find a valid processing order of all modules will fail the transfomation.
///
/// (1)
/// Thereforth module definitions are parsed into base-nodes. These base nodes already contain their inlined submodules,
/// but generics are not yet resolved, thus the interface is used as a placeholder.
///
/// When resolving a submodule is resolved there can be three scenaios:
/// - The defined typ is a concrete type without generic args. The the node can be initialized with the already parsed
///   instance of this node type. Just clone it.
/// - The defined typ is a concrete type with generic args. The base-node must be cloned, but all placeholder must be
///   initialized with concrete types. (TODO: nested generics for concrete replacement)
/// - The defined type is a generic argument. A node of the interface type must be used as a placeholder, but its symbol
///   must be changed to the generics binding name.
///
/// # Errors
///
/// Returns an error if the `Def` does not describe a valid network.
pub fn transform(def: &Def) -> Result<Network> {
    let mut modules = def
        .modules
        .iter()
        .map(|(ident, def)| ((ident, def), def.required_symbols(ident)))
        .collect::<Vec<_>>();

    // All values 0..idx are allready resolvable in their position
    let mut idx = 0;
    let mut provider_set = FxHashSet::default();
    while idx < modules.len() {
        let Some(next) = modules[idx..]
            .iter()
            .position(|(_, deps)| deps.iter().all(|symbol| provider_set.contains(*symbol)))
        else {
            return Err(ErrorKind::UnresolvableDependency(
                modules[idx..]
                    .iter()
                    .map(|((ident, _), _)| ident.ident.clone())
                    .collect::<Vec<_>>(),
            )
            .into());
        };
        // relative iterator
        let next = next + idx;

        modules.swap(idx, next);
        provider_set.insert(modules[idx].0 .0.ident.clone());
        idx += 1;
    }

    let links: FxHashMap<String, Link> = def.links.clone();

    let mut archetypes = FxHashMap::default();
    for ((ident, module), _) in modules {
        let archetyp = transform_module(ident, module, &archetypes, &links)
            .map_err(|e| e.span_module(&ident.ident))?;
        archetypes.insert(ident.ident.clone(), archetyp);
    }

    archetypes
        .remove(&def.entry)
        .map(|v| v.0)
        .ok_or_else(|| ErrorKind::UnknownModule(def.entry.clone()).into())
}

//
fn transform_module(
    ident: &TypClause<ModuleGenericsDef>,
    def: &ModuleDef,
    nodes: &FxHashMap<String, (Node, Vec<ModuleGenericsDef>)>,
    links: &FxHashMap<String, Link>,
) -> Result<(Node, Vec<ModuleGenericsDef>)> {
    // (0) Ensure that the ident is valid. Therefore all generic argument must not collide.
    for i in 0..ident.args.len() {
        for j in (i + 1)..ident.args.len() {
            if ident.args[i].binding == ident.args[j].binding {
                return Err(ErrorKind::SymbolAlreadyDefined(ident.args[j].to_string()).into());
            }
        }
    }

    // (1) Transform gates. This is a NOP with only validity changes
    let mut gates = transform_gates(&ident.ident, &def.gates)?;

    // (2) Transform submodules. This might resolve generics, thus global nodes are required.
    let mut submodules = transform_submodules(ident, &def.submodules, nodes)?;

    // (3) Defer connections computation after inherit, to use known gates.
    let mut connections = Vec::new();

    // (4) Inherit known symbols and definitions
    if let Some(ref parent) = def.inherit {
        let (arch, _) = nodes
            .get(parent)
            .expect("unreachable: parse order should guarantee, that all required modules are already parsed");
        gates.extend(arch.gates.iter().cloned());
        submodules.extend(arch.submodules.iter().cloned());
        connections.extend(arch.connections.iter().cloned());
    }

    // (5) Parse connections with elsewise fully defined node. If the node is generic, connections on the generic node
    // must work with the placeholders only
    let connections =
        transform_connections(connections, &def.connections, &submodules, &gates, links)?;

    Ok((
        Node {
            typ: Symbol::from(&ident.ident),
            gates,
            submodules,
            connections,
        },
        ident.args.clone(),
    ))
}

/// Transform the gates.
/// - Check that no gate clusterof size 0 is defined.
fn transform_gates(ident: &str, defs: &[GateDef]) -> Result<FxHashSet<Gate>> {
    defs.iter().try_for_each(|v| {
        if v.kardinality == Kardinality::Cluster(0) {
            Err(
                Error::from(ErrorKind::InvalidGate(ident.to_string(), v.ident.clone()))
                    .span_gate(&v.to_string()),
            )
        } else {
            Ok(())
        }
    })?;
    Ok(defs.iter().cloned().collect())
}

/// Transform the submodules
/// - check that no module cluster of size 0 is defined.
/// -
fn transform_submodules(
    ident: &TypClause<ModuleGenericsDef>,
    defs: &FxHashMap<FieldDef, TypClause<String>>,
    nodes: &FxHashMap<String, (Node, Vec<ModuleGenericsDef>)>,
) -> Result<Vec<Submodule>> {
    defs.iter()
        .map(|(field, typ)| {
            transform_submodule(field, ident, typ, nodes)
                .map_err(|e| e.span_submodule(&field.to_string()))
        })
        .collect::<Result<Vec<_>>>()
}

fn transform_submodule(
    field: &FieldDef,
    ident: &TypClause<ModuleGenericsDef>,
    typ: &TypClause<String>,
    nodes: &std::collections::HashMap<
        String,
        (Node, Vec<ModuleGenericsDef>),
        std::hash::BuildHasherDefault<fxhash::FxHasher>,
    >,
) -> std::prelude::v1::Result<Submodule, error::Error> {
    if field.kardinality == Kardinality::Cluster(0) {
        return Err(ErrorKind::InvalidSubmodule(ident.to_string(), field.ident.clone()).into());
    }

    if typ.args.is_empty() {
        // Submodule has no Args defined: two cases are possible
        // (a) concrete global type
        // (b) local generic

        // Transform the type with the local scope context. NOP for concrete global types.
        // Generic types are replaced by their interface.
        let typ_ident_processed = ident.inner_ty_to_outer_ty(&typ.ident);

        // Get the node definition for either the concrete type or the interface.
        let (node, node_generic_requirements) = nodes.get(typ_ident_processed)
            .expect("unreachable: parse order should guarantee, that all required modules are already parsed");
        let mut node = node.clone();
        // Change the symbol to the local binding name. NOP for concrete types. Rename for generics.
        node.typ = Symbol::from(&typ.ident);

        // Check that provided type does not require generics
        if !node_generic_requirements.is_empty() {
            return Err(ErrorKind::InvalidTypStatement(
                typ.clone(),
                node_generic_requirements.clone(),
            )
            .into());
        }

        Ok(Submodule {
            name: field.clone(),
            typ: node,
        })
    } else {
        // TypDef has generics attached: only one case
        // (c) Subtype is generic with a concretisation here

        // Get base-node for the generic type.
        let (mut node, req_args) = nodes.get(&typ.ident)
            .expect("unreachable: parse order should guarantee, that all required modules are already parsed")
            .clone();

        // Check that the assigment matches all required generics
        if req_args.len() != typ.args.len() {
            return Err(ErrorKind::InvalidTypStatement(typ.clone(), req_args).into());
        }

        // Replace the placeholder with their concrete data type
        // - check that replacements are concrete types, with no generics themselves
        // - check that replacement conforms to interface
        for (i, generic_binding) in req_args.iter().enumerate() {
            // The assigment of the local submodule
            let concrete_replacement_name = &typ.args[i];

            // Get the concrete type, used as a replacement
            let (concrete_replacement, replacement_deps) = nodes.get(concrete_replacement_name)
                .expect("unreachable: parse order should guarantee, that all required modules are already parsed");
            assert!(replacement_deps.is_empty());

            // Ensure that the replacement conforms to all required parameters
            let interface = nodes.get(&generic_binding.bound).expect("unreachable: parse order should guarantee, that all required modules are already parsed");
            if !concrete_replacement.conform_to(&interface.0) {
                return Err(ErrorKind::AssignedTypDoesNotConformToInterface(typ.clone()).into());
            }

            // Replace all instances of the generic binding with the concrete typ.
            for submodule in &mut node.submodules {
                if *submodule.typ.typ == generic_binding.binding {
                    submodule.typ = concrete_replacement.clone();
                }
            }
        }

        Ok(Submodule {
            name: field.clone(),
            typ: node,
        })
    }
}

fn transform_connections(
    initial: Vec<Connection>,
    defs: &[ConnectionDef],
    submodules: &[Submodule],
    gates: &FxHashSet<Gate>,
    links: &FxHashMap<String, Link>,
) -> Result<Vec<Connection>> {
    let mut results = initial;
    for (idx, def) in defs.iter().enumerate() {
        transform_connection(def, submodules, gates, links, &mut results)
            .map_err(|e| e.span_connection(idx))?;
    }

    Ok(results)
}

fn transform_connection(
    def: &ConnectionDef,
    submodules: &[Submodule],
    gates: &FxHashSet<FieldDef>,
    links: &FxHashMap<String, def::LinkDef>,
    results: &mut Vec<Connection>,
) -> Result<()> {
    let lhs_resolved = transform_connection_endpoint(&def.peers[0], submodules, gates)?;
    let rhs_resolved = transform_connection_endpoint(&def.peers[1], submodules, gates)?;
    if lhs_resolved.len() != rhs_resolved.len() {
        return Err(ErrorKind::UnequalPeers(lhs_resolved.len(), rhs_resolved.len()).into());
    }
    let link = def.link.as_ref().map(|link_def| {
        links
            .get(link_def)
            .cloned()
            .ok_or_else(|| ErrorKind::UnknownLink(link_def.clone()))
    });
    let link = match link {
        None => None,
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => return Err(e.into()),
    };

    for (lhs, rhs) in lhs_resolved.into_iter().zip(rhs_resolved) {
        results.push(Connection {
            peers: [lhs, rhs],
            link: link.clone(),
        });
    }

    Ok(())
}

fn transform_connection_endpoint(
    def: &ConnectionEndpointDef,
    submodules: &[Submodule],
    gates: &FxHashSet<Gate>,
) -> Result<Vec<ConnectionEndpoint>> {
    transform_connection_endpoint_inner(&mut Vec::new(), &def.accessors, submodules, gates)
}

fn transform_connection_endpoint_inner(
    position: &mut Vec<ConnectionEndpointAccessor>,
    accessors: &[FieldDef],
    submodules: &[Submodule],
    gates: &FxHashSet<Gate>,
) -> Result<Vec<ConnectionEndpoint>> {
    assert!(!accessors.is_empty(), "accessors must be non-empty");
    let accessor = &accessors[0];
    if accessors.len() == 1 {
        // (A) Select gates from current module
        let gate_def = gates
            .iter()
            .find(|g| g.ident == accessor.ident)
            .ok_or_else(|| ErrorKind::UnknownGateInConnection(accessor.clone()))?;

        Ok(
            iter_for_kardinality_access(gate_def, accessor, &accessor.ident)?
                .map(|final_accessor| ConnectionEndpoint {
                    accessors: position
                        .iter()
                        .cloned()
                        .chain(once(final_accessor))
                        .collect(),
                })
                .collect(),
        )
    } else {
        // (B) Index into deeper submodules
        let submodule_def = submodules
            .iter()
            .find(|n| n.name.ident == accessor.ident)
            .ok_or_else(|| ErrorKind::UnknownSubmoduleInConnection(accessor.clone()))?;

        let submodule_iter =
            iter_for_kardinality_access(&submodule_def.name, accessor, &accessor.ident)?;

        let mut results = Vec::new();
        for local_module in submodule_iter {
            position.push(local_module);
            let mut inner_result = transform_connection_endpoint_inner(
                position,
                &accessors[1..],
                &submodule_def.typ.submodules,
                &submodule_def.typ.gates,
            )?;
            results.append(&mut inner_result);
            position.pop();
        }

        Ok(results)
    }
}

fn iter_for_kardinality_access<'a>(
    def: &FieldDef,
    access: &FieldDef,
    ident: &'a str,
) -> Result<Box<dyn Iterator<Item = ConnectionEndpointAccessor> + 'a>> {
    use Kardinality::{Atom, Cluster};
    match (def.kardinality, access.kardinality) {
        // 1:1 into atom
        (Atom, Atom) => Ok(Box::new(once(ConnectionEndpointAccessor {
            name: ident.to_string(),
            index: None,
        }))),

        // 1:1 into cluster
        (Cluster(n), Cluster(i)) if i < n => Ok(Box::new(once(ConnectionEndpointAccessor {
            name: ident.to_string(),
            index: Some(i),
        }))),
        (Cluster(_), Cluster(_)) => {
            Err(ErrorKind::ConnectionIndexOutOfBounds(access.clone()).into())
        }

        // cluster access into atom
        (Atom, Cluster(_)) => Err(ErrorKind::ConnectionIndexOutOfBounds(access.clone()).into()),

        // access cluster as multi-atom
        (Cluster(n), Atom) => Ok(Box::new((0..n).map(|i| ConnectionEndpointAccessor {
            name: ident.to_string(),
            index: Some(i),
        }))),
    }
}
