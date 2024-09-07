use fxhash::{FxHashMap, FxHashSet};
use std::{collections::HashSet, iter::once};

pub mod def;
pub mod error;
pub mod network;

use def::{ConnectionDef, ConnectionEndpointDef, Def, FieldDef, GateDef, Kardinality, ModuleDef};
use error::{Error, Result};
use network::{
    Connection, ConnectionEndpoint, ConnectionEndpointAccessor, Gate, Link, Network, Node,
    Submodule, Symbol,
};

pub fn transform(def: &Def) -> Result<Network> {
    let mut modules = def
        .modules
        .iter()
        .map(|m| (m, m.1.required_symbols().collect::<HashSet<_>>()))
        .collect::<Vec<_>>();

    // All values 0..idx are allready resolvable in their position
    let mut idx = 0;
    let mut provider_set = FxHashSet::default();
    while idx < modules.len() {
        let Some(next) = modules[idx..]
            .iter()
            .position(|module| module.1.iter().all(|symbol| provider_set.contains(*symbol)))
        else {
            return Err(Error::UnresolvableDependency(
                modules[idx..]
                    .iter()
                    .map(|v| v.0 .0.clone())
                    .collect::<Vec<_>>(),
            ));
        };
        // relative iterator
        let next = next + idx;

        modules.swap(idx, next);
        provider_set.insert(modules[idx].0 .0.clone());
        idx += 1;
    }

    let links: FxHashMap<String, Link> = def.links.clone();

    let mut archetypes = FxHashMap::default();
    for ((ident, module), _) in modules {
        let archetyp = transform_module(ident, module, &archetypes, &links)?;
        archetypes.insert(ident.clone(), archetyp);
    }

    archetypes
        .remove(&def.entry)
        .ok_or_else(|| Error::UnknownModule(def.entry.clone()))
}

fn transform_module(
    ident: &String,
    def: &ModuleDef,
    nodes: &FxHashMap<String, Node>,
    links: &FxHashMap<String, Link>,
) -> Result<Node> {
    let mut gates = transform_gates(ident, &def.gates)?;
    let mut submodules = transform_submodules(ident, &def.submodules, nodes)?;
    let mut connections = Vec::new();

    if let Some(ref parent) = def.parent {
        let arch = nodes
            .get(parent)
            .expect("unreachable: parse order should guarantee, that all required modules are already parsed");
        gates.extend(arch.gates.iter().cloned());
        submodules.extend(arch.submodules.iter().cloned());
        connections.extend(arch.connections.iter().cloned());
    }

    let connections =
        transform_connections(connections, &def.connections, &submodules, &gates, links)?;

    Ok(Node {
        typ: Symbol::from(ident),
        gates,
        submodules,
        connections,
    })
}

fn transform_gates(ident: &String, defs: &[GateDef]) -> Result<Vec<Gate>> {
    defs.iter()
        .map(|v| {
            if v.kardinality == Kardinality::Cluster(0) {
                Err(Error::InvalidGate(ident.clone(), v.ident.clone()))
            } else {
                Ok(())
            }
        })
        .collect::<Result<()>>()?;
    Ok(defs.to_vec())
}

fn transform_submodules(
    ident: &String,
    defs: &FxHashMap<FieldDef, String>,
    nodes: &FxHashMap<String, Node>,
) -> Result<Vec<Submodule>> {
    defs.into_iter()
        .map(|(field, typ)| {
            if field.kardinality == Kardinality::Cluster(0) {
                return Err(Error::InvalidSubmodule(ident.clone(), field.ident.clone()))
            }
            Ok(Submodule {
                name: field.clone(),
                typ: nodes.get(typ)
                    .expect("unreachable: parse order should guarantee, that all required modules are already parsed")
                    .clone(),
            })
        })
        .collect::<Result<Vec<_>>>()
}

fn transform_connections(
    initial: Vec<Connection>,
    defs: &[ConnectionDef],
    nodes: &[Submodule],
    gates: &[Gate],
    links: &FxHashMap<String, Link>,
) -> Result<Vec<Connection>> {
    let mut results = initial;
    for (idx, def) in defs.into_iter().enumerate() {
        let lhs_resolved = transform_connection_endpoint(idx, &def.peers[0], nodes, gates)?;
        let rhs_resolved = transform_connection_endpoint(idx, &def.peers[1], nodes, gates)?;

        if lhs_resolved.len() != rhs_resolved.len() {
            return Err(Error::UnequalPeers(
                idx,
                lhs_resolved.len(),
                rhs_resolved.len(),
            ));
        }

        let link = def.link.as_ref().map(|link_def| {
            links
                .get(link_def)
                .cloned()
                .ok_or_else(|| Error::UnknownLink(link_def.clone()))
        });
        let link = match link {
            None => None,
            Some(Ok(v)) => Some(v),
            Some(Err(e)) => return Err(e),
        };

        for (lhs, rhs) in lhs_resolved.into_iter().zip(rhs_resolved) {
            results.push(Connection {
                peers: [lhs, rhs],
                link: link.clone(),
            })
        }
    }

    Ok(results)
}

fn transform_connection_endpoint(
    idx: usize,
    def: &ConnectionEndpointDef,
    nodes: &[Submodule],
    gates: &[Gate],
) -> Result<Vec<ConnectionEndpoint>> {
    match def {
        ConnectionEndpointDef::Local(local) => {
            Ok(transform_connection_endpoint_part_local(idx, local, gates)?
                .map(|part| ConnectionEndpoint::Local(part))
                .collect())
        }

        ConnectionEndpointDef::Remote(local, remote) => {
            transform_connection_endpoint_part_remote(idx, local, remote, nodes)
        }
    }
}

fn transform_connection_endpoint_part_remote(
    idx: usize,
    local: &FieldDef,
    remote: &FieldDef,
    submodules: &[Submodule],
) -> Result<Vec<ConnectionEndpoint>> {
    let submodule_def = submodules
        .into_iter()
        .find(|n| n.name.ident == local.ident)
        .ok_or_else(|| Error::UnknownSubmoduleInConnection(idx, local.clone()))?;

    let submodule_iter =
        iter_for_kardinality_access(idx, &submodule_def.name, local, &local.ident)?;

    let mut results = Vec::new();
    for local_module in submodule_iter {
        for remote_gate in
            transform_connection_endpoint_part_local(idx, remote, &submodule_def.typ.gates)?
        {
            results.push(ConnectionEndpoint::Remote(
                local_module.clone(),
                remote_gate,
            ))
        }
    }

    Ok(results)
}

fn transform_connection_endpoint_part_local<'a>(
    idx: usize,
    def: &'a FieldDef,
    gates: &[Gate],
) -> Result<Box<dyn Iterator<Item = ConnectionEndpointAccessor> + 'a>> {
    let gate_def = gates
        .into_iter()
        .find(|g| g.ident == def.ident)
        .ok_or_else(|| Error::UnknownGateInConnection(idx, def.clone()))?;

    iter_for_kardinality_access(idx, gate_def, def, &def.ident)
}

fn iter_for_kardinality_access<'a>(
    idx: usize,
    def: &FieldDef,
    access: &FieldDef,
    ident: &'a String,
) -> Result<Box<dyn Iterator<Item = ConnectionEndpointAccessor> + 'a>> {
    use Kardinality::*;
    match (def.kardinality, access.kardinality) {
        // 1:1 into atom
        (Atom, Atom) => Ok(Box::new(once(ConnectionEndpointAccessor {
            name: ident.clone(),
            index: None,
        }))),

        // 1:1 into cluster
        (Cluster(n), Cluster(i)) if i < n => Ok(Box::new(once(ConnectionEndpointAccessor {
            name: ident.clone(),
            index: Some(i),
        }))),
        (Cluster(_), Cluster(_)) => Err(Error::ConnectionIndexOutOfBounds(idx, access.clone())),

        // cluster access into atom
        (Atom, Cluster(_)) => Err(Error::ConnectionIndexOutOfBounds(idx, access.clone())),

        // access cluster as multi-atom
        (Cluster(n), Atom) => Ok(Box::new((0..n).into_iter().map(|i| {
            ConnectionEndpointAccessor {
                name: ident.clone(),
                index: Some(i),
            }
        }))),
    }
}
