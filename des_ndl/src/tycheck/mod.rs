use crate::SourceAsset;

use super::{
    error::{Error, ErrorCode::*},
    loc::Loc,
    parser::{LinkDef, ModuleDef, NetworkDef, ParsingResult},
    source::SourceAssetDescriptor,
    NdlResolver,
};

mod tests;

const PAR_TYPES: [&str; 15] = [
    "usize", "u8", "u16", "u32", "u64", "u128", "isize", "i8", "i16", "i32", "i64", "i128", "bool",
    "char", "String",
];

///
/// Validates the given [ParsingResult] 'unit' using the resovler and the global [TyContext]
/// as parameters.
/// Returns all sematic errors that were encountered.
///
#[allow(unused)]
pub fn validate(
    resolver: &NdlResolver,
    unit: &ParsingResult,
    global_tyctx: &TyContext,
) -> Vec<Error> {
    let mut tyctx = TyContext::new();
    let mut errors = Vec::new();
    let asset = resolver
        .assets
        .iter()
        .find(|a| a.descriptor == unit.asset)
        .unwrap();

    resolve_includes(resolver, unit, &mut tyctx, &mut errors, asset);

    match tyctx.check_name_collision() {
        Ok(()) => {
            //
            // === Module check ===
            //

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
                        let gty = global_tyctx
                            .modules
                            .iter()
                            .find(|&m| m.name == *submodule.ty);

                        errors.push(Error::new_ty_missing(
                            TycModuleSubmoduleInvalidTy,
                            format!(
                                "No module with name '{}' exists in the scope of module '{}'.",
                                submodule.ty, module.name
                            ),
                            submodule.loc,
                            asset,
                            gty,
                        ));
                    }
                }

                //
                // === Gate check ===
                //

                let mut self_gates = Vec::new();
                for gate in &module.gates {
                    if gate.size == 0 {
                        errors.push(Error::new(
                            TycGateInvalidNullGate,
                            String::from("Cannot create gate of size 0."),
                            gate.loc,
                            false,
                            asset,
                        ))
                        // Still hold the descriptor to prevent transient errors
                    }

                    if self_gates.iter().any(|&n| n == &gate.name) {
                        errors.push(Error::new(
                            TycGateFieldDuplication,
                            format!("Gate '{}' was allready defined.", gate.name),
                            gate.loc,
                            false,
                            asset,
                        ))
                    } else {
                        self_gates.push(&gate.name);
                    }
                }

                //
                // === Connection check ===
                //

                for connection in &module.connections {
                    // check channel
                    if let Some(channel) = &connection.channel {
                        let ch_valid = tyctx.links.iter().any(|&l| l.name == *channel);
                        if !ch_valid {
                            errors.push(Error::new_ty_missing(
                                TycModuleConInvalidChannelTy,
                                format!(
                                    "No channel named '{}' exists in the scope of module '{}'.",
                                    channel, module.name
                                ),
                                connection.loc,
                                asset,
                                global_tyctx.links.iter().find(|l| l.name == *channel),
                            ));
                        }
                    }

                    // check peers
                    let peers = [&connection.from, &connection.to].map(|peer| {
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

                                return None;
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
                                return None;
                            }

                            let mod_def = mod_def.unwrap();

                            let peer_subident_valid =
                                mod_def.gates.iter().find(|g| g.name == *subident);

                            if peer_subident_valid.is_none() {
                                errors.push(Error::new(
                                    TycModuleConUnknownIdentSymbol,
                                    format!(
                                        "No gate '{}' exists on submodule '{}' of type '{}'",
                                        subident, peer.ident, mod_def.name
                                    ),
                                    peer.loc,
                                    false,
                                    asset,
                                ));

                                return None;
                            }

                            peer_subident_valid
                        } else {
                            // referencing direct value
                            let peer_valid = module.gates.iter().find(|g| g.name == peer.ident);
                            if peer_valid.is_none() {
                                errors.push(Error::new(
                                    TycModuleConUnknownIdentSymbol,
                                    format!(
                                        "No gate '{}' exists on module '{}'",
                                        peer.ident, module.name
                                    ),
                                    peer.loc,
                                    false,
                                    asset,
                                ));

                                return None;
                            }

                            peer_valid
                        }
                    });

                    if let Some(from) = peers[0] {
                        if let Some(to) = peers[1] {
                            if from.size != to.size {
                                // This could only be a warning once handeling procedures
                                // are implemented

                                errors.push(
                                    Error::new(
                                    TycModuleConNonMatchingGateSizes,
                                    format!("Gates '{}' and '{}' cannot be connected since they have different sizes.", from, to),
                                    connection.loc,
                                    false,
                                    asset
                                ))
                            }
                        }
                    }
                }

                //
                // === Par check ===
                //

                let mut par_names = Vec::new();

                for par in &module.parameters {
                    // Check ty
                    if !PAR_TYPES.contains(&&par.ty[..]) {
                        errors.push(Error::new(
                            TycParInvalidType,
                            format!("Parameter type '{}' does not exist.", par.ty),
                            par.loc,
                            false,
                            asset,
                        ));
                        continue;
                    }

                    if par_names.contains(&&par.ident) {
                        errors.push(Error::new(
                            TycParAllreadyDefined,
                            format!("Parameter '{}' was already defined.", par.ident),
                            par.loc,
                            false,
                            asset,
                        ));
                        continue;
                    } else {
                        par_names.push(&par.ident);
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
    errors: &mut Vec<Error>,
    asset: &SourceAsset,
) {
    let new_unit = tyctx.include(unit);
    if new_unit {
        // resolve meta imports.
        for include in &unit.includes {
            if let Some(unit) = resolver.units.get(&include.path) {
                resolve_includes(resolver, unit, tyctx, errors, asset);
            } else {
                errors.push(Error::new(
                    TycIncludeInvalidAlias,
                    format!(
                        "Include '{}' cannot be resolved. No such file exists. {:?}",
                        include.path, include.loc
                    ),
                    include.loc,
                    false,
                    asset,
                ))
            }
        }
    }
}

///
/// A collection of all existing types available
/// in this scope.
///
#[derive(Debug)]
pub struct TyContext<'a> {
    /// A reference of all included assets.
    pub included: Vec<SourceAssetDescriptor>,

    /// A collection of all included channel definitions.
    pub links: Vec<&'a LinkDef>,
    /// A collection of all included module definitions.
    pub modules: Vec<&'a ModuleDef>,
    /// A collection of all included network definitions.
    pub networks: Vec<&'a NetworkDef>,
}

impl<'a> TyContext<'a> {
    ///
    /// Creates a new empty type context.
    ///
    pub fn new() -> Self {
        Self {
            included: Vec::new(),

            links: Vec::new(),
            modules: Vec::new(),
            networks: Vec::new(),
        }
    }

    ///
    /// Checks the type context for name collsions.
    ///
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
    /// and returns whether any new defs were added (or all was allready imported).
    ///
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

impl Default for TyContext<'_> {
    fn default() -> Self {
        Self::new()
    }
}
