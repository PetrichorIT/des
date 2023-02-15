use std::path::PathBuf;
use std::sync::Arc;

use crate::ast;
use crate::context::Context;
use crate::error::*;
use crate::ir;
use crate::resource::AssetIdentifier;

mod ast_tables;
mod ir_tables;
mod link;
mod local_tables;
mod module;
mod util;

pub use self::ast_tables::*;
pub use self::ir_tables::*;
pub use self::link::*;
pub use self::local_tables::*;
pub use self::module::*;

pub(crate) use self::util::*;

impl Context {
    pub(super) fn load_ir(&mut self, errors: &mut ErrorsMut) {
        let order = self.ir_load_order();
        for asset in order {
            let asset_str = asset.alias().to_string();
            let asset_path = asset.path().unwrap_or(&PathBuf::new()).clone();

            // Add asset info to all errors.
            errors.with_mapping(
                move |e| {
                    e.add_hints(ErrorHint::Note(format!(
                        "found in assset '{}' ({:?})",
                        asset_str, asset_path
                    )))
                },
                |errors| {
                    // Collect all defined items.
                    let mut items = Vec::new();

                    // Link deriving uses ast-link defs for local info, ir_links allready symbolised in
                    // dependencies and a global ast context for symbol resoloution.
                    let mut ast_links = LinkAstTable::from_ctx(self, &asset, errors);
                    let mut ir_links = LinkIrTable::from_ctx(self, &asset, errors, false);
                    let global_ast = GlobalAstTable::new(self, &asset);

                    // Resolve links
                    // - all nonlocal dependencies are allready ir
                    // - local dependencies may be out of order
                    ast_links.order_local_deps();
                    for link in ast_links.local() {
                        let ident = link.ident.raw.clone();
                        errors.with_mapping(
                            move |e| {
                                e.add_hints(ErrorHint::Note(format!(
                                    "found in link definition '{}'",
                                    ident
                                )))
                            },
                            |errors| {
                                // Use the link_specific symboliser to parse a link;
                                let ir = ir::Link::from_ast(
                                    link.clone(),
                                    &ir_links,
                                    &global_ast,
                                    errors,
                                );
                                let ir = Arc::new(ir);

                                // Add the link to the local ir_table to ensure that other links
                                // in this link can use it in later iterations.
                                // order is ensure by order_local_deps
                                ir_links.add(ir.clone());
                                items.push(ir::Item::Link(ir));
                            },
                        );
                    }

                    // Modules use local ast info, and dependency ir info, with global ast-debug info
                    let mut ast_modules = ModuleAstTable::from_ctx(self, &asset, errors);
                    let mut ir_modules = ModuleIrTable::from_ctx(self, &asset, errors, false);
                    let global_ast = GlobalAstTable::new(self, &asset);

                    // Resolve mdoules
                    // - same
                    ast_modules.order_local_deps();
                    for module in ast_modules.local() {
                        let ident = module.ident.raw.clone();
                        errors.with_mapping(
                            move |e| {
                                e.add_hints(ErrorHint::Note(format!(
                                    "found in module definition '{}'",
                                    ident
                                )))
                            },
                            |errors| {
                                // Use the local symboliser for modules.
                                let ir = ir::Module::from_ast(
                                    module.clone(),
                                    &ir_modules,
                                    &ir_links,
                                    &global_ast,
                                    errors,
                                );

                                let ir = Arc::new(ir);

                                // Add the module to the local ir_table to ensure that other modules
                                // in this module can use it in later iterations.
                                // order is ensure by order_local_deps
                                ir_modules.add(ir.clone());
                                items.push(ir::Item::Module(ir));
                            },
                        )
                    }

                    // Address collected items with asset identifier
                    self.ir.insert(asset, ir::Items { items });
                },
            );
        }
    }

    fn ir_load_order(&self) -> Vec<AssetIdentifier> {
        let mut order = Vec::new();
        let mut rem = self.assets.clone();
        while !rem.is_empty() {
            for i in 0..rem.len() {
                let deps = self.deps.get(&rem[i]).unwrap();
                let mut loadable = true;
                for dep in deps {
                    if !order.contains(dep) {
                        loadable = false;
                        break;
                    }
                }

                if loadable {
                    let asset = rem.remove(i);
                    order.push(asset);
                    break;
                }
            }
        }

        order
    }

    pub(super) fn load_entry(&mut self, errors: &mut ErrorsMut) {
        let ir_table = ModuleIrTable::from_ctx(self, &self.root, errors, true);

        let asts = self.asts_for_asset(&self.root);
        for ast in &asts {
            for item in &ast.1.items {
                let ast::Item::Entry(entry) = item else {
                    continue;
                };

                let Some(symbol) = ir_table.get(&entry.symbol.raw) else {
                    errors.add(Error::new(
                        ErrorKind::SymbolNotFound,
                        format!("defined entry symbol '{}' not in scope", entry.symbol.raw)
                    ));
                    return;
                };
                self.entry = Some(symbol);
                return;
            }
        }

        errors.add(Error::new(
            ErrorKind::MissingEntryPoint,
            "missing entry point to ndl topology",
        ));
    }
}
