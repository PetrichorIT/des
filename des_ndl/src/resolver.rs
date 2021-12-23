use std::fmt::Display;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    parse, tokenize_and_validate, validate, AssetDescriptor, Error, ErrorCode, GlobalErrorContext,
    ParsingResult, SourceMap, TyContext,
};

///
/// The primary entry point for comipling a NDL
/// workspace.
///
#[derive(Debug, Clone)]
pub struct NdlResolver {
    /// A indicator in which state the resolver is / may have stopped.
    pub state: NdlResolverState,
    /// The config of the resolver.
    pub options: NdlResolverOptions,
    /// The root directory of the NDL workspace.
    pub root_dir: PathBuf,
    /// A list of all loaded assets in the current workspace.
    pub source_map: SourceMap,
    /// A list of all lexed/parsed assets in the current workspace.
    pub units: HashMap<String, ParsingResult>,
    /// An error handler to record errors on the way.
    pub ectx: GlobalErrorContext,
}

impl NdlResolver {
    ///
    /// Creats a new resolver of the given workspace directory.
    ///
    pub fn new(raw_path: &str) -> Result<Self, &'static str> {
        Self::new_with(raw_path, NdlResolverOptions::default())
    }

    ///
    /// Creats a new resolver of the given workspace directory adn options.
    ///
    pub fn new_with(raw_path: &str, options: NdlResolverOptions) -> Result<Self, &'static str> {
        let root_dir = Path::new(raw_path).to_owned();
        if !root_dir.exists() {
            return Err("Resolver must point to valid root.");
        }

        Ok(Self {
            state: NdlResolverState::Idle,
            options,

            source_map: SourceMap::new(),
            root_dir,
            units: HashMap::new(),

            ectx: GlobalErrorContext::new(),
        })
    }

    ///
    /// Runs the parser. This creates asssets, lexes and parses them
    /// and finally typchecks and validates the results.
    ///
    /// TOOD codegen
    ///
    pub fn run(&mut self) -> Result<TyContext, &'static str> {
        let scopes = self.get_ndl_scopes();

        for scope in scopes {
            // === Namespacing ===
            let descriptor = AssetDescriptor::from_path(scope, &self.root_dir);

            // === Asset Loading ===
            let asset = match self.source_map.load(descriptor) {
                Ok(asset) => asset,
                Err(_e) => {
                    // Log error
                    continue;
                }
            };

            // === Lexing ===

            let validated_token_stream = match tokenize_and_validate(asset, &mut self.ectx) {
                Ok(v) => v,
                Err(_e) => {
                    self.ectx.lexing_errors.push(Error::new(
                        ErrorCode::TooManyErrors,
                        format!("Too many errors in '{}'", asset.descriptor().alias),
                        asset.start_loc(),
                        false,
                    ));
                    continue;
                }
            };

            // === Compile Unit parsing

            let unit = parse(asset, validated_token_stream);
            self.ectx.parsing_errors.append(&mut unit.errors.clone());

            self.units.insert(unit.asset.alias.clone(), unit);
        }

        self.state = NdlResolverState::Parsed;

        // === TY CHECK ===

        let mut global_tyctx = TyContext::new();
        self.units.values().for_each(|unit| {
            let _ = global_tyctx.include(unit);
        });

        for unit in self.units.values() {
            self.ectx
                .tychecking_errors
                .append(&mut validate(self, unit, &global_tyctx))
        }

        // === FIN ===

        if self.ectx.has_errors() && !self.options.silent {
            let mut errs = Vec::new();

            for le in &self.ectx.lexing_errors {
                errs.push(le)
            }
            for pe in &self.ectx.parsing_errors {
                errs.push(pe)
            }
            for te in &self.ectx.tychecking_errors {
                errs.push(te)
            }

            errs.sort_by(|&lhs, &rhs| lhs.loc.pos.cmp(&rhs.loc.pos));

            for e in errs {
                e.print(&self.source_map)
                    .expect("Failed to write error to stderr")
            }
        }

        Ok(global_tyctx)
    }

    ///
    /// Extracts all *.ndl files from the working directory (recursivly).
    ///
    fn get_ndl_scopes(&self) -> Vec<PathBuf> {
        const TREE_SEARCH_MAX_ITR: usize = 100;

        fn recusive(path: PathBuf, itr: &mut usize, results: &mut Vec<PathBuf>) {
            *itr += 1;

            if *itr >= TREE_SEARCH_MAX_ITR {
                return;
            }

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "ndl" {
                        results.push(path)
                    }
                }
            } else if path.is_dir() {
                if let Ok(dir) = path.read_dir() {
                    for entry in dir.flatten() {
                        recusive(entry.path(), itr, results)
                    }
                }
            }
        }

        let mut results = Vec::new();
        recusive(self.root_dir.clone(), &mut 0, &mut results);
        results
    }
}

impl Display for NdlResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== NDL RESOLVER ({:?}) ===", self.state)?;
        writeln!(f, "root: {:?}", self.root_dir)?;
        writeln!(
            f,
            "assets: {} units: {}",
            self.source_map.len_assets(),
            self.units.len()
        )?;

        for (k, v) in &self.units {
            writeln!(f, "Scope '{}'", k)?;
            writeln!(f, "{}", v)?;
        }

        Ok(())
    }
}

///
/// The state a NDL resolver is currently in.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NdlResolverState {
    /// The resolver was created and the workspace exists, but was not yet mapped.
    Idle,
    /// All *.ndl files in the workspace have been mapped, lexed and parsed.
    Parsed,
    /// All *.ndl files in the workspace have been validated.
    Validated,
    /// The resolver finished.
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NdlResolverOptions {
    pub silent: bool,
}

impl NdlResolverOptions {
    pub fn bench() -> Self {
        Self { silent: true }
    }
}

mod tests {
    #[test]
    fn it_works() {
        use super::*;

        let mut resolver = NdlResolver::new("./examples").expect("Failed to create resolver");

        println!("{}", resolver);

        let _ = resolver.run();

        println!("{}", resolver);
    }
}
