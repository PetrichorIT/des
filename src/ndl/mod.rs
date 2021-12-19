use std::collections::VecDeque;
use std::fmt::Display;
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

use self::error::{Error, ErrorCode::*, GlobalErrorContext};
use self::tycheck::TyContext;
use self::{parser::ParsingResult, souce::SourceAsset};

pub mod error;
pub mod loc;
pub mod souce;

pub mod lexer;
pub mod parser;
pub mod tycheck;

#[derive(Debug, Clone)]
pub struct NdlResolver {
    pub(crate) state: NdlResolverState,
    pub(crate) root_dir: PathBuf,
    pub(crate) assets: Vec<SourceAsset>,
    pub(crate) units: HashMap<String, ParsingResult>,
    pub(crate) ectx: GlobalErrorContext,
}

impl NdlResolver {
    pub fn new(raw_path: &str) -> Result<Self, &'static str> {
        let root_dir = Path::new(raw_path).to_owned();
        if !root_dir.exists() {
            return Err("Resolver must point to valid root.");
        }

        Ok(Self {
            state: NdlResolverState::Idle,

            assets: Vec::new(),
            root_dir,
            units: HashMap::new(),

            ectx: GlobalErrorContext::new(),
        })
    }

    pub fn run(&mut self) {
        let scopes = self.get_ndl_scopes();

        let root_len = self.root_dir.components().count();
        for scope in scopes {
            // === Namespacing ===
            let components = scope.components().collect::<Vec<Component>>();
            let naming_subset = &components[root_len..]
                .iter()
                .filter_map(|c| match c {
                    Component::Normal(str) => Some(str.to_str()?),
                    _ => None,
                })
                .collect::<Vec<&str>>();

            let mut name = naming_subset.join("/");
            name.truncate(name.len() - 4);

            // === Asset Loading ===
            let asset = match SourceAsset::load(scope, name) {
                Ok(asset) => asset,
                Err(_e) => {
                    // Log error
                    continue;
                }
            };

            let token_stream = lexer::tokenize(&asset.data);
            // Validated token stream
            let mut validated_token_stream = VecDeque::new();

            for token in token_stream {
                if !token.kind.valid() {
                    if matches!(token.kind, lexer::TokenKind::InvalidIdent) {
                        self.ectx.lexing_errors.push(Error {
                            code: LexInvalidSouceIdentifier,
                            msg: String::from("Found invalid identifer in token stream"),

                            loc: token.loc,
                            asset: asset.descriptor.clone(),
                            source: token
                                .loc
                                .padded_referenced_slice_in(&asset.data)
                                .to_string(),

                            transient: false,
                        })
                    } else {
                        self.ectx.lexing_errors.push(Error {
                            code: LexInvalidSouceToken,
                            msg: String::from("Found invalid token in token stream"),

                            loc: token.loc,
                            asset: asset.descriptor.clone(),
                            source: token
                                .loc
                                .padded_referenced_slice_in(&asset.data)
                                .to_string(),

                            transient: false,
                        })
                    }
                    continue;
                }

                if !token.kind.reducable() {
                    validated_token_stream.push_back(token)
                }
            }

            // === Compile Unit parsing

            let unit = parser::parse(&asset, validated_token_stream);
            self.ectx.parsing_errors.append(&mut unit.errors.clone());

            self.assets.push(asset);
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
                .append(&mut tycheck::validate(self, unit, &global_tyctx))
        }

        // === FIN ===

        if self.ectx.has_errors() {
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

            errs.sort_by(|&lhs, &rhs| lhs.asset.alias.cmp(&rhs.asset.alias));

            for e in errs {
                e.print().expect("Failed to write error to stderr")
            }
        }
    }

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

    pub fn validate(&self) {}
}

impl Display for NdlResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== NDL RESOLVER ({:?}) ===", self.state)?;
        writeln!(f, "root: {:?}", self.root_dir)?;
        writeln!(
            f,
            "assets: {} units: {}",
            self.assets.len(),
            self.units.len()
        )?;

        for (k, v) in &self.units {
            writeln!(f, "Scope '{}'", k)?;
            writeln!(f, "{}", v)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NdlResolverState {
    Idle,
    Parsed,
    Validated,
    Done,
}

#[test]
fn it_works() {
    let mut resolver = NdlResolver::new("src/ndl/examples").expect("Failed to create resolver");

    println!("{}", resolver);

    resolver.run();

    println!("{}", resolver);
}
