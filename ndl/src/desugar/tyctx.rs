use crate::error::*;
use crate::parser::*;
use crate::resolver::*;
use crate::source::*;
use crate::Loc;

///
/// A global type context over the definitions stored in a resolver.
///
pub struct GlobalTyDefContext<'a> {
    resolver: &'a NdlResolver,
}

impl<'a> GlobalTyDefContext<'a> {
    /// The used [SourceMap] of the resolver.
    pub fn source_map(&self) -> &SourceMap {
        &self.resolver.source_map
    }

    ///
    /// Creates a new instance of self using the given resolver as data source.
    ///
    pub fn new(resolver: &'a NdlResolver) -> Self {
        Self { resolver }
    }

    ///
    /// Returns a link def with the given ident from the type context.
    ///
    pub fn link(&self, ident: &str) -> Option<&LinkDef> {
        for unit in self.resolver.units.values() {
            match unit.links.iter().find(|l| l.name == ident) {
                Some(link) => return Some(link),
                None => continue,
            }
        }
        None
    }

    ///
    /// REturns a module def with the given ident from the type context.
    ///
    pub fn module(&self, ident: &str) -> Option<&ModuleDef> {
        for unit in self.resolver.units.values() {
            match unit.modules_and_prototypes.iter().find(|l| l.name == ident) {
                Some(module) => return Some(module),
                None => continue,
            }
        }
        None
    }

    pub fn module_or_alias_loc(&self, ident: &str) -> Option<Loc> {
        for unit in self.resolver.units.values() {
            match unit.modules_and_prototypes.iter().find(|l| l.name == ident) {
                Some(module) => return Some(module.loc),
                None => match unit.aliases.iter().find(|a| a.name == ident) {
                    Some(alias) => return Some(alias.loc),
                    None => continue,
                },
            }
        }
        None
    }
}

///
/// A type context of non-desugared definitions.
///
#[derive(Debug)]
pub struct TyDefContext<'a> {
    /// A reference of all included assets.
    pub included: Vec<AssetDescriptor>,

    /// A collection of all included channel definitions.
    pub links: Vec<&'a LinkDef>,
    /// A collection of all included module definitions.
    pub modules_and_prototypes: Vec<&'a ModuleDef>,
    /// A collection of all included network definitions.
    pub networks: Vec<&'a NetworkDef>,
}

impl<'a> TyDefContext<'a> {
    ///
    /// Creates a new empty type context.
    ///
    pub fn new() -> Self {
        Self {
            included: Vec::new(),

            links: Vec::new(),
            modules_and_prototypes: Vec::new(),
            networks: Vec::new(),
        }
    }

    ///
    /// Createa a new instance of Self by using a resolver and a unit
    /// to include all nessecaryy units.
    ///
    pub fn new_for(
        unit: &'a ParsingResult,
        resolver: &'a NdlResolver,
        errors: &mut Vec<Error>,
    ) -> Self {
        let mut obj = TyDefContext::new();

        fn resolve_recursive<'a>(
            resolver: &'a NdlResolver,
            unit: &'a ParsingResult,
            tyctx: &mut TyDefContext<'a>,
            errors: &mut Vec<Error>,
        ) {
            let new_unit = tyctx.include(unit);
            if new_unit {
                // resolve meta imports.
                for include in &unit.includes {
                    if let Some(unit) = resolver.units.get(&include.path) {
                        resolve_recursive(resolver, unit, tyctx, errors);
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

        resolve_recursive(resolver, unit, &mut obj, errors);

        obj
    }

    ///
    /// Checks the type context for name collsions.
    ///
    pub fn check_name_collision(&self) -> Result<(), &'static str> {
        let dup_links = (1..self.links.len()).any(|i| self.links[i..].contains(&self.links[i - 1]));
        let dup_modules = (1..self.modules_and_prototypes.len()).any(|i| {
            self.modules_and_prototypes[i..].contains(&self.modules_and_prototypes[i - 1])
        });
        let dup_networks =
            (1..self.networks.len()).any(|i| self.networks[i..].contains(&self.networks[i - 1]));

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

        for module in &unit.modules_and_prototypes {
            self.modules_and_prototypes.push(module)
        }

        for network in &unit.networks {
            self.networks.push(network)
        }

        true
    }
}

impl Default for TyDefContext<'_> {
    fn default() -> Self {
        Self::new()
    }
}
