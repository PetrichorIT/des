use std::{collections::HashMap, sync::Arc};

use crate::{
    ast::{LinkStmt, Spanned},
    error::*,
    ir::{Link, RawSymbol},
    resolve::LinkIrTable,
};

use super::GlobalAstTable;

impl Link {
    pub fn from_ast(
        link: Arc<LinkStmt>,
        ir_links: &LinkIrTable,
        globals: &GlobalAstTable,
        errors: &mut ErrorsMut,
    ) -> Link {
        let errlen = errors.len();
        // We can assume
        // - no dup, valid fields (if existent)
        let mut ir = Link {
            ident: RawSymbol {
                raw: link.ident.raw.clone(),
            },

            fields: HashMap::new(),
            jitter: f64::NEG_INFINITY,
            latency: f64::NEG_INFINITY,
            bitrate: i32::MIN,

            ast: link.clone(),

            dirty: false,
        };

        // Apply inheritence in order
        if let Some(ref inh) = link.inheritance {
            for symbol in inh.symbols.iter() {
                // Resolve ident
                // All values in scope are allread in IR table
                // - local elements are non-nessecary in scope, but in order
                let Some(dep) = ir_links.get(symbol) else {
                    errors.add(Error::new(
                        ErrorKind::SymbolNotFound,
                        format!("did not find link symbol '{}', not in scope", symbol.raw)
                    ).spanned(inh.span()).map(|e| globals.err_resolve_symbol(&symbol.raw, false, e)));
                    continue;
                };
                ir.apply(&dep);
            }
        }

        for field in link.data.items.iter() {
            match field.key.raw.as_str() {
                "jitter" => ir.jitter = field.value.as_float(),
                "latency" => ir.latency = field.value.as_float(),
                "bitrate" => ir.bitrate = field.value.as_integer(),
                other => {
                    let _ = ir
                        .fields
                        .insert(other.to_string(), field.value.clone().into());
                }
            }
        }

        if ir.jitter == f64::NEG_INFINITY {
            errors.add(
                Error::new(
                    ErrorKind::LinkMissingRequiredFields,
                    "missing required field 'jitter', was not defined locally or in prototypes",
                )
                .spanned(ir.ast.span()),
            );
        }
        if ir.latency == f64::NEG_INFINITY {
            errors.add(
                Error::new(
                    ErrorKind::LinkMissingRequiredFields,
                    "missing required field 'latency', was not defined locally or in prototypes",
                )
                .spanned(ir.ast.span()),
            );
        }
        if ir.bitrate == i32::MIN {
            errors.add(
                Error::new(
                    ErrorKind::LinkMissingRequiredFields,
                    "missing required field 'bitrate', was not defined locally or in prototypes",
                )
                .spanned(ir.ast.span()),
            );
        }

        if errlen < errors.len() {
            ir.dirty = true;
        }

        ir
    }

    fn apply(&mut self, other: &Link) {
        self.jitter = other.jitter;
        self.latency = other.latency;
        self.bitrate = other.bitrate;
        for (k, v) in other.fields.iter() {
            let _ = self.fields.insert(k.clone(), v.clone());
        }
    }
}
