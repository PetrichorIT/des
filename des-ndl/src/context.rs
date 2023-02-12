use std::{
    collections::{HashMap, LinkedList},
    path::Path,
    sync::Arc,
};

use crate::{
    ast::{self, validate::Validate, Parse, ParseBuffer, Spanned, TokenStream},
    error::*,
    ir,
    resource::{fs::canon, AssetIdentifier},
    SourceMap,
};

#[derive(Debug)]
pub struct Context {
    pub smap: SourceMap,

    pub root: AssetIdentifier,
    pub assets: Vec<AssetIdentifier>,
    pub deps: HashMap<AssetIdentifier, Vec<AssetIdentifier>>,

    pub ast: HashMap<AssetIdentifier, ast::File>,
    pub ir: HashMap<AssetIdentifier, ir::Items>,
    pub entry: Option<Arc<ir::Module>>,
}

impl Context {
    pub fn load(path: impl AsRef<Path>) -> Result<Context> {
        let path = path.as_ref().to_path_buf();

        let mut smap = SourceMap::new();
        let ident = AssetIdentifier::Root {
            path,
            alias: "root".to_string(),
        };

        let asset = smap.load_file(ident.clone()).map_err(Error::from_io)?;
        let ts = TokenStream::new(asset)?;
        let buf = ParseBuffer::new(asset, ts);

        let file = ast::File::parse(&buf)?;
        let mut this = Context {
            smap,
            root: ident.clone(),
            assets: vec![ident.clone()],
            deps: HashMap::new(),

            ast: HashMap::from([(ident, file)]),
            ir: HashMap::new(),
            entry: None,
        };
        this.load_includes()?;

        let mut errors = LinkedList::new();

        this.ast_validate_assets(&mut errors);
        if !errors.is_empty() {
            return Err(Error::root(errors));
        }

        this.load_deps(&mut errors);
        if !errors.is_empty() {
            return Err(Error::root(errors));
        }

        this.load_ir(&mut errors);
        if !errors.is_empty() {
            return Err(Error::root(errors));
        }

        this.load_entry(&mut errors);
        if !errors.is_empty() {
            return Err(Error::root(errors));
        }

        Ok(this)
    }

    fn load_includes(&mut self) -> Result<()> {
        let mut i = 0;
        while i < self.assets.len() {
            let asset = self.assets[i].clone();
            let items = self
                .ast
                .get(&asset)
                .expect("Asset was registered, but not ast provided");

            let mut tasks = Vec::new();
            for item in &items.items {
                if let ast::Item::Include(include) = item {
                    let ipath = include.path.path();
                    match asset.path() {
                        Ok(anchor) => {
                            assert!(anchor
                                .extension()
                                .map(|e| e.to_string_lossy().contains("ndl"))
                                .unwrap_or(false));

                            let mut anchor = anchor.parent().unwrap().to_path_buf();
                            for comp in ipath.split('/') {
                                anchor.push(comp)
                            }
                            anchor = canon(anchor);
                            anchor.set_extension("ndl");
                            // anchor = anchor.canonicalize().unwrap();

                            if !self
                                .assets
                                .iter()
                                .any(|asset| asset.path().unwrap() == &anchor)
                            {
                                tasks.push((anchor, include.span()))
                            };
                        }
                        Err(_) => todo!(),
                    }
                }
            }

            // drop scope for items
            for (path, span) in tasks {
                let alias = self.root.relative_asset_alias(&path);

                let ident = AssetIdentifier::Included {
                    path,
                    alias,
                    include: span,
                };
                let asset = self.smap.load_file(ident.clone()).map_err(Error::from_io)?;
                let ts = TokenStream::new(asset)?;
                let buf = ParseBuffer::new(asset, ts);

                let file = ast::File::parse(&buf)?;
                self.assets.push(ident.clone());
                self.ast.insert(ident, file);
            }

            i += 1;
        }
        Ok(())
    }

    fn ast_validate_assets(&mut self, errors: &mut LinkedList<Error>) {
        for (_asset, ast) in &self.ast {
            ast.validate(errors)
        }
    }

    pub(crate) fn asts_for_asset(
        &self,
        asset: &AssetIdentifier,
    ) -> Vec<(&AssetIdentifier, &ast::File)> {
        let iter = self
            .deps
            .get(&asset)
            .unwrap()
            .into_iter()
            .map(|k| (k, self.ast.get(k).unwrap()));

        let asset = self.assets.iter().find(|a| *a == asset).unwrap(); // for lifetimes
        let init = std::iter::once((asset, self.ast.get(asset).unwrap()));

        Vec::from_iter(init.chain(iter))
    }

    pub(crate) fn ir_for_asset(
        &self,
        asset: &AssetIdentifier,
        include_self: bool,
    ) -> Vec<(&AssetIdentifier, &ir::Items)> {
        let iter = self
            .deps
            .get(&asset)
            .unwrap()
            .into_iter()
            .map(|k| (k, self.ir.get(k).unwrap()));

        if include_self {
            let asset = self.assets.iter().find(|a| *a == asset).unwrap(); // for lifetimes
            let init = std::iter::once((asset, self.ir.get(asset).unwrap()));

            Vec::from_iter(init.chain(iter))
        } else {
            iter.collect()
        }
    }

    fn load_deps(&mut self, errors: &mut LinkedList<Error>) {
        if !self.deps.is_empty() {
            return;
        }

        let mut topo: Vec<Vec<usize>> = vec![Vec::new(); self.assets.len()];

        // Add raw edges
        for i in 0..self.assets.len() {
            let ast = self.ast.get(&self.assets[i]).unwrap();
            for item in ast.items.iter() {
                if let ast::Item::Include(include) = item {
                    let ipath = include.path.path();
                    let mut anchor = self.assets[i]
                        .path()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_path_buf();
                    for comp in ipath.split('/') {
                        anchor.push(comp)
                    }
                    anchor = canon(anchor);
                    anchor.set_extension("ndl");

                    let Some(asset) = self.assets.iter().enumerate().find(|asset| *asset.1.path().unwrap() == anchor) else {
                        unreachable!()
                    };

                    topo[i].push(asset.0);
                }
            }
        }

        // Recusivly add entries (DFS)
        for i in 0..self.assets.len() {
            if let Err(e) = self.load_recursive_deps_for(i, &topo) {
                errors.push_back(e)
            }
        }
    }

    fn load_recursive_deps_for(&mut self, i: usize, topo: &[Vec<usize>]) -> Result<()> {
        let mut dep = Vec::new();
        let mut visited = vec![false; self.assets.len()];

        fn dfs(
            topo: &[Vec<usize>],
            o: usize,
            i: usize,
            visited: &mut [bool],
            deps: &mut Vec<AssetIdentifier>,
            labels: &[AssetIdentifier],
        ) -> Result<()> {
            if visited[i] {
                if i == o {
                    Err(Error::new(ErrorKind::CyclicDeps, "cyclic deps"))
                } else {
                    Ok(())
                }
            } else {
                visited[i] = true;
                if o != i {
                    deps.push(labels[i].clone());
                }
                for edge in &topo[i] {
                    dfs(topo, o, *edge, visited, deps, labels)?;
                }
                Ok(())
            }
        }

        dfs(topo, i, i, &mut visited, &mut dep, &self.assets)?;
        self.deps.insert(self.assets[i].clone(), dep);
        Ok(())
    }
}
