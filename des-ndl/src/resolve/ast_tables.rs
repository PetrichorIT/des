use std::{collections::LinkedList, sync::Arc};

use crate::{
    ast::{Item, LinkStmt},
    resource::AssetIdentifier,
    Context, Error, ErrorHint, ErrorKind, ModuleStmt, Spanned,
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
}

impl ModuleAstTable {
    pub fn from_ctx(ctx: &Context, asset: AssetIdentifier, errors: &mut LinkedList<Error>) -> Self {
        let mut modules = Vec::new();

        for (_, ast) in ctx.asts_for_asset(&asset) {
            for item in &ast.items {
                if let Item::Module(module) = item {
                    modules.push(module.clone())
                }
            }
        }

        Self::check_dup(&modules, errors);

        Self {
            source: asset,
            modules,
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
