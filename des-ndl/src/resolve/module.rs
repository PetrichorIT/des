use std::{collections::LinkedList, sync::Arc};

use crate::{
    ast::ModuleStmt,
    error::*,
    ir::{
        Cluster, Connection, ConnectionEndpoint, Gate, GateServiceType, Module, RawSymbol,
        Submodule, Symbol,
    },
    resolve::{LinkIrTable, LocalGatesTable, LocalSubmoduleTable, ModuleIrTable, SharedGatesTable},
};

impl Module {
    pub fn from_ast(
        ast: Arc<ModuleStmt>,
        modules: &ModuleIrTable,
        links: &LinkIrTable,
        errors: &mut LinkedList<Error>,
    ) -> Module {
        let errlen = errors.len();

        // Include gate definitions
        let mut ir_gates: Vec<Gate> =
            Vec::with_capacity(ast.submodules.as_ref().map(|s| s.items.len()).unwrap_or(0));
        if let Some(ref gates) = ast.gates {
            for gate in gates.items.iter() {
                if ir_gates.iter().any(|d| d.ident.raw == gate.ident.raw) {
                    errors.push_back(Error::new(
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
                    errors.push_back(Error::new(
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
                        errors.push_back(Error::new(
                            ErrorKind::SymbolNotFound,
                            format!("symbol '{}' was not found", submodule.ident.raw),
                        ));
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
                let Some(source) = shared_gtable.get(&con.source) else {
                    errors.push_back(Error::new(
                        ErrorKind::SymbolNotFound,
                        format!("did not find symbol '{}'", con.source)
                    ));
                    continue;
                };
                let Some(target) = shared_gtable.get(&con.target) else {
                    errors.push_back(Error::new(
                        ErrorKind::SymbolNotFound,
                        format!("did not find symbol '{}'", con.target)
                    ));
                    continue;
                };

                // Check input output
                if source.service_typ == GateServiceType::Input {
                    errors.push_back(Error::new(
                        ErrorKind::InvalidConGateServiceTyp,
                        "invalid gate service type for source",
                    ));
                }
                if target.service_typ == GateServiceType::Output {
                    errors.push_back(Error::new(
                        ErrorKind::InvalidConGateServiceTyp,
                        "invalid gate service type for target",
                    ));
                }

                if let Some(ref link) = con.link {
                    let Some(link) = links.get(&link.raw) else {
                        errors.push_back(Error::new(
                            ErrorKind::SymbolNotFound,
                            format!("did not find symbol '{}'", link.raw)
                        ));
                        continue;
                    };

                    ir_connections.push(Connection {
                        from: ConnectionEndpoint::from(&con.source),
                        to: ConnectionEndpoint::from(&con.target),
                        delay: Some(Symbol::from(link)),
                    });
                } else {
                    ir_connections.push(Connection {
                        from: ConnectionEndpoint::from(&con.source),
                        to: ConnectionEndpoint::from(&con.target),
                        delay: None,
                    });
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
