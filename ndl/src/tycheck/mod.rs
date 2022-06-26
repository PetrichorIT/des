use crate::{error::*, DesugaredResult, NdlResolver, SourceMap};
use crate::{AssetDescriptor, ChildNodeSpec};

const PAR_TYPES: [&str; 15] = [
    "usize", "u8", "u16", "u32", "u64", "u128", "isize", "i8", "i16", "i32", "i64", "i128", "bool",
    "char", "String",
];

pub fn check_proto_impl(result: &mut DesugaredResult, smap: &SourceMap) {
    let mut errors = Vec::new();

    for module in &result.modules {
        for child in &module.submodules {
            check_proto_impl_block(
                child,
                result,
                false,
                smap,
                &mut errors,
                module.ident.asset(),
            )
        }
    }

    for subsystem in &result.subsystems {
        for child in &subsystem.nodes {
            check_proto_impl_block(
                child,
                result,
                true,
                smap,
                &mut errors,
                subsystem.ident.asset(),
            )
        }
    }

    result.errors.append(&mut errors);
}

fn check_proto_impl_block(
    child: &ChildNodeSpec,
    result: &DesugaredResult,
    is_subsystem: bool,
    smap: &SourceMap,
    errors: &mut Vec<Error>,
    asset: &AssetDescriptor,
) {
    // PROTO IMPL
    if let Some(ref p) = child.proto_impl {
        let ty = result
            .modules
            .iter()
            .find(|m| m.ident == *child.ty.valid_ident().unwrap())
            .expect("[desugar] This should have bee checked in the first pass");

        // check whether a proto impl makes any sense
        let dof: Vec<(&str, &str)> = ty.degrees_of_freedom().collect();
        if dof.is_empty() {
            // makes no sense
            errors.push(Error::new(
                DsgProtoImplForNonProtoValue,
                format!("Cannot at a prototype implmentation block to a child of type '{}' that has no prototype components.", child.ty.valid_ident().unwrap().raw()),
                child.loc,
                false,
            ));
            return;
        }

        // check whether all protos are correctly implemented
        for (ident, proto_ty) in dof {
            let associated_ty = p.get(ident);

            let associated_ty = match associated_ty {
                Some(t) => t,
                None => {
                    errors.push(Error::new(
                        DsgProtoImplMissingField,
                        format!("Missing prototype impl field '{}'.", ident),
                        child.loc,
                        false,
                    ));
                    continue;
                }
            };

            // check for associated ty
            let assoc_ty_spec = result
                .modules
                .iter()
                .filter(|m| m.ident.asset() == asset)
                .find(|m| m.ident.raw() == *associated_ty);

            let assoc_ty_spec = match assoc_ty_spec {
                Some(s) => s,
                None => {
                    errors.push(Error::new_ty_missing(
                        DsgProtoImplTyMissing,
                        format!("Unknown type '{}'.", associated_ty),
                        child.loc,
                        smap,
                        result.module(&associated_ty[..]).map(|t| t.loc),
                    ));
                    continue;
                }
            };

            // check whether the associated type fulfills the prototype criteria
            if assoc_ty_spec.derived_from.is_none()
                || assoc_ty_spec.derived_from.as_ref().unwrap() != proto_ty
            {
                errors.push(Error::new(
                    DsgProtoImplAssociatedTyNotDerivedFromProto,
                    format!(
                        "Assigned type '{}' does not fulfill the prototype '{}'.",
                        associated_ty, proto_ty
                    ),
                    child.loc,
                    false,
                ));
            }
        }
    } else if !child.ty.is_dynamic() && child.ty.valid_ident().is_some() {
        // NO IMPL
        let ty = result
            .modules
            .iter()
            .find(|m| m.ident == *child.ty.valid_ident().unwrap())
            .map(|module| module.degrees_of_freedom().count())
            .or_else(|| {
                if is_subsystem {
                    result
                        .subsystems
                        .iter()
                        .find(|s| s.ident == *child.ty.valid_ident().unwrap())
                        .map(|subsys| subsys.degrees_of_freedom().count())
                } else {
                    None
                }
            })
            .expect("[desugar] This should have bee checked in the first pass");

        // all proto ty must have an impl
        if ty > 0 {
            // err
            errors.push(Error::new(
                DsgProtoImlMissing,
                format!(
                    "Missing prototype impl block for type '{}'.",
                    child.ty.valid_ident().unwrap().raw()
                ),
                child.loc,
                false,
            ))
        }
    }
}

pub fn tychk(resolver: &mut NdlResolver) {
    if let Some(result) = &resolver.result {
        let mut errs = validate(result, resolver);
        resolver.ectx.tychecking_errors.append(&mut errs)
    }
}

///
/// Validates the given an internal DesugaredParsingResult 'unit' using the resovler
/// as parameters.
/// Returns all sematic errors that were encountered.
///
pub fn validate(unit: &DesugaredResult, _resolver: &NdlResolver) -> Vec<Error> {
    let mut errors = Vec::new();

    //
    // === Module check ===
    //
    //

    let mut module_names = Vec::with_capacity(unit.modules.len());

    for module in &unit.modules {
        let self_ty = &module.ident;

        if module_names.contains(&self_ty) {
            // errors.push(Error::new(
            //     TycModuleAllreadyDefined,
            //     format!("Module '{}' was allready defined.", self_ty),
            //     module.loc,
            //     false,

            // ))
        } else {
            module_names.push(self_ty)
        }

        //
        // === Gate check ===
        //

        let mut self_gates = Vec::with_capacity(module.gates.len());
        for gate in &module.gates {
            if gate.size == 0 {
                errors.push(Error::new(
                    TycGateInvalidNullGate,
                    String::from("Cannot create gate of size 0."),
                    gate.loc,
                    false,
                ))
                // Still hold the descriptor to prevent transient errors
            }

            if self_gates.iter().any(|&n| n == &gate.ident) {
                // TODO: Defer to dsg else UB
                errors.push(Error::new(
                    TycGateFieldDuplication,
                    format!("Gate '{}' was allready defined.", gate.ident),
                    gate.loc,
                    false,
                ))
            } else {
                self_gates.push(&gate.ident);
            }
        }

        //
        // === Par check ===
        //

        let mut par_names = Vec::with_capacity(module.params.len());

        for par in &module.params {
            // Check ty
            if !PAR_TYPES.contains(&&par.ty[..]) {
                errors.push(Error::new(
                    TycParInvalidType,
                    format!("Parameter type '{}' does not exist.", par.ty),
                    par.loc,
                    false,
                ));
                continue;
            }

            if par_names.contains(&&par.ident) {
                errors.push(Error::new(
                    TycParAllreadyDefined,
                    format!("Parameter '{}' was already defined.", par.ident),
                    par.loc,
                    false,
                ));
                continue;
            } else {
                par_names.push(&par.ident);
            }
        }
    }

    //
    // === Network check ===
    //

    let mut network_names = Vec::with_capacity(unit.subsystems.len());

    for network in &unit.subsystems {
        let self_ty = &network.ident;

        if network_names.contains(&self_ty) {
            // errors.push(Error::new(
            //     TycNetworkAllreadyDefined,
            //     format!("Network '{}' was allready defined.", self_ty),
            //     network.loc,
            //     false,

            // ))
        } else {
            network_names.push(self_ty)
        }

        if network.nodes.is_empty() {
            errors.push(Error::new(
                TycNetworkEmptyNetwork,
                format!("Network '{}' does not contain any nodes.", self_ty.raw()),
                network.loc,
                false,
            ))
        }

        // //
        // // === Par check ===
        // //

        let mut par_names = Vec::with_capacity(network.params.len());

        for par in &network.params {
            // Check ty
            if !PAR_TYPES.contains(&&par.ty[..]) {
                errors.push(Error::new(
                    TycParInvalidType,
                    format!("Parameter type '{}' does not exist.", par.ty),
                    par.loc,
                    false,
                ));
                continue;
            }

            if par_names.contains(&&par.ident) {
                errors.push(Error::new(
                    TycParAllreadyDefined,
                    format!("Parameter '{}' was already defined.", par.ident),
                    par.loc,
                    false,
                ));
                continue;
            } else {
                par_names.push(&par.ident);
            }
        }
    }

    errors
}
