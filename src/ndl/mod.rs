use std::{
    collections::HashMap,
    fmt::Display,
    path::{Component, Path, PathBuf},
};

use self::parser::Parser;

pub mod cursor;
pub mod error;

pub mod lexer;
pub mod parser;
pub mod validation;

#[derive(Debug, Clone)]
pub struct NdlResolver {
    pub(crate) state: NdlResolverState,

    pub(crate) root_dir: PathBuf,

    pub(crate) scopes: Vec<PathBuf>,
    pub(crate) units: HashMap<String, Parser>,
}

impl NdlResolver {
    pub fn new(raw_path: &str) -> Result<Self, &'static str> {
        let root_dir = Path::new(raw_path).to_owned();
        if !root_dir.exists() {
            return Err("Resolver must point to valid root.");
        }

        Ok(Self {
            state: NdlResolverState::Idle,

            root_dir,
            scopes: Vec::new(),
            units: HashMap::new(),
        })
    }

    pub fn parse(&mut self) {
        self.scopes = self.get_ndl_scopes();

        let root_component_len = self.root_dir.components().count();
        for scope in &self.scopes {
            let components = scope.components().collect::<Vec<Component>>();
            let naming_subset = &components[root_component_len..]
                .iter()
                .filter_map(|c| match c {
                    Component::Normal(str) => Some(str.to_str()?),
                    _ => None,
                })
                .collect::<Vec<&str>>();
            let mut name = naming_subset.join("/");
            name.truncate(name.len() - 4);

            let mut parser = Parser::new(scope);
            parser.parse();

            self.units.insert(name, parser);
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
            "scopes: {} units: {}",
            self.scopes.len(),
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
    let mut resolver = NdlResolver::new("src/ndl").expect("Failed to create resolver");

    println!("{}", resolver);

    resolver.parse();

    println!("{}", resolver);
}
