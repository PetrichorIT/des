use crate::*;

use crate::desugar::DesugaredParsingResult;

pub struct GlobalTySpecContext<'a> {
    resolver: &'a NdlResolver,
}

impl<'a> GlobalTySpecContext<'a> {
    pub fn source_map(&self) -> &SourceMap {
        &self.resolver.source_map
    }

    pub fn new(resolver: &'a NdlResolver) -> Self {
        Self { resolver }
    }

    pub fn module<T: PartialEq<String>>(&self, ident: T) -> Option<&ModuleSpec> {
        for unit in self.resolver.desugared_units.values() {
            match unit.modules.iter().find(|l| ident == l.ident) {
                Some(module) => return Some(module),
                None => continue,
            }
        }
        None
    }

    pub fn network<T: PartialEq<String>>(&self, ident: T) -> Option<&NetworkSpec> {
        for unit in self.resolver.desugared_units.values() {
            match unit.networks.iter().find(|l| ident == l.ident) {
                Some(network) => return Some(network),
                None => continue,
            }
        }
        None
    }

    pub fn to_owned(&self) -> OwnedTySpecContext {
        OwnedTySpecContext::new(self)
    }
}

pub struct OwnedTySpecContext {
    /// A collection of all included module definitions.
    pub modules: Vec<ModuleSpec>,
    /// A collection of all included network definitions.
    pub networks: Vec<NetworkSpec>,
}

impl OwnedTySpecContext {
    pub fn new(gtyctx: &GlobalTySpecContext) -> Self {
        let mut modules = Vec::new();
        let mut networks = Vec::new();

        for unit in gtyctx.resolver.desugared_units.values() {
            for module in &unit.modules {
                modules.push(module.clone())
            }
            for network in &unit.networks {
                networks.push(network.clone())
            }
        }

        Self { modules, networks }
    }

    pub fn module<T: PartialEq<String>>(&self, ident: T) -> Option<&ModuleSpec> {
        self.modules.iter().find(|l| ident == l.ident)
    }

    pub fn network<T: PartialEq<String>>(&self, ident: T) -> Option<&NetworkSpec> {
        self.networks.iter().find(|l| ident == l.ident)
    }
}

///
/// A collection of all existing types available
/// in this scope.
///
#[derive(Debug)]
pub struct TySpecContext<'a> {
    /// A reference of all included assets.
    pub included: Vec<AssetDescriptor>,

    /// A collection of all included module definitions.
    pub modules: Vec<&'a ModuleSpec>,
    /// A collection of all included network definitions.
    pub networks: Vec<&'a NetworkSpec>,
}

impl<'a> TySpecContext<'a> {
    ///
    /// Creates a new empty type context.
    ///
    pub fn new() -> Self {
        Self {
            included: Vec::new(),

            modules: Vec::new(),
            networks: Vec::new(),
        }
    }

    ///
    /// Checks the type context for name collsions.
    ///
    pub fn check_name_collision(&self) -> Result<(), &'static str> {
        let dup_modules =
            (1..self.modules.len()).any(|i| self.modules[i..].contains(&self.modules[i - 1]));
        let dup_networks =
            (1..self.networks.len()).any(|i| self.networks[i..].contains(&self.networks[i - 1]));

        if dup_modules || dup_networks {
            Err("Found duplicated symbols")
        } else {
            Ok(())
        }
    }

    ///
    /// Includes all definitions from the given parsing result (by ref)
    /// and returns whether any new defs were added (or all was allready imported).
    ///
    pub fn include(&mut self, unit: &'a DesugaredParsingResult) -> bool {
        if self.included.contains(&unit.asset) {
            return false;
        }

        self.included.push(unit.asset.clone());

        for module in &unit.modules {
            self.modules.push(module)
        }

        for network in &unit.networks {
            self.networks.push(network)
        }

        true
    }
}

impl Default for TySpecContext<'_> {
    fn default() -> Self {
        Self::new()
    }
}
