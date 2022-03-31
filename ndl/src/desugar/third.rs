use super::*;

///
/// ONlY A CHECK PASS
///
pub fn third_pass(
    unit: &DesugaredParsingResult,
    all: &HashMap<String, DesugaredParsingResult>,
    resolver: &NdlResolver,
) -> Vec<Error> {
    let mut errors = Vec::new();
    let tyctx = ThirdPassTyCtx::new_for(unit, all, &mut errors);
    let gtyctx = GlobalTyDefContext::new(resolver);

    //
    // 'some' checking (order is important) for modules
    //
    for module in &unit.modules {
        for child in &module.submodules {
            // first get the ty of the child

            // PROTO IMPL
            if let Some(ref p) = child.proto_impl {
                let ty = tyctx
                    .modules
                    .iter()
                    .find(|m| m.ident == child.ty.inner())
                    .expect("[desugar] This should have bee checked in the first pass");

                // check whether a proto impl makes any sense
                let dof: Vec<(&String, &String)> = ty.degrees_of_freedom().collect();
                if dof.is_empty() {
                    // makes no sense
                    errors.push(Error::new(
                        DsgProtoImplForNonProtoValue,
                        format!("Cannot at a prototype implmentation block to a child of type '{}' that has no prototype components.", child.ty.inner()),
                        child.loc,
                        false,
                    ));
                    continue;
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
                    let assoc_ty_spec = tyctx.modules.iter().find(|m| m.ident == *associated_ty);

                    let assoc_ty_spec = match assoc_ty_spec {
                        Some(s) => s,
                        None => {
                            errors.push(Error::new_ty_missing(
                                DsgProtoImplTyMissing,
                                format!("Unknown type '{}'.", associated_ty),
                                child.loc,
                                &resolver.source_map,
                                gtyctx.module_or_alias_loc(associated_ty),
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
            } else if !child.ty.is_dynamic() {
                // NO IMPL
                let ty = tyctx
                    .modules
                    .iter()
                    .find(|m| m.ident == child.ty.inner())
                    .expect("[desugar] This should have bee checked in the first pass");

                // all proto ty must have an impl
                if ty.degrees_of_freedom().count() > 0 {
                    // err
                    errors.push(Error::new(
                        DsgProtoImlMissing,
                        format!(
                            "Missing prototype impl block for type '{}'.",
                            child.ty.inner()
                        ),
                        child.loc,
                        false,
                    ))
                }
            }
        }
    }

    //
    // 'some' checking (order is important) for modules
    //
    for network in &unit.networks {
        for child in &network.nodes {
            // first get the ty of the child

            // PROTO IMPL
            if let Some(ref p) = child.proto_impl {
                let ty = tyctx
                    .modules
                    .iter()
                    .find(|m| m.ident == child.ty.inner())
                    .expect("[desugar] This should have bee checked in the first pass");

                // check whether a proto impl makes any sense
                let dof: Vec<(&String, &String)> = ty.degrees_of_freedom().collect();
                if dof.is_empty() {
                    // makes no sense
                    errors.push(Error::new(
                        DsgProtoImplForNonProtoValue,
                        format!("Cannot at a prototype implmentation block to a child of type '{}' that has no prototype components.", child.ty.inner()),
                        child.loc,
                        false,
                    ));
                    continue;
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
                    let assoc_ty_spec = tyctx.modules.iter().find(|m| m.ident == *associated_ty);

                    let assoc_ty_spec = match assoc_ty_spec {
                        Some(s) => s,
                        None => {
                            errors.push(Error::new_ty_missing(
                                DsgProtoImplTyMissing,
                                format!("Unknown type '{}'.", associated_ty),
                                child.loc,
                                &resolver.source_map,
                                gtyctx.module_or_alias_loc(associated_ty),
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
            } else if !child.ty.is_dynamic() {
                // NO IMPL
                let ty = tyctx
                    .modules
                    .iter()
                    .find(|m| m.ident == child.ty.inner())
                    .expect("[desugar] This should have bee checked in the first pass");

                // all proto ty must have an impl
                if ty.degrees_of_freedom().count() > 0 {
                    // err
                    errors.push(Error::new(
                        DsgProtoImlMissing,
                        format!(
                            "Missing prototype impl block for type '{}'.",
                            child.ty.inner()
                        ),
                        child.loc,
                        false,
                    ))
                }
            }
        }
    }

    errors
}

pub(crate) struct ThirdPassTyCtx<'a> {
    pub included: Vec<AssetDescriptor>,

    pub modules: Vec<&'a ModuleSpec>,
}

impl<'a> ThirdPassTyCtx<'a> {
    pub fn new() -> Self {
        Self {
            included: Vec::new(),

            modules: Vec::new(),
        }
    }

    pub fn new_for(
        fpass: &'a DesugaredParsingResult,
        all: &'a HashMap<String, DesugaredParsingResult>,
        errors: &mut Vec<Error>,
    ) -> Self {
        let mut obj = Self::new();

        fn resolve_recursive<'a>(
            all: &'a HashMap<String, DesugaredParsingResult>,
            unit: &'a DesugaredParsingResult,
            tyctx: &mut ThirdPassTyCtx<'a>,
            errors: &mut Vec<Error>,
        ) {
            let new_unit = tyctx.include(unit);
            if new_unit {
                // resolve meta imports.
                for include in &unit.includes {
                    if let Some(unit) = all.get(&include.path) {
                        resolve_recursive(all, unit, tyctx, errors);
                    } else {
                        errors.push(Error::new(
                            DsgIncludeInvalidAlias,
                            format!(
                                "Include '{}' cannot be resolved. No such file exists. {:?}",
                                include.path, include.loc
                            ),
                            include.loc,
                            false,
                        ))
                    }
                }
            }
        }

        resolve_recursive(all, fpass, &mut obj, errors);

        obj
    }

    pub fn include(&mut self, fpass: &'a DesugaredParsingResult) -> bool {
        if self.included.contains(&fpass.asset) {
            return false;
        }

        for module in &fpass.modules {
            self.modules.push(module)
        }

        true
    }
}

impl Default for ThirdPassTyCtx<'_> {
    fn default() -> Self {
        Self::new()
    }
}
