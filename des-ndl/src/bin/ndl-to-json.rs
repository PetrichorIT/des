use std::{error::Error, fs};

use des_ndl::{
    ast::{ClusterDefinition, ModuleGateReference},
    ir::Item,
    Context,
};
use des_networks::ndl::def::{self, FieldDef, Kardinality};

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    for arg in args {
        println!("arg:={arg}");
        let ctx = match Context::load(&arg) {
            Ok(ctx) => ctx,
            Err(e) => {
                println!("[{e}]");
                return Ok(());
            }
        };

        let mut def = def::Def {
            entry: ctx.entry.as_ref().map(|v| v.ident.raw.clone()).unwrap(),
            ..Default::default()
        };
        for (_, items) in ctx.ir {
            for item in items.items {
                match item {
                    Item::Link(link) => {
                        def.links.insert(
                            link.ident.raw.clone(),
                            def::LinkDef {
                                latency: link.latency,
                                jitter: link.jitter,
                                bitrate: link.bitrate,
                                other: link
                                    .fields
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.to_string()))
                                    .collect(),
                            },
                        );
                    }
                    Item::Module(module) => {
                        let ident = module.ident.raw.clone();
                        let gates = module
                            .ast
                            .gates
                            .iter()
                            .flat_map(|g| g.items.iter())
                            .map(|g| def::GateDef {
                                ident: g.ident.raw.clone(),
                                kardinality: c2ko(&g.cluster),
                            })
                            .collect::<Vec<_>>();

                        let submodules = module
                            .ast
                            .submodules
                            .iter()
                            .flat_map(|s| s.items.iter())
                            .map(|s| {
                                (
                                    FieldDef {
                                        ident: s.ident.raw.clone(),
                                        kardinality: c2ko(&s.cluster),
                                    },
                                    s.typ.raw(),
                                )
                            })
                            .collect();
                        let connections = module
                            .ast
                            .connections
                            .iter()
                            .flat_map(|c| c.items.iter())
                            .map(|c| def::ConnectionDef {
                                peers: [e2e(&c.lhs), e2e(&c.rhs)],
                                link: c.link.as_ref().map(|s| s.raw.clone()),
                            })
                            .collect::<Vec<_>>();

                        def.modules.insert(
                            ident,
                            def::ModuleDef {
                                parent: module.inherited.first().map(|s| s.raw().raw.clone()),
                                submodules,
                                gates,
                                connections,
                            },
                        );
                    }
                }
            }
        }

        let str_json = serde_json::to_string_pretty(&def)?;
        let new_path = arg.trim_end_matches(".ndl").to_string() + ".json";
        fs::write(new_path, str_json)?;

        let str_yml = serde_yml::to_string(&def)?;
        let new_path = arg.trim_end_matches(".ndl").to_string() + ".yml";
        fs::write(new_path, str_yml)?;
    }

    Ok(())
}

fn e2e(ep: &ModuleGateReference) -> def::ConnectionEndpointDef {
    match ep {
        ModuleGateReference::Local(local) => def::ConnectionEndpointDef {
            accessors: vec![FieldDef {
                ident: local.gate.raw.clone(),
                kardinality: c2ko(&local.gate_cluster),
            }],
        },
        ModuleGateReference::Nonlocal(nonlocal) => def::ConnectionEndpointDef {
            accessors: vec![
                FieldDef {
                    ident: nonlocal.submodule.raw.clone(),
                    kardinality: c2ko(&nonlocal.submodule_cluster),
                },
                FieldDef {
                    ident: nonlocal.gate.gate.raw.clone(),
                    kardinality: c2ko(&nonlocal.gate.gate_cluster),
                },
            ],
        },
    }
}

fn c2ko(cluster: &Option<ClusterDefinition>) -> Kardinality {
    match cluster {
        None => Kardinality::Atom,
        Some(cluster) => Kardinality::Cluster(cluster.lit.as_integer() as usize),
    }
}
