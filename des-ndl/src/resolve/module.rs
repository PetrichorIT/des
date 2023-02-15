use std::sync::Arc;

use crate::{
    ast::{ModuleStmt, Spanned},
    error::*,
    ir::{
        Cluster, Connection, ConnectionEndpoint, Gate, GateServiceType, Module, RawSymbol,
        Submodule, Symbol,
    },
    resolve::{LinkIrTable, LocalGatesTable, LocalSubmoduleTable, ModuleIrTable, SharedGatesTable},
};

use super::GlobalAstTable;

impl Module {
    pub fn from_ast(
        ast: Arc<ModuleStmt>,
        modules: &ModuleIrTable,
        links: &LinkIrTable,
        global: &GlobalAstTable,
        errors: &mut ErrorsMut,
    ) -> Module {
        let errlen = errors.len();

        // Include gate definitions
        let mut ir_gates: Vec<Gate> =
            Vec::with_capacity(ast.submodules.as_ref().map(|s| s.items.len()).unwrap_or(0));
        if let Some(ref gates) = ast.gates {
            for gate in gates.items.iter() {
                if ir_gates.iter().any(|d| d.ident.raw == gate.ident.raw) {
                    errors.add(Error::new(
                        ErrorKind::SymbolDuplication,
                        "gate symbol duplication ( should never appear )",
                    ));
                    continue;
                }
                ir_gates.push(Gate {
                    ident: RawSymbol {
                        raw: gate.ident.raw.clone(),
                    },
                    cluster: gate
                        .cluster
                        .as_ref()
                        .map(Cluster::from)
                        .unwrap_or(Cluster::Standalone),
                    service_typ: gate
                        .annotation
                        .as_ref()
                        .map(GateServiceType::from)
                        .unwrap_or(GateServiceType::None),
                });
            }
        }

        // Include submodule definition
        let mut ir_submodules: Vec<Submodule> =
            Vec::with_capacity(ast.submodules.as_ref().map(|s| s.items.len()).unwrap_or(0));
        if let Some(ref submodules) = ast.submodules {
            for submodule in submodules.items.iter() {
                if ir_submodules
                    .iter()
                    .any(|d| d.ident.raw == submodule.ident.raw)
                {
                    errors.add(Error::new(
                        ErrorKind::SymbolDuplication,
                        "gate symbol duplication ( should never appear )",
                    ));
                    continue;
                }

                let cluster = Cluster::from(&submodule.cluster);

                // Confirm existence of symbol
                let typ = modules
                    .get(&submodule.typ.raw)
                    .map(Symbol::from)
                    .unwrap_or_else(|| {
                        errors.add(
                            Error::new(
                                ErrorKind::SymbolNotFound,
                                format!(
                                    "did not find submodule symbol '{}', not in scope",
                                    submodule.typ.raw
                                ),
                            )
                            .spanned(submodule.typ.span()),
                        );
                        Symbol::Unresolved(submodule.typ.raw.clone())
                    });

                let ident = RawSymbol {
                    raw: submodule.ident.raw.clone(),
                };
                ir_submodules.push(Submodule {
                    ident,
                    cluster,
                    typ,
                });
            }
        }

        let local_gtable = LocalGatesTable::new(&ir_gates);
        let sm_table = LocalSubmoduleTable::new(&ir_submodules);
        let shared_gtable = SharedGatesTable::new(&local_gtable, &sm_table);

        let mut ir_connections: Vec<Connection> =
            Vec::with_capacity(ast.connections.as_ref().map(|s| s.items.len()).unwrap_or(0));
        if let Some(ref connections) = ast.connections {
            for con in connections.items.iter() {
                let delay = if let Some(link) = &con.link {
                    let Some(link) = links.get(&link.raw) else {
                        errors.add(Error::new(
                            ErrorKind::SymbolNotFound,
                            format!("did not find link symbol '{}', not in scope", link.raw)
                        ).spanned(con.span()).map(|e| global.err_resolve_symbol(&link.raw, false, e)));
                        continue
                    };
                    Some(Symbol::from(link))
                } else {
                    None
                };

                let lhs = shared_gtable.resolve(&con.source);
                let rhs = shared_gtable.resolve(&con.target);

                let (lhs, rhs) = match (lhs, rhs) {
                    (Ok(lhs), Ok(rhs)) => (lhs, rhs),
                    (Err(e), Ok(_)) => {
                        errors.add(e);
                        continue;
                    }
                    (Ok(_), Err(e)) => {
                        errors.add(e);
                        continue;
                    }
                    (Err(e1), Err(e2)) => {
                        errors.add(e1);
                        errors.add(e2);
                        continue;
                    }
                };

                let mut lhs = lhs.collect::<Vec<_>>();
                let mut rhs = rhs.collect::<Vec<_>>();

                let n = lhs.len().min(rhs.len());
                for _ in 0..n {
                    let l = lhs.pop().unwrap();
                    let r = rhs.pop().unwrap();

                    if l.def.service_typ == GateServiceType::Input {
                        errors.add(Error::new(
                            ErrorKind::InvalidConGateServiceTyp,
                            format!("Gate '{}' cannot serve as connection source, since it is of serivce type '{:?}'", l.def.ident.raw, l.def.service_typ)
                        ));
                    }

                    if r.def.service_typ == GateServiceType::Output {
                        errors.add(Error::new(
                            ErrorKind::InvalidConGateServiceTyp,
                            format!("Gate '{}' cannot serve as connection target, since it is of serivce type '{:?}'", r.def.ident.raw, r.def.service_typ)
                        ));
                    }

                    ir_connections.push(Connection {
                        from: ConnectionEndpoint::new(&con.source, &l),
                        to: ConnectionEndpoint::new(&con.target, &r),
                        delay: delay.clone(),
                    });
                }

                if lhs.len() > 0 {
                    errors.add(Error::new(
                        ErrorKind::InvalidConDefSizes,
                        format!("Invalid connection statement, source domain is bigger than target domain (by {} gates)", lhs.len())
                    ))
                }
                if rhs.len() > 0 {
                    errors.add(Error::new(
                        ErrorKind::InvalidConDefSizes,
                        format!("Invalid connection statement, target domain is bigger than source domain (by {} gates)", rhs.len())
                    ))
                }
            }
        }

        let ident = RawSymbol {
            raw: ast.ident.raw.clone(),
        };
        Module {
            ast,
            ident,
            gates: ir_gates,
            submodules: ir_submodules,
            connections: ir_connections,
            dirty: errlen < errors.len(),
        }
    }
}
