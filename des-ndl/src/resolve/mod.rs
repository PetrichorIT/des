use std::collections::LinkedList;
use std::sync::Arc;

use crate::context::Context;
use crate::ir;
use crate::resource::AssetIdentifier;
use crate::Error;

mod ast_tables;
mod ir_tables;
mod link;

pub use self::ast_tables::*;
pub use self::ir_tables::*;
pub use self::link::*;

impl Context {
    pub fn load_ir(&mut self, errors: &mut LinkedList<Error>) {
        let order = self.ir_load_order();
        for asset in order {
            let mut ast_links = LinkAstTable::from_ctx(self, &asset, errors);
            // let ast_modules = LinkAstTable::from_ctx(self, &asset, errors);

            let mut ir_links = LinkIrTable::from_ctx(self, &asset, errors);

            // Resolve links
            // - all nonlocal dependencies are allready ir
            // - local dependencies may be out of order
            ast_links.order_local_deps();

            let mut items = Vec::new();
            for link in ast_links.local() {
                let ir = ir::Link::from_ast(link.clone(), &ir_links, errors);
                let ir = Arc::new(ir);

                ir_links.add(ir.clone());
                items.push(ir::Item::Link(ir));
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
}
