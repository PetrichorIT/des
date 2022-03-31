use super::*;

pub(crate) fn second_pass(
    unit: &FirstPassDesugarResult,
    all: &HashMap<String, FirstPassDesugarResult>,
    resolver: &NdlResolver,
) -> DesugaredParsingResult {
    let mut errors = Vec::new();
    let tyctx = SecondPassTyCtx::new_for(&unit, all, &mut errors);

    let asset = unit.asset.clone();
    let mut modules = unit.modules.clone();

    // let asset = asset.clone();
    // let includes = includes.clone();
    // let networks = networks.clone();

    let gtyctx = resolver.gtyctx_def();

    //
    // Alias derefing,
    //
    for AliasDef {
        loc,
        name,
        prototype,
    } in &unit.aliases
    {
        // search for proto
        let proto = tyctx.prototypes.iter().find(|p| p.ident == *prototype);

        if let Some(proto) = proto {
            let mut proto: ModuleSpec = proto.to_owned().to_owned();
            proto.ident = name.clone();
            modules.push(proto);
        } else {
            errors.push(Error::new_ty_missing(
                DsgInvalidPrototype,
                format!("No prototype called '{}' found.", prototype),
                *loc,
                &resolver.source_map,
                gtyctx.module(&prototype).map(|m| m.loc),
            ));
        }
    }

    //
    // 'some' checking (order is important) for modules
    //
    for module in &modules {
        for child in &module.submodules {
            // PROTO DEF
            if let TySpec::Dynamic(ref s) = child.ty {
                // check whether s is a existing prototype.

                let exists = tyctx.prototypes.iter().any(|p| &p.ident == s);
                if !exists {
                    errors.push(Error::new_ty_missing(
                        DsgInvalidPrototype,
                        format!("Unknown prototype '{}'.", s),
                        child.loc,
                        &resolver.source_map,
                        gtyctx.module(s).map(|m| m.loc),
                    ))
                }
            }
        }
    }

    DesugaredParsingResult {
        asset,
        errors,
        includes: unit.includes.clone(),
        modules,
        networks: unit.networks.clone(),
    }
}

pub(crate) struct SecondPassTyCtx<'a> {
    pub included: Vec<AssetDescriptor>,

    pub prototypes: Vec<&'a ModuleSpec>,
    pub modules: Vec<&'a ModuleSpec>,
}

impl<'a> SecondPassTyCtx<'a> {
    pub fn new() -> Self {
        Self {
            included: Vec::new(),
            prototypes: Vec::new(),
            modules: Vec::new(),
        }
    }

    pub fn new_for(
        fpass: &'a FirstPassDesugarResult,
        all: &'a HashMap<String, FirstPassDesugarResult>,
        errors: &mut Vec<Error>,
    ) -> Self {
        let mut obj = Self::new();

        fn resolve_recursive<'a>(
            all: &'a HashMap<String, FirstPassDesugarResult>,
            unit: &'a FirstPassDesugarResult,
            tyctx: &mut SecondPassTyCtx<'a>,
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

    pub fn include(&mut self, fpass: &'a FirstPassDesugarResult) -> bool {
        if self.included.contains(&fpass.asset) {
            return false;
        }

        for proto in &fpass.prototypes {
            self.prototypes.push(proto)
        }

        for module in &fpass.modules {
            self.modules.push(module)
        }

        true
    }
}

impl Default for SecondPassTyCtx<'_> {
    fn default() -> Self {
        Self::new()
    }
}
