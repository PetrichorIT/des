use super::*;
use crate::ast::*;

impl Validate for ModuleStmt {
    fn validate(&self, errors: &mut LinkedList<Error>) {
        self.gates.as_ref().map(|g| g.validate(errors));
        self.submodules.as_ref().map(|s| s.validate(errors));
        self.connections.as_ref().map(|c| c.validate(errors));
    }
}

impl Validate for GatesStmt {
    fn validate(&self, errors: &mut LinkedList<Error>) {
        let mut symbols = Vec::with_capacity(self.items.len());
        for gate_def in self.items.iter() {
            // (0) Duplication checking
            if symbols.contains(&&gate_def.ident.raw) {
                errors.push_back(Error::new(
                    ErrorKind::ModuleGatesDuplicatedSymbols,
                    format!(
                        "gate(-cluster) '{}' was defined multiple times",
                        gate_def.ident.raw
                    ),
                ));
            } else {
                symbols.push(&gate_def.ident.raw);
            }

            // (1) Annotation correctness
            match gate_def.annotation.as_ref().map(|a| a.raw.as_ref()) {
                Some("input") | Some("in") | Some("Input") | Some("In") => { /* NOP */ }
                Some("output") | Some("out") | Some("Output") | Some("Out") => { /* NOP */ }
                Some(annot) => errors.push_back(Error::new(
                    ErrorKind::InvalidAnnotation,
                    format!(
                        "invalid annotation '{}', gates can only be annoted with input/output",
                        annot
                    ),
                )),
                None => { /* NOP */ }
            }

            // (2) Literal checking
            if let Some(cluster) = gate_def.cluster.as_ref() {
                if let LitKind::Integer { ref lit } = cluster.lit.kind {
                    if *lit > 0 {
                        /* GOOD */
                    } else {
                        errors.push_back(Error::new(
                            ErrorKind::ModuleGatesInvalidClusterSize,
                            format!(
                                "cannot create gate cluster of size '{}', requires positiv integer",
                                lit
                            ),
                        ));
                    }
                } else {
                    errors.push_back(Error::new(
                        ErrorKind::InvalidLitTyp,
                        format!(
                            "invalid literal type {}, expected literal of type integer",
                            cluster.lit.kind.typ()
                        ),
                    ))
                }
            }
        }
    }
}

impl Validate for SubmodulesStmt {
    fn validate(&self, errors: &mut LinkedList<Error>) {
        let mut symbols = Vec::with_capacity(self.items.len());
        for gate_def in self.items.iter() {
            // (0) Duplication checking
            if symbols.contains(&&gate_def.ident.raw) {
                errors.push_back(Error::new(
                    ErrorKind::ModuleSubDuplicatedSymbols,
                    format!(
                        "submodule(-cluster) '{}' was defined multiple times",
                        gate_def.ident.raw
                    ),
                ));
            } else {
                symbols.push(&gate_def.ident.raw);
            }

            // (2) Literal checking
            if let Some(cluster) = gate_def.cluster.as_ref() {
                if let LitKind::Integer { ref lit } = cluster.lit.kind {
                    if *lit > 0 {
                        /* GOOD */
                    } else {
                        errors.push_back(Error::new(
                            ErrorKind::ModuleSubInvalidClusterSize,
                            format!(
                                "cannot create submodule cluster of size '{}', requires positiv integer",
                                lit
                            ),
                        ));
                    }
                } else {
                    errors.push_back(Error::new(
                        ErrorKind::InvalidLitTyp,
                        format!(
                            "invalid literal type {}, expected literal of type integer",
                            cluster.lit.kind.typ()
                        ),
                    ))
                }
            }
        }
    }
}

impl Validate for ConnectionsStmt {
    fn validate(&self, _errors: &mut LinkedList<Error>) {}
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{Parse, ParseBuffer},
        SourceMap,
    };

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
                inout @input,
                outin[2] @output,
            }
         }",
        );
        let mut errors = LinkedList::new();
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
                inout @input,
                outin[2] @output,
                in,
                out,
            }
         }",
        );
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(
            errors.pop_front().unwrap().kind,
            ErrorKind::ModuleGatesDuplicatedSymbols
        );
        assert_eq!(
            errors.pop_front().unwrap().kind,
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
                inout @inpuat,
                outin[2] @oautput
            }
         }",
        );
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(
            errors.pop_front().unwrap().kind,
            ErrorKind::InvalidAnnotation
        );
        assert_eq!(
            errors.pop_front().unwrap().kind,
            ErrorKind::InvalidAnnotation
        );

        // # Case 3 (literals)
        let stmt = load_module(
            &mut smap,
            "raw:case3",
            "module A { 
            gates {
                in,
                out[1.0],
                inout @input,
                outin[\"\"] @output
            }
         }",
        );
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(errors.pop_front().unwrap().kind, ErrorKind::InvalidLitTyp);
        assert_eq!(errors.pop_front().unwrap().kind, ErrorKind::InvalidLitTyp);

        // # Case 3 (cluster-size)
        let stmt = load_module(
            &mut smap,
            "raw:case3",
            "module A { 
            gates {
                in,
                out[0],
                inout @input,
            }
         }",
        );
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.pop_front().unwrap().kind,
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
        let mut errors = LinkedList::new();
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
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(
            errors.pop_front().unwrap().kind,
            ErrorKind::ModuleSubDuplicatedSymbols
        );
        assert_eq!(
            errors.pop_front().unwrap().kind,
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
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 2);

        assert_eq!(errors.pop_front().unwrap().kind, ErrorKind::InvalidLitTyp);
        assert_eq!(errors.pop_front().unwrap().kind, ErrorKind::InvalidLitTyp);

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
        let mut errors = LinkedList::new();
        stmt.validate(&mut errors);
        assert_eq!(errors.len(), 1);

        assert_eq!(
            errors.pop_front().unwrap().kind,
            ErrorKind::ModuleSubInvalidClusterSize
        );
    }
}
