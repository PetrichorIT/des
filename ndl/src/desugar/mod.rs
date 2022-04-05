use std::collections::HashMap;

use crate::NdlResolver;

pub(crate) mod first_pass;
pub(crate) mod second_pass;
pub(crate) mod tychk;

mod specs;
pub use specs::*;

mod results;
pub use results::*;

mod ctx;
pub use ctx::*;

pub fn desugar(resolver: &mut NdlResolver) {
    let mut fst_pass_results = HashMap::new();

    for (alias, unit) in &resolver.units {
        let result = first_pass::first_pass(unit, resolver);
        resolver.write_if_verbose(format!("{}.fdesugar", alias), &result);
        fst_pass_results.insert(alias.clone(), result);
    }

    for (alias, unit) in &fst_pass_results {
        let result = second_pass::second_pass(unit, &fst_pass_results, resolver);
        resolver.write_if_verbose(format!("{}.sdesugar", alias), &result);
        resolver.desugared_units.insert(alias.clone(), result);
    }

    for (_alias, unit) in &resolver.desugared_units {
        let mut errs = tychk::tychk(unit, &resolver.desugared_units, resolver);

        // error aligment
        resolver
            .ectx
            .desugaring_errors
            .append(&mut unit.errors.clone());
        resolver.ectx.desugaring_errors.append(&mut errs);
    }
}
