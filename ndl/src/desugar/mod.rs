use crate::*;

use crate::error::*;
use crate::parser::*;
use std::collections::HashMap;
use std::fmt::Display;

mod first;
mod second;
mod third;

mod results;
mod specs;
mod tyctx;
pub use results::*;
pub use specs::*;
pub use tyctx::*;

pub fn desugar(resolver: &mut NdlResolver) {
    let mut first_pass_units: HashMap<String, FirstPassDesugarResult> = HashMap::new();

    // First pass
    for (alias, unit) in &resolver.units {
        let desugared = first::first_pass(unit, resolver);

        resolver.write_if_verbose(format!("{}.fdesugar", alias), &desugared);

        first_pass_units.insert(alias.clone(), desugared);
    }

    // Second pass
    for (alias, fpass) in &first_pass_units {
        let result = second::second_pass(fpass, &first_pass_units, resolver);

        // // Defer errors
        // resolver
        //     .ectx
        //     .desugaring_errors
        //     .append(&mut result.errors.clone());

        resolver.desugared_units.insert(alias.clone(), result);
    }

    // third pass
    for (alias, pass) in &resolver.desugared_units {
        let mut errs = third::third_pass(pass, &resolver.desugared_units, resolver);

        resolver.write_if_verbose(format!("{}.sdesugar", alias), &pass);

        // Defer errors
        resolver
            .ectx
            .desugaring_errors
            .append(&mut pass.errors.clone());

        // TODO: Attach third pass errors to units
        resolver.ectx.desugaring_errors.append(&mut errs);
    }
}
