use super::{
    error::{Error, ErrorCode::*},
    loc::Loc,
    parser::{LinkDef, ModuleDef, NetworkDef, ParsingResult},
    souce::SourceAssetDescriptor,
    NdlResolver,
};

mod tests;

#[allow(unused)]
pub(crate) fn validate(
    resolver: &NdlResolver,
    unit: &ParsingResult,
    global_tyctx: &TyContext,
) -> Vec<Error> {
    let mut tyctx = TyContext::new();
    resolve_includes(resolver, unit, &mut tyctx);

    let asset = resolver
        .assets
        .iter()
        .find(|a| a.descriptor == unit.asset)
        .unwrap();
    let mut errors = Vec::new();

    match tyctx.check_name_collision() {
        Ok(()) => {
            for module in &unit.modules {
                let self_ty = &module.name;
                // Check submodule namespaces and types
                let mut descriptors = Vec::new();

                for submodule in &module.submodule {
                    if descriptors.contains(&&submodule.descriptor) {
                        errors.push(Error::new(
                            TycModuleSubmoduleFieldAlreadyDeclared,
                            format!("Field '{}' was already declared.", submodule.descriptor),
                            submodule.loc,
                            false,
                            asset,
                        ));
                    }
                    descriptors.push(&submodule.descriptor);

                    let ty_valid = tyctx.modules.iter().any(|&m| m.name == *submodule.ty);

                    if submodule.ty == *self_ty {
                        errors.push(Error::new(
                            TycModuleSubmoduleRecrusiveTyDefinition,
                            format!("Module '{0}' has a required submodule of type '{0}'. Cannot create cyclic definitions.", submodule.ty),
                            submodule.loc,
                            false,
                            asset
                        ))
                    } else if !ty_valid {
                        if let Some(gty) = global_tyctx
                            .modules
                            .iter()
                            .find(|&m| m.name == *submodule.ty)
                        {
                            errors.push(Error::new(
                                TycModuleSubmoduleInvalidTy,
                                format!(
                                    "No module '{}' does not exist for module '{}'. Try including '{}'",
                                    submodule.ty, module.name, gty.asset.alias
                                ),
                                submodule.loc,
                                false,
                                asset,
                            ))
                        } else {
                            errors.push(Error::new(
                                TycModuleSubmoduleInvalidTy,
                                format!(
                                    "No module '{}' does not exist for module {}",
                                    submodule.ty, module.name
                                ),
                                submodule.loc,
                                false,
                                asset,
                            ))
                        }
                    }
                }

                //

                // Check connection definition.

                for connection in &module.connections {
                    // check channel
                    if let Some(channel) = &connection.channel {
                        let ch_valid = tyctx.links.iter().any(|&l| l.name == *channel);
                        if !ch_valid {
                            if let Some(gty) =
                                global_tyctx.links.iter().find(|l| l.name == *channel)
                            {
                                errors.push(Error::new(
                                    TycModuleConInvalidChannelTy,
                                    format!(
                                        "No channel '{}' exists for module {}. Try including '{}'.",
                                        channel, module.name, gty.asset.alias
                                    ),
                                    connection.loc,
                                    false,
                                    asset,
                                ))
                            } else {
                                errors.push(Error::new(
                                    TycModuleConInvalidChannelTy,
                                    format!(
                                        "No channel '{}' exists for module {}.",
                                        channel, module.name
                                    ),
                                    connection.loc,
                                    false,
                                    asset,
                                ))
                            }
                        }
                    }

                    // check peers
                    for peer in [&connection.from, &connection.to] {
                        if let Some(subident) = &peer.subident {
                            // Referencing subvalue
                            let peer_ident_valid = descriptors.contains(&&peer.ident);
                            if !peer_ident_valid {
                                errors.push(Error::new(
                                    TycModuleConUnknownIdentSymbol,
                                    format!(
                                        "No submodule '{}' exists on module '{}'",
                                        peer.ident, module.name
                                    ),
                                    peer.loc,
                                    false,
                                    asset,
                                ));

                                continue;
                            }

                            let submod = module
                                .submodule
                                .iter()
                                .find(|&m| m.descriptor == peer.ident)
                                .unwrap();

                            let mod_def = tyctx.modules.iter().find(|m| m.name == submod.ty);

                            if mod_def.is_none() {
                                // referenced submodule has invalid ty
                                // this error was already handled
                                continue;
                            }

                            let mod_def = mod_def.unwrap();

                            let peer_subident_valid =
                                mod_def.gates.iter().any(|g| g.name == *subident);

                            if !peer_subident_valid {
                                errors.push(Error::new(
                                    TycModuleConUnknownIdentSymbol,
                                    format!(
                                        "No gate '{}' exists on submodule '{}' of type '{}'",
                                        subident, peer.ident, mod_def.name
                                    ),
                                    peer.loc,
                                    false,
                                    asset,
                                ))
                            }
                        } else {
                            // referencing direct value
                            let peer_valid = module.gates.iter().any(|g| g.name == peer.ident);
                            if !peer_valid {
                                errors.push(Error::new(
                                    TycModuleConUnknownIdentSymbol,
                                    format!(
                                        "No gate '{}' exists on module '{}'",
                                        peer.ident, module.name
                                    ),
                                    peer.loc,
                                    false,
                                    asset,
                                ))
                            }
                        }
                    }
                }
            }
        }
        Err(_e) => errors.push(Error::new(
            TycDefNameCollission,
            format!("Name collision in '{}'", unit.asset.alias),
            Loc::new(0, 0, 1),
            false,
            asset,
        )),
    }

    errors
}

fn resolve_includes<'a>(
    resolver: &'a NdlResolver,
    unit: &'a ParsingResult,
    tyctx: &mut TyContext<'a>,
) {
    let new_unit = tyctx.include(unit);
    if new_unit {
        // resolve meta imports.
        for include in &unit.includes {
            resolve_includes(resolver, resolver.units.get(&include.path).unwrap(), tyctx);
        }
    }
}

#[derive(Debug)]
pub(crate) struct TyContext<'a> {
    pub included: Vec<SourceAssetDescriptor>,

    pub links: Vec<&'a LinkDef>,
    pub modules: Vec<&'a ModuleDef>,
    pub networks: Vec<&'a NetworkDef>,
}

impl<'a> TyContext<'a> {
    pub fn new() -> Self {
        Self {
            included: Vec::new(),

            links: Vec::new(),
            modules: Vec::new(),
            networks: Vec::new(),
        }
    }

    pub fn check_name_collision(&self) -> Result<(), &'static str> {
        let dup_links = (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));
        let dup_modules =
            (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));
        let dup_networks =
            (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));

        if dup_links || dup_modules || dup_networks {
            Err("Found duplicated symbols")
        } else {
            Ok(())
        }
    }

    ///
    /// Includes all definitions from the given parsing result (by ref)
    /// and returns whether any new defs were added (or all was allready imported)
    pub fn include(&mut self, unit: &'a ParsingResult) -> bool {
        if self.included.contains(&unit.asset) {
            return false;
        }

        self.included.push(unit.asset.clone());

        for link in &unit.links {
            self.links.push(link)
        }

        for module in &unit.modules {
            self.modules.push(module)
        }

        for network in &unit.networks {
            self.networks.push(network)
        }

        true
    }
}
