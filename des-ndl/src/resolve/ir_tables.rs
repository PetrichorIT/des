use std::sync::Arc;

use crate::{
    context::Context,
    error::*,
    ir::{Item, Link, Module},
    resource::AssetIdentifier,
};

#[derive(Debug)]
pub struct LinkIrTable {
    links: Vec<Arc<Link>>,
}

impl LinkIrTable {
    pub fn get(&self, key: impl AsRef<str>) -> Option<Arc<Link>> {
        let key = key.as_ref();
        self.links
            .iter()
            .find(|v| v.ident.raw == key)
            .map(|a| a.clone())
    }

    pub fn add(&mut self, local: Arc<Link>) {
        self.links.push(local)
    }

    pub fn from_ctx(
        ctx: &Context,
        asset: &AssetIdentifier,
        _errors: &mut ErrorsMut,
        include_self: bool,
    ) -> Self {
        let mut links = Vec::new();

        for (_, ir) in ctx.ir_for_asset(asset, include_self) {
            for item in ir.items.iter() {
                if let Item::Link(link) = item {
                    links.push(link.clone())
                }
            }
        }

        // no dup checking nessecary since done in ast stage

        Self { links }
    }
}

#[derive(Debug)]
pub struct ModuleIrTable {
    modules: Vec<Arc<Module>>,
}

impl ModuleIrTable {
    pub fn get(&self, key: impl AsRef<str>) -> Option<Arc<Module>> {
        let key = key.as_ref();
        self.modules
            .iter()
            .find(|v| v.ident.raw == key)
            .map(|a| a.clone())
    }

    pub fn add(&mut self, local: Arc<Module>) {
        self.modules.push(local)
    }

    pub fn from_ctx(
        ctx: &Context,
        asset: &AssetIdentifier,
        _errors: &mut ErrorsMut,
        include_self: bool,
    ) -> Self {
        let mut modules = Vec::new();

        for (_, ir) in ctx.ir_for_asset(asset, include_self) {
            for item in ir.items.iter() {
                if let Item::Module(module) = item {
                    modules.push(module.clone())
                }
            }
        }

        // no dup checking nessecary since done in ast stage

        Self { modules }
    }
}
