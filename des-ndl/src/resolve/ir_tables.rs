use std::{collections::LinkedList, sync::Arc};

use crate::{
    context::Context,
    ir::{Item, Link},
    resource::AssetIdentifier,
    Error,
};

pub struct LinkIrTable {
    source: AssetIdentifier,
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
        _errors: &mut LinkedList<Error>,
    ) -> Self {
        let mut links = Vec::new();

        for (_, ir) in ctx.ir_for_asset(asset) {
            for item in ir.items.iter() {
                if let Item::Link(link) = item {
                    links.push(link.clone())
                }
            }
        }

        // no dup checking nessecary since done in ast stage

        Self {
            source: asset.clone(),
            links,
        }
    }
}
