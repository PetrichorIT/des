use std::collections::HashMap;

use crate::NdlResolver;

pub mod compose;
pub mod connections;
pub mod ctx;
pub mod cycles;
pub mod expand;
pub mod prepare;
pub mod result;

pub mod specs;

pub fn desugar(resolver: &mut NdlResolver) {
    let mut errors = prepare::prepare(resolver);
    resolver.ectx.desugaring_errors.append(&mut errors);

    let mut prepared = HashMap::new();

    for (alias, unit) in &resolver.units {
        let result = expand::expand(unit, resolver);
        prepared.insert(alias.clone(), result);
    }

    cycles::check_for_cycles(&mut prepared, resolver);

    // Attach errors
    for (alias, unit) in prepared.iter_mut() {
        resolver.write_if_verbose(format!("{}.prp", alias), &unit);
        resolver.ectx.desugaring_errors.append(&mut unit.errors);
    }

    let mut result = compose::compose(&mut prepared, resolver);

    crate::tycheck::check_proto_impl(&mut result, &resolver.source_map);

    resolver.ectx.desugaring_errors.append(&mut result.errors);
    resolver.result = Some(result);
}
