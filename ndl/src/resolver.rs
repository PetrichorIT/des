use crate::*;

use crate::desugar::{DesugaredParsingResult, GlobalTyDefContext};
use crate::error::*;
use crate::parser::ParsingResult;
use crate::tycheck::GlobalTySpecContext;

use std::fmt::Display;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
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
    /// The raw scopes of the included files.
    pub scopes: Vec<PathBuf>,
    /// The list of par files.
    pub par_files: Vec<PathBuf>,
    /// A list of all loaded assets in the current workspace.
    pub source_map: SourceMap,
    /// A list of all lexed/parsed assets in the current workspace.
    pub units: HashMap<String, ParsingResult>,
    /// A list of all lexed/parsed assets in the current workspace.
    pub desugared_units: HashMap<String, DesugaredParsingResult>,
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

    pub fn quiet(raw_path: &str) -> Result<Self, &'static str> {
        Self::new_with(
            raw_path,
            NdlResolverOptions {
                silent: true,
                verbose: false,
                verbose_output_dir: PathBuf::new(),
                desugar: true,
                tychk: true,
            },
        )
    }

    pub fn gtyctx_def(&self) -> GlobalTyDefContext<'_> {
        GlobalTyDefContext::new(self)
    }

    pub fn gtyctx_spec(&self) -> GlobalTySpecContext<'_> {
        GlobalTySpecContext::new(&self.desugared_units, &self.source_map)
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
            scopes: Vec::new(),
            par_files: Vec::new(),
            units: HashMap::new(),
            desugared_units: HashMap::new(),

            ectx: GlobalErrorContext::new(),
        })
    }

    ///
    /// Loads all assets into the source map without further processing.
    ///
    pub fn preload(&mut self) {
        self.get_ndl_scopes();

        for scope in self.scopes.iter() {
            // === Namespacing ===
            let descriptor = AssetDescriptor::from_path(scope.clone(), &self.root_dir);

            // === Asset Loading ===
            let _ = match self.source_map.load(descriptor) {
                Ok(asset) => asset,
                Err(_e) => {
                    // Log error
                    continue;
                }
            };
        }
    }

    ///
    /// Runs the parser. This creates asssets, lexes and parses them
    /// and finally typchecks and validates the results.
    ///
    /// TOOD codegen
    ///
    pub fn run(&mut self) -> Result<(), &'static str> {
        self.get_ndl_scopes();

        for scope in &self.scopes {
            // === Namespacing ===
            let descriptor = AssetDescriptor::from_path(scope.clone(), &self.root_dir);

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

            // write verbose output to file
            self.write_if_verbose(format!("{}.parse", unit.asset.alias), &unit);

            self.units.insert(unit.asset.alias.clone(), unit);
        }

        self.state = NdlResolverState::Parsed;

        // === TY DESUGAR ==

        if !self.options.desugar {
            self.print_errs();
            return Ok(());
        }

        desugar::desugar(self);

        // === TY CHECK ===

        if !self.options.tychk {
            self.print_errs();
            return Ok(());
        }

        tycheck::tychk(self);

        // === FIN ===

        self.print_errs();
        self.state = NdlResolverState::Done;

        Ok(())
    }

    fn print_errs(&self) {
        if self.ectx.has_errors() && !self.options.silent {
            let mut errs: Vec<&Error> = self.ectx.all().collect();
            errs.sort_by(|&lhs, &rhs| lhs.loc.pos.cmp(&rhs.loc.pos));

            for e in errs {
                e.print(&self.source_map)
                    .expect("Failed to write error to stderr")
            }
        }
    }

    pub fn run_cached(
        &mut self,
    ) -> Result<
        (
            GlobalTySpecContext<'_>,
            impl Iterator<Item = &Error>,
            Vec<PathBuf>,
        ),
        &'static str,
    > {
        if self.state != NdlResolverState::Done {
            self.run()?;
        }

        Ok((self.gtyctx_spec(), self.ectx.all(), self.par_files.clone()))
    }

    ///
    /// Extracts all *.ndl files from the working directory (recursivly).
    /// and all *par files.
    ///
    fn get_ndl_scopes(&mut self) {
        const TREE_SEARCH_MAX_ITR: usize = 100;

        fn recursive(path: PathBuf, itr: &mut usize, resolver: &mut NdlResolver) {
            *itr += 1;

            if *itr >= TREE_SEARCH_MAX_ITR {
                return;
            }

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "ndl" {
                        resolver.scopes.push(path)
                    } else if ext == "par" {
                        resolver.par_files.push(path)
                    }
                }
            } else if path.is_dir() {
                if let Ok(dir) = path.read_dir() {
                    for entry in dir.flatten() {
                        recursive(entry.path(), itr, resolver)
                    }
                }
            }
        }

        self.scopes = Vec::new();
        self.par_files = Vec::new();

        recursive(self.root_dir.clone(), &mut 0, self);
    }

    pub(crate) fn write_if_verbose(&self, object_name: String, object: impl Display) {
        use std::fs::*;
        use std::io::Write;

        if !self.options.verbose {
            return;
        }

        let mut path = self.options.verbose_output_dir.clone();
        path.push(object_name);

        let mut file = match File::create(path) {
            Ok(file) => file,
            Err(_) => return,
        };

        let _ = write!(file, "{}", object);
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

///
/// Options for specificing the behaviour of a resolver.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdlResolverOptions {
    pub silent: bool,
    pub verbose: bool,
    pub verbose_output_dir: PathBuf,

    pub desugar: bool,
    pub tychk: bool,
}

impl NdlResolverOptions {
    pub fn bench() -> Self {
        Self {
            silent: true,
            verbose: false,
            verbose_output_dir: PathBuf::new(),

            desugar: true,
            tychk: true,
        }
    }
}

impl Default for NdlResolverOptions {
    fn default() -> Self {
        Self {
            silent: false,
            verbose: false,
            verbose_output_dir: PathBuf::new(),
            desugar: true,
            tychk: true,
        }
    }
}
