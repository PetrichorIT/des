use super::*;
use crate::ast::*;

impl Validate for ModuleStmt {
    fn validate(&self, errors: &mut ErrorsMut) {
        let has_inh = self.inheritance.is_some();
        let has_dyn = self
            .submodules
            .iter()
            .any(|s| s.items.iter().any(|s| s.typ.is_dyn()));
        if has_inh && has_dyn {
            errors.add(
                Error::new(
                    ErrorKind::ModuleBothInheritanceAndDyn,
                    "module is defined with both inheritance and dyn members: not supported",
                )
                .spanned(self.span()),
            );
        }

        if let Some(ref inh) = self.inheritance {
            inh.validate(errors)
        }
        self.gates.iter().for_each(|s| s.validate(errors));
        self.submodules.iter().for_each(|s| s.validate(errors));
        self.connections.iter().for_each(|s| s.validate(errors));
    }
}

impl Validate for ModuleInheritance {
    fn validate(&self, errors: &mut ErrorsMut) {
        let mut symbols = Vec::with_capacity(self.symbols.len());
        for symbol in self.symbols.iter() {
            if symbols.contains(&&symbol.raw) {
                errors.add(
                    Error::new(
                        ErrorKind::ModuleInheritanceDuplicatedSymbols,
                        format!(
                            "found duplicated symbol '{}' in module inheritance statement",
                            symbol.raw
                        ),
                    )
                    .spanned(self.span()),
                );
                continue;
            }
            symbols.push(&symbol.raw);
        }
    }
}

impl Validate for GatesStmt {
    fn validate(&self, errors: &mut ErrorsMut) {
        let mut symbols = Vec::with_capacity(self.items.len());
        for gate_def in self.items.iter() {
            // (0) Duplication checking
            if symbols.contains(&&gate_def.ident.raw) {
                errors.add(
                    Error::new(
                        ErrorKind::ModuleGatesDuplicatedSymbols,
                        format!(
                            "gate(-cluster) '{}' was defined multiple times",
                            gate_def.ident.raw
                        ),
                    )
                    .spanned(gate_def.span()),
                );
            } else {
                symbols.push(&gate_def.ident.raw);
            }
            // (2) Literal checking
            if let Some(cluster) = gate_def.cluster.as_ref() {
                if let LitKind::Integer { ref lit } = cluster.lit.kind {
                    if *lit > 0 {
                        /* GOOD */
                    } else {
                        errors.add(
                            Error::new(
                                ErrorKind::ModuleGatesInvalidClusterSize,
                                format!(
                                "cannot create gate-cluster of size '{}', requires positiv integer",
                                lit
                            ),
                            )
                            .spanned(cluster.span()),
                        );
                    }
                } else {
                    errors.add(Error::new(
                        ErrorKind::InvalidLitTyp,
                        format!(
                            "cannot create gate-cluster with literal of type {}, expected literal of type integer",
                            cluster.lit.kind.typ()
                        ),
                    ).spanned(cluster.span()))
                }
            }
        }
    }
}

impl Validate for SubmodulesStmt {
    fn validate(&self, errors: &mut ErrorsMut) {
        let mut symbols = Vec::with_capacity(self.items.len());
        for submod_def in self.items.iter() {
            // (0) Duplication checking
            if symbols.contains(&&submod_def.ident.raw) {
                errors.add(
                    Error::new(
                        ErrorKind::ModuleSubDuplicatedSymbols,
                        format!(
                            "submodule(-cluster) '{}' was defined multiple times",
                            submod_def.ident.raw
                        ),
                    )
                    .spanned(submod_def.span()),
                );
            } else {
                symbols.push(&submod_def.ident.raw);
            }

            // (2) Literal checking
            if let Some(cluster) = submod_def.cluster.as_ref() {
                if let LitKind::Integer { ref lit } = cluster.lit.kind {
                    if *lit > 0 {
                        /* GOOD */
                    } else {
                        errors.add(Error::new(
                            ErrorKind::ModuleSubInvalidClusterSize,
                            format!(
                                "cannot create submodule cluster of size '{}', requires positiv integer",
                                lit
                            ),
                        ).spanned(cluster.span()));
                    }
                } else {
                    errors.add(
                        Error::new(
                            ErrorKind::InvalidLitTyp,
                            format!(
                                "invalid literal type {}, expected literal of type integer",
                                cluster.lit.kind.typ()
                            ),
                        )
                        .spanned(cluster.span()),
                    )
                }
            }
        }
    }
}

impl Validate for ConnectionsStmt {
    fn validate(&self, _errors: &mut ErrorsMut) {}
}

#[cfg(test)]
mod tests {
    use crate::SourceMap;

    use super::*;

    fn load_module(smap: &mut SourceMap, asset: &str, raw: &str) -> ModuleStmt {
        let asset = smap.load_raw(asset, raw);
        let ts = TokenStream::new(asset).expect("Failed to create tokenstream, in validation pass");
        let buf = ParseBuffer::new(asset, ts);

        ModuleStmt::parse(&buf).expect("Failed to create object, in validation pass")
    }

    #[test]
    fn invalid_gates() {
        let mut smap = SourceMap::new();

        // # Case 0 (baseline)
        let stmt = load_module(
            &mut smap,
            "raw:case0",
            "module A { 
            gates {
                in,
                out[5],
                inout,
                outin[2],
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert!(errors.is_empty());

        // # Case 1 (duplication)
        let stmt = load_module(
            &mut smap,
            "raw:case1",
            "module A { 
            gates {
                in,
                out[5],
                inout,
                outin[2],
                in,
                out,
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::ModuleGatesDuplicatedSymbols
        );
        assert_eq!(
            errors.get(1).unwrap().kind,
            ErrorKind::ModuleGatesDuplicatedSymbols
        );

        // # Case 2 (annotation)
        let stmt = load_module(
            &mut smap,
            "raw:case2",
            "module A { 
            gates {
                in,
                out[5],
                inout,
                outin[2]
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 0);

        // # Case 3 (literals)
        let stmt = load_module(
            &mut smap,
            "raw:case3",
            "module A { 
            gates {
                in,
                out[1.0],
                inout,
                outin[\"\"]
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(errors.get(0).unwrap().kind, ErrorKind::InvalidLitTyp);
        assert_eq!(errors.get(1).unwrap().kind, ErrorKind::InvalidLitTyp);

        // # Case 3 (cluster-size)
        let stmt = load_module(
            &mut smap,
            "raw:case3",
            "module A { 
            gates {
                in,
                out[0],
                inout,
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::ModuleGatesInvalidClusterSize
        );
    }

    #[test]
    fn invalid_submodules() {
        let mut smap = SourceMap::new();

        // # Case 0 (baseline)
        let stmt = load_module(
            &mut smap,
            "raw:case0",
            "module A { 
            submodules {
                in: A,
                out[5]: B,
                inout: C,
                outin[2]: D,
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert!(errors.is_empty());

        // # Case 1 (duplication)
        let stmt = load_module(
            &mut smap,
            "raw:case1",
            "module A { 
            submodules {
                in: In,
                in: Out,
                out: Out,
                out[4]: In
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::ModuleSubDuplicatedSymbols
        );
        assert_eq!(
            errors.get(1).unwrap().kind,
            ErrorKind::ModuleSubDuplicatedSymbols
        );

        // # Case 2 (literals)
        let stmt = load_module(
            &mut smap,
            "raw:case2",
            "module A { 
            submodules {
                in: In,
                out[\"str\"]: Out,
                pash[0.0]: C
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(errors.get(0).unwrap().kind, ErrorKind::InvalidLitTyp);
        assert_eq!(errors.get(1).unwrap().kind, ErrorKind::InvalidLitTyp);

        // # Case 2 (cluster-size)
        let stmt = load_module(
            &mut smap,
            "raw:case2",
            "module A { 
            submodules {
                in: In,
                out[0]: Out,
            }
         }",
        );
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 1);

        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::ModuleSubInvalidClusterSize
        );
    }
}
