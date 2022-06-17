use super::*;
use crate::{AssetDescriptor, Error};
use std::fmt::Display;

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
    pub networks: Vec<SubsystemSpec>,
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
