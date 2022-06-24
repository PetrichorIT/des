// use std::collections::HashMap;

use std::{cell::RefCell, collections::HashMap};

use crate::error::ErrorCode::*;
use crate::parser::*;
use crate::utils::{edit_distance, TyResolveError};
use crate::*;
use crate::{common::*, utils::TyResolveResult};

use super::{
    compose::ComposedSubsystem,
    expand::{ExpandedModule, ExpandedUnit},
};

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
            match unit.links.iter().find(|l| l.ident.raw() == ident) {
                Some(link) => return Some(link),
                None => continue,
            }
        }
        None
    }

    ///
    /// REturns a module def with the given ident from the type context.
    ///
    pub fn prototype(&self, ident: &str) -> Option<&ModuleDef> {
        for unit in self.resolver.units.values() {
            match unit.prototypes.iter().find(|l| l.ident.raw() == ident) {
                Some(module) => return Some(module),
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
            match unit.modules.iter().find(|l| l.ident.raw() == ident) {
                Some(module) => return Some(module),
                None => continue,
            }
        }
        None
    }

    pub fn module_or_alias_loc(&self, ident: &str) -> Option<Loc> {
        for unit in self.resolver.units.values() {
            match unit.modules.iter().find(|l| l.ident.raw() == ident) {
                Some(module) => return Some(module.loc),
                None => match unit.aliases.iter().find(|a| a.ident.raw() == ident) {
                    Some(alias) => return Some(alias.loc),
                    None => continue,
                },
            }
        }
        None
    }

    pub fn subsystem(&self, ident: &str) -> Option<&SubsystemDef> {
        for unit in self.resolver.units.values() {
            match unit.subsystems.iter().find(|l| l.ident.raw() == ident) {
                Some(subsystem) => return Some(subsystem),
                None => continue,
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
    pub included: Vec<(AssetDescriptor, Loc)>,

    /// A collection of all included channel definitions.
    pub links: Vec<&'a LinkDef>,
    /// A collection of all included module definitions.
    pub modules: Vec<&'a ModuleDef>,
    /// Proto
    pub prototypes: Vec<&'a ModuleDef>,
    /// A collection of all included network definitions.
    pub subsystems: Vec<&'a SubsystemDef>,
}

pub const MAX_ERROR_EDIT_DISTANCE: usize = 3;

impl<'a> TyDefContext<'a> {
    ///
    /// Creates a new empty type context.
    ///
    pub fn new() -> Self {
        Self {
            included: Vec::new(),

            links: Vec::new(),
            modules: Vec::new(),
            prototypes: Vec::new(),
            subsystems: Vec::new(),
        }
    }

    // tyctx.prototypes.iter().find(|p| p.ident.raw() == prototype)

    ///
    /// Returns a prototype or an error.
    ///
    /// Returns a lookalike if another ty is found with an ident with less that MAX_ERROR_EDIT_DISTANCE
    /// edit steps AND the number of edit steps does not exceed 50% of the total characters
    ///
    pub fn link(&self, raw_ident: &str) -> TyResolveResult<&LinkDef> {
        match self.links.iter().find(|l| l.ident.raw() == raw_ident) {
            Some(l) => Ok(l),
            None => {
                // Find best lookalike
                let lookalike = self
                    .links
                    .iter()
                    .map(|l| (l, edit_distance(l.ident.raw(), raw_ident)))
                    .filter(|(l, d)| *d < MAX_ERROR_EDIT_DISTANCE && l.ident.raw().len() > 2 * d)
                    .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                match lookalike {
                    Some((lookalike, distance)) => {
                        Err(TyResolveError::FoundLookalike(*lookalike, distance))
                    }
                    None => Err(TyResolveError::NoneFound),
                }
            }
        }
    }

    ///
    /// Returns a prototype or an error.
    ///
    /// Returns a lookalike if another ty is found with an ident with less that MAX_ERROR_EDIT_DISTANCE
    /// edit steps AND the number of edit steps does not exceed 50% of the total characters
    ///
    pub fn prototype(&self, raw_ident: &str) -> TyResolveResult<&ModuleDef> {
        match self.prototypes.iter().find(|p| p.ident.raw() == raw_ident) {
            Some(p) => Ok(p),
            None => {
                // Find best lookalike
                let lookalike = self
                    .prototypes
                    .iter()
                    .map(|p| (p, edit_distance(p.ident.raw(), raw_ident)))
                    .filter(|(p, d)| *d < MAX_ERROR_EDIT_DISTANCE && p.ident.raw().len() > 2 * d)
                    .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                match lookalike {
                    Some((lookalike, distance)) => {
                        Err(TyResolveError::FoundLookalike(*lookalike, distance))
                    }
                    None => Err(TyResolveError::NoneFound),
                }
            }
        }
    }

    // tyctx.modules.iter().find(|m| &m.ident.raw() == s)

    ///
    /// Returns the searched Module or an error.
    ///
    pub fn module(&self, raw_ident: &str) -> TyResolveResult<&ModuleDef> {
        match self.modules.iter().find(|m| m.ident.raw() == raw_ident) {
            Some(m) => Ok(m),
            None => {
                // Find best lookalike
                let lookalike = self
                    .modules
                    .iter()
                    .map(|m| (m, edit_distance(m.ident.raw(), raw_ident)))
                    .filter(|(m, d)| *d < MAX_ERROR_EDIT_DISTANCE && m.ident.raw().len() > 2 * d)
                    .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                match lookalike {
                    Some((lookalike, distance)) => {
                        Err(TyResolveError::FoundLookalike(*lookalike, distance))
                    }
                    None => Err(TyResolveError::NoneFound),
                }
            }
        }
    }

    ///
    /// Returns the searched Module or an error.
    ///
    pub fn subsystem(&self, raw_ident: &str) -> TyResolveResult<&SubsystemDef> {
        match self
            .subsystems
            .iter()
            .find(|sys| sys.ident.raw() == raw_ident)
        {
            Some(sys) => Ok(sys),
            None => {
                // Find best lookalike
                let lookalike = self
                    .subsystems
                    .iter()
                    .map(|sys| (sys, edit_distance(sys.ident.raw(), raw_ident)))
                    .filter(|(sys, d)| {
                        *d < MAX_ERROR_EDIT_DISTANCE && sys.ident.raw().len() > 2 * d
                    })
                    .min_by(|lhs, rhs| lhs.1.cmp(&rhs.1));

                match lookalike {
                    Some((lookalike, distance)) => {
                        Err(TyResolveError::FoundLookalike(*lookalike, distance))
                    }
                    None => Err(TyResolveError::NoneFound),
                }
            }
        }
    }

    pub fn module_or_proto(&self, ty: &TyDef) -> Option<&&ModuleDef> {
        match ty {
            TyDef::Static(ident) => self.modules.iter().find(|m| m.ident.raw() == *ident),
            TyDef::Dynamic(ident) => self.prototypes.iter().find(|m| m.ident.raw() == *ident),
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
                                "Include '{}' cannot be resolved. No such file exists.",
                                include.path,
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
    /// Includes all definitions from the given parsing result (by ref)
    /// and returns whether any new defs were added (or all was allready imported).
    ///
    pub fn include(&mut self, unit: &'a ParsingResult) -> bool {
        if self.included.iter().any(|(asset, _)| *asset == unit.asset) {
            return false;
        }

        self.included.push((unit.asset.clone(), unit.loc));

        for link in &unit.links {
            self.links.push(link)
        }

        for module in &unit.modules {
            self.modules.push(module)
        }

        for proto in &unit.prototypes {
            self.prototypes.push(proto)
        }

        for network in &unit.subsystems {
            self.subsystems.push(network)
        }

        true
    }
}

impl Default for TyDefContext<'_> {
    fn default() -> Self {
        Self::new()
    }
}

//
// Second pass
//

pub struct TyComposeContext<'a> {
    data: &'a HashMap<String, ExpandedUnit>,
    resolver: &'a NdlResolver,
    done: Option<&'a RefCell<Vec<ComposedSubsystem>>>,
}

impl<'a> TyComposeContext<'a> {
    pub fn new(data: &'a HashMap<String, ExpandedUnit>, resolver: &'a NdlResolver) -> Self {
        Self {
            data,
            resolver,
            done: None,
        }
    }

    pub fn source_map(&self) -> &'a SourceMap {
        &self.resolver.source_map
    }

    pub fn module(&self, ident: &OIdent) -> Option<&ExpandedModule> {
        self.data
            .iter()
            .map(|(_, unit)| unit.modules.iter())
            .flatten()
            .find(|e| e.ident == *ident)
    }

    pub fn module_or_proto(&self, ident: &OIdent) -> Option<&ExpandedModule> {
        self.data
            .iter()
            .map(|(_, unit)| unit.modules.iter().chain(&unit.prototypes))
            .flatten()
            .find(|e| e.ident == *ident)
    }

    pub fn attach(&mut self, done: &'a RefCell<Vec<ComposedSubsystem>>) {
        self.done = Some(done)
    }

    pub fn composed_subsystem_gate(
        &self,
        ident: &OIdent,
        gate: &Ident,
    ) -> Option<Option<(GateSpec, Option<usize>)>> {
        match self.done {
            Some(done) => {
                done.borrow()
                    .iter()
                    .find(|s| s.ident == *ident)
                    .map(|subsys| match gate {
                        Ident::Direct { ident } => subsys
                            .exports
                            .iter()
                            .find(|g| g.gate_ident.ident == *ident)
                            .map(|g| (g.gate_ident.clone(), None)),
                        Ident::Clustered { ident, index } => subsys
                            .exports
                            .iter()
                            .find(|g| g.gate_ident.ident == *ident)
                            .map(|g| (g.gate_ident.clone(), Some(*index))),
                    })
            }
            None => None,
        }
    }
}

// #[derive(Debug)]
// pub struct ScndPassGlobalTyCtx<'a> {
//     smap: &'a SourceMap,
//     all: &'a HashMap<String, FstPassResult>,
// }

// impl<'a> ScndPassGlobalTyCtx<'a> {
//     pub const fn new(all: &'a HashMap<String, FstPassResult>, smap: &'a SourceMap) -> Self {
//         Self { smap, all }
//     }

//     /// The used [SourceMap] of the resolver.
//     pub fn source_map(&self) -> &SourceMap {
//         self.smap
//     }

//     ///
//     /// Returns a link def with the given ident from the type context.
//     ///
//     pub fn link(&self, ident: &str) -> Option<&LinkDef> {
//         for unit in self.all.values() {
//             match unit.links.iter().find(|l| l.name == ident) {
//                 Some(link) => return Some(link),
//                 None => continue,
//             }
//         }
//         None
//     }

//     ///
//     /// REturns a module def with the given ident from the type context.
//     ///
//     pub fn module(&self, ident: &str) -> Option<&ModuleDef> {
//         for unit in self.all.values() {
//             match unit.modules.iter().find(|l| l.name == ident) {
//                 Some(module) => return Some(module),
//                 None => continue,
//             }
//         }
//         None
//     }

//     ///
//     /// REturns a module def with the given ident from the type context.
//     ///
//     pub fn subsystem(&self, ident: &str) -> Option<&SubsystemDef> {
//         for unit in self.all.values() {
//             match unit.subsystems.iter().find(|l| l.name == ident) {
//                 Some(subsys) => return Some(subsys),
//                 None => continue,
//             }
//         }
//         None
//     }
// }

// ///
// /// A type context of non-desugared definitions.
// ///
// #[derive(Debug)]
// pub struct ScndPassTyCtx<'a> {
//     /// A reference of all included assets.
//     pub included: Vec<AssetDescriptor>,

//     /// A collection of all included channel definitions.
//     pub links: Vec<&'a LinkDef>,
//     /// A collection of all included module definitions.
//     pub modules: Vec<&'a ModuleDef>,
//     /// A collection of all included network definitions.
//     pub subsystems: Vec<&'a SubsystemDef>,

//     pub prototypes: Vec<&'a ModuleDef>,
// }

// impl<'a> ScndPassTyCtx<'a> {
//     ///
//     /// Creates a new empty type context.
//     ///
//     pub fn new() -> Self {
//         Self {
//             included: Vec::new(),

//             links: Vec::new(),
//             modules: Vec::new(),
//             subsystems: Vec::new(),
//             prototypes: Vec::new(),
//         }
//     }

//     ///
//     /// Createa a new instance of Self by using a resolver and a unit
//     /// to include all nessecaryy units.
//     ///
//     pub fn new_for(
//         unit: &'a FstPassResult,
//         all: &'a HashMap<String, FstPassResult>,
//         errors: &mut Vec<Error>,
//     ) -> Self {
//         let mut obj = ScndPassTyCtx::new();

//         fn resolve_recursive<'a>(
//             all: &'a HashMap<String, FstPassResult>,
//             unit: &'a FstPassResult,
//             tyctx: &mut ScndPassTyCtx<'a>,
//             _errors: &mut Vec<Error>,
//         ) {
//             let new_unit = tyctx.include(unit);
//             if new_unit {
//                 // resolve meta imports.
//                 for include in &unit.includes {
//                     if let Some(unit) = all.get(&include.path) {
//                         resolve_recursive(all, unit, tyctx, _errors);
//                     } else {
//                         // Allready thrown

//                         // errors.push(Error::new(
//                         //     DsgIncludeInvalidAlias,
//                         //     format!(
//                         //         "Include '{}' cannot be resolved. No such file exists.",
//                         //         include.path,
//                         //     ),
//                         //     include.loc,
//                         //     false,
//                         // ))
//                     }
//                 }
//             }
//         }

//         resolve_recursive(all, unit, &mut obj, errors);

//         obj
//     }

//     pub fn check_for_name_collisions(&self, errors: &mut Vec<Error>) {
//         // check links
//         if self.links.len() >= 2 {
//             for i in 0..(self.links.len() - 1) {
//                 let link = &self.links[i];
//                 let dup = self.links[(i + 1)..].iter().find(|l| l.name == link.name);
//                 if let Some(dup) = dup {
//                     errors.push(Error::new_with_solution(
//                         DsgDefNameCollision,
//                         format!("Cannot create two links with name '{}'.", link.name),
//                         link.loc,
//                         false,
//                         ErrorSolution::new("Try renaming this link".to_string(), dup.loc),
//                     ));
//                 }
//             }
//         }

//         // check links
//         if self.modules.len() >= 2 {
//             for i in 0..(self.modules.len() - 1) {
//                 let module = &self.modules[i];
//                 let dup = self.modules[(i + 1)..]
//                     .iter()
//                     .find(|m| m.name == module.name);
//                 if let Some(dup) = dup {
//                     errors.push(Error::new_with_solution(
//                         DsgDefNameCollision,
//                         format!("Cannot create two modules with name '{}'.", module.name),
//                         module.loc,
//                         false,
//                         ErrorSolution::new("Try renaming this module".to_string(), dup.loc),
//                     ));
//                 }
//             }
//         }

//         // check links
//         if self.subsystems.len() >= 2 {
//             for i in 0..(self.subsystems.len() - 1) {
//                 let network = &self.subsystems[i];
//                 let dup = self.subsystems[(i + 1)..]
//                     .iter()
//                     .find(|n| n.name == network.name);
//                 if let Some(dup) = dup {
//                     errors.push(Error::new_with_solution(
//                         DsgDefNameCollision,
//                         format!("Cannot create two networks with name '{}'.", network.name),
//                         network.loc,
//                         false,
//                         ErrorSolution::new("Try renaming this network".to_string(), dup.loc),
//                     ));
//                 }
//             }
//         }
//     }

//     ///
//     /// Includes all definitions from the given parsing result (by ref)
//     /// and returns whether any new defs were added (or all was allready imported).
//     ///
//     pub fn include(&mut self, unit: &'a FstPassResult) -> bool {
//         if self.included.contains(&unit.asset) {
//             return false;
//         }

//         self.included.push(unit.asset.clone());

//         for link in &unit.links {
//             self.links.push(link)
//         }

//         for module in &unit.modules {
//             self.modules.push(module)
//         }

//         for proto in &unit.prototypes {
//             self.prototypes.push(proto)
//         }

//         for network in &unit.subsystems {
//             self.subsystems.push(network)
//         }

//         true
//     }

//     pub fn module_or_proto(&self, ty: &TyDef) -> Option<&&ModuleDef> {
//         match ty {
//             TyDef::Static(ident) => self.modules.iter().find(|m| m.name == *ident),
//             TyDef::Dynamic(ident) => self.prototypes.iter().find(|m| m.name == *ident),
//         }
//     }
// }

// impl Default for ScndPassTyCtx<'_> {
//     fn default() -> Self {
//         Self::new()
//     }
// }
