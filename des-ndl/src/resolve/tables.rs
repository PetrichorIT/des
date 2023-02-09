use std::{collections::LinkedList, sync::Arc};

use crate::{
    ast::{self, LinkStmt},
    resource::AssetIdentifier,
    Context, Error, ErrorHint, ErrorKind, ModuleStmt, Spanned,
};

// # Links

#[derive(Debug, Clone, PartialEq)]
pub struct LinkSymbolTable {
    source: AssetIdentifier,
    links: Vec<Arc<LinkStmt>>,
}

impl LinkSymbolTable {
    pub fn from_ctx(ctx: &Context, asset: AssetIdentifier, errors: &mut LinkedList<Error>) -> Self {
        let mut links = Vec::new();

        for (_, ast) in ctx.asts_for_asset(&asset) {
            for item in &ast.items {
                if let ast::Item::Link(link) = item {
                    links.push(link.clone())
                }
            }
        }

        Self::check_dup(&links, errors);

        Self {
            source: asset,
            links,
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
pub struct ModuleSymbolTable {
    source: AssetIdentifier,
    modules: Vec<Arc<ModuleStmt>>,
}

impl ModuleSymbolTable {
    pub fn from_ctx(ctx: &Context, asset: AssetIdentifier, errors: &mut LinkedList<Error>) -> Self {
        let mut modules = Vec::new();

        for (_, ast) in ctx.asts_for_asset(&asset) {
            for item in &ast.items {
                if let ast::Item::Module(module) = item {
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
