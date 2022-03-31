use crate::source::*;

use super::*;

///
/// A raw specification of a assets defined modules, networks and includes.
///
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FirstPassDesugarResult<'a> {
    pub unit: &'a ParsingResult,

    /// The asset the [ParsingResult] was derived from.
    pub asset: AssetDescriptor,

    /// The errors that occured while desugaring,
    pub errors: Vec<Error>,

    /// The direct includes of the asset.
    pub includes: Vec<IncludeSpec>,
    /// The defined modules of the asset.
    pub modules: Vec<ModuleSpec>, // Link specs are removed and link data is stored directly in connections.

    pub prototypes: Vec<ModuleSpec>,
    pub aliases: Vec<AliasDef>,

    /// The defined networks of the asset.
    pub networks: Vec<NetworkSpec>,
}

impl<'a> FirstPassDesugarResult<'a> {
    ///
    /// Creates a new instance of Self, by referencing the [ParsingResult]
    /// to be desugared.
    ///
    pub(crate) fn new(unit: &'a ParsingResult) -> Self {
        Self {
            unit,
            asset: unit.asset.clone(),

            errors: Vec::new(),

            includes: Vec::with_capacity(unit.includes.len()),
            modules: Vec::with_capacity(unit.modules_and_prototypes.len()),
            prototypes: Vec::with_capacity(unit.modules_and_prototypes.len() / 2),
            aliases: Vec::with_capacity(unit.aliases.len()),
            networks: Vec::with_capacity(unit.networks.len()),
        }
    }
}

impl Display for FirstPassDesugarResult<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DesugaredParsingResult {{")?;

        if !self.includes.is_empty() {
            writeln!(f, "    includes:")?;
            for include in &self.includes {
                writeln!(f, "    - {}", include)?;
            }
        }
        if !self.modules.is_empty() {
            writeln!(f)?;
            writeln!(f, "    modules:")?;
            for module in &self.modules {
                writeln!(f, "    - {} {{", module.ident)?;

                writeln!(f, "      submodules:")?;
                for submodule in &module.submodules {
                    writeln!(f, "        {} {}", submodule.ty, submodule.descriptor)?;
                }

                writeln!(f)?;
                writeln!(f, "      gates:")?;
                for gate in &module.gates {
                    writeln!(f, "        {}", gate)?;
                }

                writeln!(f)?;
                writeln!(f, "      connections:")?;
                for con in &module.connections {
                    writeln!(f, "        {}", con)?;
                }

                writeln!(f, "    }}")?;
            }
        }

        if !self.prototypes.is_empty() {
            writeln!(f)?;
            writeln!(f, "    prototypes:")?;
            for proto in &self.prototypes {
                writeln!(f, "    - {} {{", proto.ident)?;

                writeln!(f, "      submodules:")?;
                for submodule in &proto.submodules {
                    writeln!(f, "        {} {}", submodule.ty, submodule.descriptor)?;
                }

                writeln!(f)?;
                writeln!(f, "      gates:")?;
                for gate in &proto.gates {
                    writeln!(f, "        {}", gate)?;
                }

                writeln!(f)?;
                writeln!(f, "      connections:")?;
                for con in &proto.connections {
                    writeln!(f, "        {}", con)?;
                }

                writeln!(f, "    }}")?;
            }
        }

        if !self.aliases.is_empty() {
            writeln!(f)?;
            writeln!(f, "    aliases:")?;
            for alias in &self.aliases {
                writeln!(f, "    - alias {} like {}", alias.name, alias.prototype)?
            }
        }

        if !self.networks.is_empty() {
            writeln!(f)?;
            writeln!(f, "    networks:")?;
            for module in &self.networks {
                writeln!(f, "    - {} {{", module.ident)?;

                writeln!(f, "      nodes:")?;
                for submodule in &module.nodes {
                    writeln!(f, "        {} {}", submodule.ty, submodule.descriptor)?;
                }

                writeln!(f)?;
                writeln!(f, "      connections:")?;
                for con in &module.connections {
                    writeln!(f, "        {}", con)?;
                }

                writeln!(f, "    }}")?;
            }
        }

        write!(f, "}}")
    }
}

///
/// A raw specification of a assets defined modules, networks and includes.
///
#[derive(Debug, Clone, PartialEq)]
pub struct DesugaredParsingResult {
    /// The asset the [ParsingResult] was derived from.
    pub asset: AssetDescriptor,

    /// The errors that occured while desugaring,
    pub errors: Vec<Error>,

    /// The direct includes of the asset.
    pub includes: Vec<IncludeSpec>,
    /// The defined modules of the asset.
    pub modules: Vec<ModuleSpec>, // Link specs are removed and link data is stored directly in connections.
    /// The defined networks of the asset.
    pub networks: Vec<NetworkSpec>,
}

impl Display for DesugaredParsingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DesugaredParsingResult {{")?;

        if !self.includes.is_empty() {
            writeln!(f, "<< includes >>")?;
            for include in &self.includes {
                writeln!(f, "- {}", include)?;
            }
        }

        if !self.modules.is_empty() {
            writeln!(f, "<< modules >>")?;
            for module in &self.modules {
                writeln!(f, "{}", module)?
            }
        }

        if !self.networks.is_empty() {
            writeln!(f, "<< networks >>")?;
            for network in &self.networks {
                writeln!(f, "{}", network)?
            }
        }

        write!(f, "}}")
    }
}
