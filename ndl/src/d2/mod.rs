use std::collections::HashMap;

use crate::NdlResolver;

mod compose;
mod ctx;
mod cycles;
mod prepare;
mod specs;

pub fn desugar(resolver: &mut NdlResolver) {
    let mut prepared = HashMap::new();

    for (alias, unit) in &resolver.units {
        let result = prepare::prepare(unit, resolver);
        prepared.insert(alias.clone(), result);
    }

    cycles::check_for_cycles(&mut prepared, resolver);

    // Attach errors
    for (alias, unit) in prepared.iter_mut() {
        resolver.write_if_verbose(format!("{}.prp", alias), &unit);
        resolver.ectx.desugaring_errors.append(&mut unit.errors);
    }
}
