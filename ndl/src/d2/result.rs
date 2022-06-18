use crate::{common::OType, Error};

use super::compose::{ComposedModule, ComposedSubsystem};

#[derive(Debug, Clone)]
pub struct DesugaredResult {
    pub modules: Vec<ComposedModule>,
    pub subsystems: Vec<ComposedSubsystem>,

    pub(crate) errors: Vec<Error>,
}

impl DesugaredResult {
    ///
    /// Returns a module spec with the given ident from the type context.
    ///
    pub fn module(&self, ident: &str) -> Option<&ComposedModule> {
        self.modules
            .iter()
            .find(|m| m.ident.raw() == ident && m.ident.typ() == OType::Module)
    }

    ///
    /// Returns a network sepc with the given ident from the type context.
    ///
    pub fn subsystem(&self, ident: &str) -> Option<&ComposedSubsystem> {
        self.subsystems
            .iter()
            .find(|m| m.ident.raw() == ident && m.ident.typ() == OType::Subsystem)
    }
}
