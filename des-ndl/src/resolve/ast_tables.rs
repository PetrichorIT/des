use std::{collections::LinkedList, sync::Arc};

use crate::{
    ast::{self, Item, LinkStmt, ModuleStmt, Spanned},
    error::*,
    resource::AssetIdentifier,
    Context, SourceMap, Span,
};

// # Links

#[derive(Debug, Clone, PartialEq)]
pub struct LinkAstTable {
    source: AssetIdentifier,
    links: Vec<Arc<LinkStmt>>,
    ptr: usize,
}

impl LinkAstTable {
    pub fn order_local_deps(&mut self) {
        let local = self.local_mut();
        let mut s = 0;

        while s < local.len() {
            // let mut loadable = false;
            let mut i = s;

            'seacher: while i < local.len() {
                // check if i depents are in ..s
                if let Some(ref inh) = local[i].inheritance {
                    'inner: for dep in inh.symbols.iter() {
                        let valid = local[..s].iter().any(|l| l.ident.raw == dep.raw);

                        if !valid {
                            let valid_nonlocal = local[s..].iter().any(|l| l.ident.raw == dep.raw);
                            if valid_nonlocal {
                                continue 'inner;
                            }

                            i += 1;
                            continue 'seacher;
                        }
                    }
                    // all deps are valid
                    // loadable = true;
                    break;
                } else {
                    // loadable = true;
                    break;
                }
            }

            // not all deps may be loadable, since nonlocal deps are not repr
            if s != i && i < local.len() {
                local.swap(s, i);
            }
            s += 1;
        }
    }

    pub fn local(&self) -> &[Arc<LinkStmt>] {
        &self.links[..self.ptr]
    }

    pub fn local_mut(&mut self) -> &mut [Arc<LinkStmt>] {
        &mut self.links[..self.ptr]
    }

    pub fn from_ctx(
        ctx: &Context,
        asset: &AssetIdentifier,
        errors: &mut LinkedList<Error>,
    ) -> Self {
        let mut links = Vec::new();

        let asts = ctx.asts_for_asset(&asset);
        let ptr = asts[0]
            .1
            .items
            .iter()
            .filter(|i| matches!(i, Item::Link(_)))
            .count();

        for (_, ast) in asts {
            for item in &ast.items {
                if let Item::Link(link) = item {
                    links.push(link.clone())
                }
            }
        }

        Self::check_dup(&links, errors);

        Self {
            source: asset.clone(),
            links,
            ptr,
        }
    }

    fn check_dup(links: &[Arc<LinkStmt>], errors: &mut LinkedList<Error>) {
        if links.len() <= 1 {
            return;
        }
        for s in 0..(links.len() - 1) {
            let mut dups = Vec::new();
            for i in (s + 1)..links.len() {
                if links[s].ident.raw == links[i].ident.raw {
                    dups.push(i)
                }
            }

            if !dups.is_empty() {
                let mut e = Error::new(
                    ErrorKind::SymbolDuplication,
                    format!(
                        "Found duplicated symbol '{}', {} duplications",
                        links[s].ident.raw,
                        dups.len()
                    ),
                )
                .spanned(links[s].span());
                for i in dups {
                    e = e.add_hints(ErrorHint::Note(format!(
                        "duplicated symbol definition found at {:?}",
                        links[i].span()
                    )));
                }

                errors.push_back(e)
            }
        }
    }
}

// # Modules

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleAstTable {
    source: AssetIdentifier,
    modules: Vec<Arc<ModuleStmt>>,
    ptr: usize,
}

impl ModuleAstTable {
    pub fn order_local_deps(&mut self) {
        let local = self.local_mut();
        let mut s = 0;

        while s < local.len() {
            // let mut loadable = false;
            let mut i = s;

            'seacher: while i < local.len() {
                // check if i depents are in ..s
                if let Some(ref sbm) = local[i].submodules {
                    'inner: for dep in sbm.items.iter() {
                        let dep = &dep.typ;
                        let valid = local[..s].iter().any(|l| l.ident.raw == dep.raw);

                        if !valid {
                            let valid_nonlocal = local[s..].iter().any(|l| l.ident.raw == dep.raw);
                            if valid_nonlocal {
                                continue 'inner;
                            }

                            i += 1;
                            continue 'seacher;
                        }
                    }
                    // all deps are valid
                    // loadable = true;
                    break;
                } else {
                    // loadable = true;
                    break;
                }
            }

            // not all deps may be loadable, since nonlocal deps are not repr
            if s != i && i < local.len() {
                local.swap(s, i);
            }
            s += 1;
        }
    }

    pub fn local(&self) -> &[Arc<ModuleStmt>] {
        &self.modules[..self.ptr]
    }

    pub fn local_mut(&mut self) -> &mut [Arc<ModuleStmt>] {
        &mut self.modules[..self.ptr]
    }

    pub fn from_ctx(
        ctx: &Context,
        asset: &AssetIdentifier,
        errors: &mut LinkedList<Error>,
    ) -> Self {
        let mut modules = Vec::new();

        let asts = ctx.asts_for_asset(&asset);
        let ptr = asts[0]
            .1
            .items
            .iter()
            .filter(|i| matches!(i, Item::Module(_)))
            .count();

        // println!("{asts:#?}");

        for (_, ast) in asts {
            for item in &ast.items {
                if let Item::Module(module) = item {
                    modules.push(module.clone())
                }
            }
        }

        Self::check_dup(&modules, errors);

        Self {
            source: asset.clone(),
            modules,
            ptr,
        }
    }

    fn check_dup(modules: &[Arc<ModuleStmt>], errors: &mut LinkedList<Error>) {
        if modules.len() <= 1 {
            return;
        }
        for s in 0..(modules.len() - 1) {
            let mut dups = Vec::new();
            for i in (s + 1)..modules.len() {
                if modules[s].ident.raw == modules[i].ident.raw {
                    dups.push(i)
                }
            }

            if !dups.is_empty() {
                let mut e = Error::new(
                    ErrorKind::SymbolDuplication,
                    format!(
                        "Found duplicated symbol '{}', {} duplications",
                        modules[s].ident.raw,
                        dups.len()
                    ),
                )
                .spanned(modules[s].span());
                for i in dups {
                    e = e.add_hints(ErrorHint::Note(format!(
                        "duplicated symbol definition found at {:?}",
                        modules[i].span()
                    )));
                }

                errors.push_back(e)
            }
        }
    }
}

pub struct GlobalAstTable<'a> {
    this: AssetIdentifier,
    smap: &'a SourceMap,
    modules: Vec<Arc<ModuleStmt>>,
    links: Vec<Arc<LinkStmt>>,
}

impl<'a> GlobalAstTable<'a> {
    pub fn new(ctx: &'a Context, this: &AssetIdentifier) -> GlobalAstTable<'a> {
        let mut modules = Vec::new();
        let mut links = Vec::new();

        for file in ctx.ast.values() {
            for item in &file.items {
                match item {
                    ast::Item::Module(module) => modules.push(module.clone()),
                    ast::Item::Link(link) => links.push(link.clone()),
                    _ => {}
                }
            }
        }

        GlobalAstTable {
            this: this.clone(),
            smap: &ctx.smap,
            modules,
            links,
        }
    }

    pub fn err_resolve_symbol(&self, symbol: &str, expect_module: bool, mut error: Error) -> Error {
        for module in &self.modules {
            if module.ident.raw == symbol {
                let target_asset = self.smap.asset_for(module.span()).unwrap();
                let target = target_asset.ident.path().unwrap().to_str().unwrap();

                if expect_module {
                    let this = self.smap.asset(&self.this).unwrap();
                    let span = Span::new(this.offset, 0);
                    let replacement = format!("include {};", this.include_for(target_asset));

                    error.hints.push(ErrorHint::Help(format!(
                        "similar symbol '{symbol}' was found, but not included ({target})"
                    )));
                    error.hints.push(ErrorHint::Solution(ErrorSolution {
                        description: format!("try including '{symbol}'"),
                        span,
                        replacement,
                    }))
                } else {
                    error.hints.push(ErrorHint::Note(format!(
                        "similar symbol '{symbol}' was found, but it is a module ({target})"
                    )));
                }
            }
        }

        for link in &self.links {
            if link.ident.raw == symbol {
                let target_asset = self.smap.asset_for(link.span()).unwrap();
                let target = target_asset.ident.path().unwrap().to_str().unwrap();

                if !expect_module {
                    let this = self.smap.asset(&self.this).unwrap();
                    let span = Span::new(this.offset, 0);
                    let replacement = format!("include {};", this.include_for(target_asset));

                    error.hints.push(ErrorHint::Help(format!(
                        "similar symbol '{symbol}' was found, but not included"
                    )));
                    error.hints.push(ErrorHint::Solution(ErrorSolution {
                        description: format!("try including '{symbol}'"),
                        span,
                        replacement,
                    }))
                } else {
                    error.hints.push(ErrorHint::Note(format!(
                        "similar symbol '{symbol}' was found, but it is a link ({target})"
                    )));
                }
            }
        }

        error
    }
}
