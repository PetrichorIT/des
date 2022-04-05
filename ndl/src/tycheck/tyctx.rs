use std::collections::HashMap;

use crate::*;

use crate::desugar::DesugaredParsingResult;

///
/// A global type context over the specifications stored in
/// a resolver.
///
pub struct GlobalTySpecContext<'a> {
    smap: &'a SourceMap,
    all: &'a HashMap<String, DesugaredParsingResult>,
}

impl<'a> GlobalTySpecContext<'a> {
    /// The [SourceMap] of the referenced resolver.
    pub fn source_map(&self) -> &SourceMap {
        &self.smap
    }

    ///
    /// Creates a new instance of self given a resolver ref.
    ///
    pub fn new(all: &'a HashMap<String, DesugaredParsingResult>, smap: &'a SourceMap) -> Self {
        Self { all, smap }
    }

    ///
    /// Returns a module spec with the given ident from the type context.
    ///
    pub fn module(&self, ident: &str) -> Option<&ModuleSpec> {
        for unit in self.all.values() {
            match unit.modules.iter().find(|l| ident == l.ident) {
                Some(module) => return Some(module),
                None => continue,
            }
        }
        None
    }

    ///
    /// Returns a network sepc with the given ident from the type context.
    ///
    pub fn network<T: PartialEq<String>>(&self, ident: T) -> Option<&NetworkSpec> {
        for unit in self.all.values() {
            match unit.networks.iter().find(|l| ident == l.ident) {
                Some(network) => return Some(network),
                None => continue,
            }
        }
        None
    }

    ///
    /// Extracts all specs from the current context and stores it in a [OwnedTySpecContext].
    ///
    pub fn to_owned(&self) -> OwnedTySpecContext {
        OwnedTySpecContext::new(self)
    }
}

///
/// A owned type spec context.
///
#[derive(Debug)]
pub struct OwnedTySpecContext {
    /// A collection of all included module definitions.
    pub modules: Vec<ModuleSpec>,
    /// A collection of all included network definitions.
    pub networks: Vec<NetworkSpec>,
}

impl OwnedTySpecContext {
    ///
    /// Createa a new OwnedTySpecContext from a GlobalTySpecContext.
    ///
    pub fn new(gtyctx: &GlobalTySpecContext) -> Self {
        let mut modules = Vec::new();
        let mut networks = Vec::new();

        for unit in gtyctx.all.values() {
            for module in &unit.modules {
                modules.push(module.clone())
            }
            for network in &unit.networks {
                networks.push(network.clone())
            }
        }

        Self { modules, networks }
    }

    ///
    /// Returns a module sepc with the given ident from the type context.
    ///
    pub fn module<T: PartialEq<String>>(&self, ident: T) -> Option<&ModuleSpec> {
        self.modules.iter().find(|l| ident == l.ident)
    }

    ///
    /// Returns a network sepc with the given ident from the type context.
    ///
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

    pub fn new_for(
        fpass: &'a DesugaredParsingResult,
        all: &'a HashMap<String, DesugaredParsingResult>,
    ) -> Self {
        let mut obj = Self::new();

        fn resolve_recursive<'a>(
            all: &'a HashMap<String, DesugaredParsingResult>,
            unit: &'a DesugaredParsingResult,
            tyctx: &mut TySpecContext<'a>,
        ) {
            let new_unit = tyctx.include(unit);
            if new_unit {
                // resolve meta imports.
                for include in &unit.includes {
                    if let Some(unit) = all.get(&include.path) {
                        resolve_recursive(all, unit, tyctx);
                    } else {
                        // Allready logged
                    }
                }
            }
        }

        resolve_recursive(all, fpass, &mut obj);

        obj
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
