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

        // load deps;
        // TODO: check for dirty deps.
        let mut deps = Vec::new();
        let mut inherited = Vec::new();
        if let Some(ref inh) = ast.inheritance {
            for dep in inh.symbols.iter() {
                let typ = match modules.get(dep) {
                    Some(v) => v,
                    None => {
                        errors.add(
                            Error::new(
                                ErrorKind::SymbolNotFound,
                                format!(
                                    "did not find inheritance symbol '{}', not in scope",
                                    dep.raw
                                ),
                            )
                            .spanned(dep.span())
                            .map(|e| global.err_resolve_symbol(&dep.raw, true, e)),
                        );
                        inherited.push(Symbol::Unresolved(RawSymbol {
                            raw: dep.raw.clone(),
                        }));
                        continue;
                    }
                };
                deps.push(typ.clone());
                inherited.push(Symbol::from(typ));
            }
        }

        // Include gate definitions
        let mut ir_gates: Vec<Gate> = Vec::with_capacity(ast.gates.len());
        for inh in &deps {
            ir_gates.extend(inh.gates.iter().cloned());
        }
        for gate in ast.gates.iter().flat_map(|stmt| stmt.items.iter()) {
            if ir_gates.iter().any(|d| d.ident.raw == gate.ident.raw) {
                errors.add(
                    Error::new(
                        ErrorKind::SymbolDuplication,
                        format!(
                            "gate(-cluster) '{}' was defined multiple times",
                            gate.ident.raw
                        ),
                    )
                    .spanned(gate.span()),
                );
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

        // Include submodule definition
        let mut ir_submodules: Vec<Submodule> = Vec::with_capacity(ast.submodules.len());
        for inh in &deps {
            ir_submodules.extend(inh.submodules.iter().cloned());
        }
        for submodule_ast in ast.submodules.iter().flat_map(|stmt| stmt.items.iter()) {
            if ir_submodules
                .iter()
                .any(|d| d.ident.raw == submodule_ast.ident.raw)
            {
                errors.add(
                    Error::new(
                        ErrorKind::SymbolDuplication,
                        format!(
                            "submodule(-cluster) '{}' was defined multiple times",
                            submodule_ast.ident.raw
                        ),
                    )
                    .spanned(submodule_ast.span()),
                );
                continue;
            }

            let cluster = Cluster::from(&submodule_ast.cluster);

            // Confirm existence of symbol
            let typ = modules
                .get(&submodule_ast.typ.raw())
                .map(Symbol::from)
                .unwrap_or_else(|| {
                    println!("seaching: {}", submodule_ast.typ.raw());
                    println!("modules: {modules:#?}");

                    errors.add(
                        Error::new(
                            ErrorKind::SymbolNotFound,
                            format!(
                                "did not find submodule symbol '{}', not in scope",
                                submodule_ast.typ.raw()
                            ),
                        )
                        .spanned(submodule_ast.typ.span())
                        .map(|e| global.err_resolve_symbol(&submodule_ast.typ.raw(), true, e)),
                    );
                    Symbol::Unresolved(RawSymbol {
                        raw: submodule_ast.typ.raw(),
                    })
                });

            let submod_ir = if let Some(ref specs) = submodule_ast.dyn_spec {
                // since we not monomorphise a new entry, create a new instance
                if let Some(mut override_ir) = typ.as_module().cloned() {
                    // override the existing specs.
                    for spec in specs.items.iter() {
                        // found overide <dyn_field> = <value>
                        let dyn_field = &spec.key.raw;
                        let Some(dyn_field) = override_ir.submodules.iter_mut().find(|d| d.ident.raw == *dyn_field) else {
                            // <dyn_field> pointed to an unknown submodule (of the submodule)
                            errors.add(Error::new(
                                ErrorKind::SymbolNotFound,
                                format!("did not find submodule symbol for dyn-spec '{}', not in scope", dyn_field)
                            ).spanned(spec.key.span()));
                            continue;
                        };

                        let typ = modules.get(&spec.value.raw);
                        if let Some(ref typ) = typ {
                            if let Some(expected_proto) = dyn_field.typ.as_module_arc() {
                                // check wheter the provided type is really
                                // implementing the expected proto
                                let valid = typ.inherited.iter().any(|s| {
                                    Arc::ptr_eq(&s.as_module_arc().unwrap(), &expected_proto)
                                });
                                if !valid {
                                    errors.add(
                                        Error::new(
                                            ErrorKind::ModuleDynConstraintsBroken,
                                            format!("module '{}' does not inherit '{}', thus cannot be assigned to dyn field '{}'", typ.ident.raw, expected_proto.ident.raw, spec.key.raw)
                                        ).spanned(spec.span())
                                    )
                                }
                            }
                        }

                        let typ = typ.map(Symbol::from).unwrap_or_else(|| {
                            errors.add(
                                Error::new(
                                    ErrorKind::SymbolNotFound,
                                    format!(
                                        "did not find dyn-spec submodule symbol '{}', not in scope",
                                        spec.value.raw
                                    ),
                                )
                                .spanned(spec.value.span())
                                .map(|e| global.err_resolve_symbol(&spec.value.raw, true, e)),
                            );
                            Symbol::Unresolved(RawSymbol {
                                raw: submodule_ast.typ.raw(),
                            })
                        });

                        dyn_field.dynamic = false;
                        dyn_field.typ = typ;
                    }

                    let ident = RawSymbol {
                        raw: submodule_ast.ident.raw.clone(),
                    };
                    Submodule {
                        span: override_ir.ast.span(),
                        ident,
                        cluster,
                        typ: Symbol::from(Arc::new(override_ir)),
                        dynamic: submodule_ast.typ.is_dyn(),
                    }
                } else {
                    // if the symbol is not resolved either way just add it in its incomplete form
                    let ident = RawSymbol {
                        raw: submodule_ast.ident.raw.clone(),
                    };
                    Submodule {
                        span: submodule_ast.span(),
                        ident,
                        cluster,
                        typ,
                        dynamic: submodule_ast.typ.is_dyn(),
                    }
                }
            } else {
                let ident = RawSymbol {
                    raw: submodule_ast.ident.raw.clone(),
                };
                Submodule {
                    span: submodule_ast.span(),
                    ident,
                    cluster,
                    typ,
                    dynamic: submodule_ast.typ.is_dyn(),
                }
            };

            if let Some(s) = submod_ir.typ.as_module() {
                let mut missing = Vec::new();
                for dep in s.submodules.iter() {
                    if dep.dynamic {
                        missing.push(&dep.ident.raw[..])
                    }
                }

                if !missing.is_empty() {
                    let s = missing.join(", ");
                    errors.add(Error::new(
                        ErrorKind::ModuleDynNotResolved,
                        format!(
                            "missing specification for dynamic members of submodule '{}': missing fields '{}'",
                            submod_ir.ident.raw, s
                        ),
                    ).spanned(submod_ir.span))
                }
            }

            ir_submodules.push(submod_ir);
        }

        let local_gtable = LocalGatesTable::new(&ir_gates);
        let sm_table = LocalSubmoduleTable::new(&ir_submodules);
        let shared_gtable = SharedGatesTable::new(&local_gtable, &sm_table);

        let mut ir_connections: Vec<Connection> = Vec::with_capacity(ast.connections.len());
        for inh in &deps {
            ir_connections.extend(inh.connections.iter().cloned());
        }
        for con in ast.connections.iter().flat_map(|s| s.items.iter()) {
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
                    from: ConnectionEndpoint::from(l),
                    to: ConnectionEndpoint::from(r),
                    delay: delay.clone(),
                });
            }

            if !lhs.is_empty() {
                errors.add(Error::new(
                        ErrorKind::InvalidConDefSizes,
                        format!("Invalid connection statement, source domain is bigger than target domain (by {} gates)", lhs.len())
                    ))
            }
            if !rhs.is_empty() {
                errors.add(Error::new(
                        ErrorKind::InvalidConDefSizes,
                        format!("Invalid connection statement, target domain is bigger than source domain (by {} gates)", rhs.len())
                    ))
            }
        }

        let ident = RawSymbol {
            raw: ast.ident.raw.clone(),
        };
        Module {
            ast,
            ident,
            inherited,
            gates: ir_gates,
            submodules: ir_submodules,
            connections: ir_connections,
            dirty: errlen < errors.len(),
        }
    }
}
