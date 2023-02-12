use std::{
    collections::{HashMap, LinkedList},
    sync::Arc,
};

use crate::{
    ast::LinkStmt,
    error::*,
    ir::{Link, RawSymbol},
    resolve::LinkIrTable,
};

impl Link {
    pub fn from_ast(
        link: Arc<LinkStmt>,
        ir_links: &LinkIrTable,
        errors: &mut LinkedList<Error>,
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
                    errors.push_back(Error::new(
                        ErrorKind::SymbolNotFound,
                        "link symbol not found"
                    ));
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
            errors.push_back(Error::new(
                ErrorKind::LinkMissingRequiredFields,
                "missing required field 'jitter', was not defined locally or in prototypes",
            ));
        }
        if ir.latency == f64::NEG_INFINITY {
            errors.push_back(Error::new(
                ErrorKind::LinkMissingRequiredFields,
                "missing required field 'latency', was not defined locally or in prototypes",
            ));
        }
        if ir.bitrate == i32::MIN {
            errors.push_back(Error::new(
                ErrorKind::LinkMissingRequiredFields,
                "missing required field 'bitrate', was not defined locally or in prototypes",
            ));
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
