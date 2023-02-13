use std::collections::LinkedList;
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

pub use self::ast_tables::*;
pub use self::ir_tables::*;
pub use self::link::*;
pub use self::local_tables::*;
pub use self::module::*;

impl Context {
    pub(super) fn load_ir(&mut self, errors: &mut LinkedList<Error>) {
        let order = self.ir_load_order();
        for asset in order {
            let mut items = Vec::new();

            let mut ast_links = LinkAstTable::from_ctx(self, &asset, errors);
            let mut ir_links = LinkIrTable::from_ctx(self, &asset, errors, false);

            // Resolve links
            // - all nonlocal dependencies are allready ir
            // - local dependencies may be out of order
            ast_links.order_local_deps();
            for link in ast_links.local() {
                let ir = ir::Link::from_ast(link.clone(), &ir_links, errors);
                let ir = Arc::new(ir);

                ir_links.add(ir.clone());
                items.push(ir::Item::Link(ir));
            }

            let mut ast_modules = ModuleAstTable::from_ctx(self, &asset, errors);
            let mut ir_modules = ModuleIrTable::from_ctx(self, &asset, errors, false);

            // Resolve mdoules
            // - same
            ast_modules.order_local_deps();
            for module in ast_modules.local() {
                let ir = ir::Module::from_ast(module.clone(), &ir_modules, &ir_links, errors);
                let ir = Arc::new(ir);

                ir_modules.add(ir.clone());
                items.push(ir::Item::Module(ir));
            }

            self.ir.insert(asset, ir::Items { items });
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

    pub(super) fn load_entry(&mut self, errors: &mut LinkedList<Error>) {
        let ir_table = ModuleIrTable::from_ctx(self, &self.root, errors, true);

        let asts = self.asts_for_asset(&self.root);
        for ast in &asts {
            for item in &ast.1.items {
                let ast::Item::Entry(entry) = item else {
                    continue;
                };

                let Some(symbol) = ir_table.get(&entry.symbol.raw) else {
                    errors.push_back(Error::new(
                        ErrorKind::SymbolNotFound,
                        format!("defined entry symbol '{}' not in scope", entry.symbol.raw)
                    ));
                    return;
                };
                self.entry = Some(symbol);
                return;
            }
        }

        errors.push_back(Error::new(
            ErrorKind::MissingEntryPoint,
            "missing entry point to ndl topology",
        ));
    }
}
