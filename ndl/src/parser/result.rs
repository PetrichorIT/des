use super::*;
use crate::{AssetDescriptor, Loc};

///
/// The result of parsing an asset.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsingResult {
    /// The descriptor of the asset that was parsed.
    pub asset: AssetDescriptor,

    /// The location of the referenced asset.
    pub loc: Loc,

    /// A collection of all unchecked includes.
    pub includes: Vec<IncludeDef>,
    /// A collection of all unchecked channel definitions.
    pub links: Vec<LinkDef>,
    /// A collection of all unchecked modules definitions.
    pub modules: Vec<ModuleDef>,

    /// A collection of unchecked prototypes.
    pub prototypes: Vec<ModuleDef>,

    /// A collection of all aliases refering to prototypes.
    pub aliases: Vec<AliasDef>,
    /// A collection of all unchecked network definitions.
    pub subsystems: Vec<SubsystemDef>,

    /// A list of all parsing errors that were encountered.
    pub errors: Vec<Error>,
}

impl Display for ParsingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ParsingResult {{")?;

        writeln!(f, "    includes:")?;
        for include in &self.includes {
            writeln!(f, "    - {}", include)?;
        }

        writeln!(f)?;
        writeln!(f, "    links:")?;
        for link in &self.links {
            writeln!(f, "    - {}", link)?;
        }

        writeln!(f)?;
        writeln!(f, "    modules:")?;
        for module in &self.modules {
            writeln!(
                f,
                "    - {}{} {{",
                module.ident,
                if module.is_prototype {
                    " @prototype"
                } else {
                    ""
                }
            )?;

            writeln!(f, "      submodules:")?;
            for submodule in &module.submodules {
                writeln!(f, "        {}: {}", submodule.desc, submodule.ty)?;
                if let Some(ref proto) = submodule.proto_impl {
                    for (ident, ty) in &proto.defs {
                        writeln!(f, "          -> {} = {}", ident, ty)?
                    }
                }
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

        writeln!(f)?;
        writeln!(f, "    aliases:")?;
        for alias in &self.aliases {
            writeln!(f, "    - alias {} like {}", alias.ident, alias.prototype)?
        }

        writeln!(f)?;
        writeln!(f, "    networks:")?;
        for module in &self.subsystems {
            writeln!(f, "    - {} {{", module.ident)?;

            writeln!(f, "      nodes:")?;
            for submodule in &module.nodes {
                writeln!(f, "        {} {}", submodule.ty, submodule.desc)?;
            }

            writeln!(f)?;
            writeln!(f, "      exports:")?;
            for exp in &module.exports {
                writeln!(f, "        {}", exp)?;
            }

            writeln!(f)?;
            writeln!(f, "      connections:")?;
            for con in &module.connections {
                writeln!(f, "        {}", con)?;
            }

            writeln!(f, "    }}")?;
        }

        write!(f, "}}")
    }
}
