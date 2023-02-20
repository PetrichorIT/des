use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    ast::{self, validate::Validate, Parse, ParseBuffer, Spanned, TokenStream},
    error::*,
    ir,
    resource::{fs::canon, AssetIdentifier},
    util::dfs_cycles,
    SourceMap, Span,
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
    pub fn load(path: impl AsRef<Path>) -> RootResult<Context> {
        let path = path.as_ref().to_path_buf();

        let mut this = Self::load_initial_tree(path)?;
        let mut errors = Errors::new().as_mut();

        this.ast_validate_assets(&mut errors);
        if !errors.is_empty() {
            return Err(RootError::new(errors.into_inner(), this.smap));
        }

        this.load_deps(&mut errors);
        if !errors.is_empty() {
            return Err(RootError::new(errors.into_inner(), this.smap));
        }

        this.load_ir(&mut errors);
        if !errors.is_empty() {
            return Err(RootError::new(errors.into_inner(), this.smap));
        }

        this.load_entry_and_check_dyn(&mut errors);
        if !errors.is_empty() {
            return Err(RootError::new(errors.into_inner(), this.smap));
        }

        Ok(this)
    }

    fn load_initial_tree(path: PathBuf) -> RootResult<Context> {
        let mut smap = SourceMap::new();
        let ident = AssetIdentifier::Root {
            path,
            alias: "root".to_string(),
        };

        let asset = match smap.load_file(ident.clone()).map_err(Error::from_io) {
            Ok(asset) => asset,
            Err(e) => return Err(RootError::single(e, smap)),
        };
        let ts = match TokenStream::new(asset) {
            Ok(ts) => ts,
            Err(e) => return Err(RootError::single(e, smap)),
        };
        let buf = ParseBuffer::new(asset, ts);
        let file = match ast::File::parse(&buf) {
            Ok(file) => file,
            Err(e) => return Err(RootError::single(e, smap)),
        };

        let mut this = Context {
            smap,
            root: ident.clone(),
            assets: vec![ident.clone()],
            deps: HashMap::new(),

            ast: HashMap::from([(ident, file)]),
            ir: HashMap::new(),
            entry: None,
        };
        if let Err(e) = this.load_includes() {
            return Err(RootError::single(e, this.smap));
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

    fn ast_validate_assets(&mut self, errors: &mut ErrorsMut) {
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

    fn load_deps(&mut self, errors: &mut ErrorsMut) {
        if !self.deps.is_empty() {
            return;
        }

        let mut topo: Vec<Vec<usize>> = vec![Vec::new(); self.assets.len()];
        let mut topo_span: Vec<Vec<Span>> = vec![Vec::new(); self.assets.len()];

        // Build topology from raw edges.
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
                    topo_span[i].push(include.span())
                }
            }
        }

        // Check for cycles - throw error if found
        match dfs_cycles(&topo) {
            Ok(reachability) => {
                // Add fully expanded dep trees
                for i in 0..self.assets.len() {
                    let mut deps = Vec::with_capacity(self.assets.len());
                    for (j, &reachable) in reachability[i].iter().enumerate() {
                        if reachable && i != j {
                            deps.push(self.assets[j].clone());
                        }
                    }

                    self.deps.insert(self.assets[i].clone(), deps);
                }
            }
            Err(cycles) => {
                // Append each elementary cycles as its own error
                for cycle in cycles {
                    let s = cycle[0];

                    let mut fmt = vec![self.assets[s].alias()];
                    for &e in cycle.iter().rev() {
                        fmt.push(self.assets[e].alias());
                    }

                    // find inital edge for span
                    let mut span = Span::new(0, 0);
                    for j in 0..topo[s].len() {
                        if topo[s][j] == cycle[1] {
                            span = topo_span[s][j]
                        }
                    }

                    errors.add(
                        Error::new(
                            ErrorKind::CyclicDeps,
                            format!("found cyclic includes: {}", fmt.join(" <- ")),
                        )
                        .spanned(span),
                    )
                }
            }
        }
    }
}
