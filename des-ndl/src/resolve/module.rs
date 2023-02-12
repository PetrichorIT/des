use std::{collections::LinkedList, fmt::format, sync::Arc};

use crate::{
    error::*,
    ir::{Cluster, Gate, Module, RawSymbol, Submodule},
    Connection, GateServiceType, LinkIrTable, LocalGatesTable, LocalSubmoduleTable, ModuleIrTable,
    ModuleStmt, Symbol,
};

impl Module {
    pub fn from_ast(
        ast: Arc<ModuleStmt>,
        ir_table: &ModuleIrTable,
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

                let cluster = submodule
                    .cluster
                    .as_ref()
                    .map(Cluster::from)
                    .unwrap_or(Cluster::Standalone);

                // Confirm existence of symbol
                let typ = ir_table
                    .get(&submodule.ident.raw)
                    .map(Symbol::from)
                    .unwrap_or_else(|| {
                        errors.push_back(Error::new(
                            ErrorKind::SymbolNotFound,
                            format!("symbol '{}' was not found", submodule.ident.raw),
                        ));
                        Symbol::Unresolved
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

        let gtable = LocalGatesTable::new(&ir_gates);
        let smtable = LocalSubmoduleTable::new(&ir_submodules);

        let mut ir_connections: Vec<Connection> =
            Vec::with_capacity(ast.connections.as_ref().map(|s| s.items.len()).unwrap_or(0));
        if let Some(ref connections) = ast.connections {
            for connection in connections.items.iter() {}
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
