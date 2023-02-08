use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    ast, error::*, resource::AssetIdentifier, Parse, ParseBuffer, SourceMap, Spanned, TokenStream,
};

pub struct Context {
    pub smap: SourceMap,
    pub root: AssetIdentifier,
    pub assets: Vec<AssetIdentifier>,
    pub ast: HashMap<AssetIdentifier, ast::File>,
}

impl Context {
    pub fn load(path: impl AsRef<Path>) -> Result<Context> {
        let mut smap = SourceMap::new();

        let ident = AssetIdentifier::Root {
            path: path.as_ref().to_owned(),
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
            ast: HashMap::from([(ident, file)]),
        };
        this.load_includes()?;

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
                            assert!(anchor.ends_with(".ndl"));
                            let mut anchor = anchor.parent().unwrap().to_path_buf();
                            for comp in ipath.split('/') {
                                anchor.push(comp)
                            }
                            anchor = anchor.canonicalize().unwrap();

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
            for (mut task, span) in tasks {
                if !task.exists() {
                    if !task.ends_with(".ndl") {
                        task.set_extension(".ndl");
                    }
                }

                let alias = self.root.relative_asset_alias(&task);

                let ident = AssetIdentifier::Included {
                    path: task,
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
}
