use std::collections::HashMap;

use crate::tycheck;
use crate::NdlResolver;

pub(crate) mod first_pass;
pub(crate) mod second_pass;

mod specs;
pub use specs::*;

mod results;
pub use results::*;

mod ctx;
pub use ctx::*;

pub fn desugar(resolver: &mut NdlResolver) {
    super::d2::desugar(resolver);
    return;

    let mut fst_pass_results = HashMap::new();

    for (alias, unit) in &resolver.units {
        let result = first_pass::first_pass(unit, resolver);
        resolver.write_if_verbose(format!("{}.fdesugar", alias), &result);
        fst_pass_results.insert(alias.clone(), result);
    }

    let mut errs = Vec::new();
    tycheck::check_cyclic_types(&fst_pass_results, &mut errs);

    for (alias, unit) in &fst_pass_results {
        let result = second_pass::second_pass(unit, &fst_pass_results, resolver);
        resolver.write_if_verbose(format!("{}.sdesugar", alias), &result);

        resolver
            .ectx
            .desugaring_errors
            .append(&mut result.errors.clone());

        resolver.desugared_units.insert(alias.clone(), result);
    }

    tycheck::check_proto_impl(&resolver.desugared_units, &resolver.source_map, &mut errs);

    resolver.ectx.desugaring_errors.append(&mut errs);
}
