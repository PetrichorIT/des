use std::sync::Arc;

mod link;
mod module;
mod refs;
mod symbol;

pub use self::link::*;
pub use self::module::*;
pub use self::refs::*;
pub use self::symbol::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Items {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Link(Arc<Link>),
    Module(Arc<Module>),
}

impl Items {
    pub fn link(&self, ident: impl AsRef<str>) -> Option<Arc<Link>> {
        let ident = ident.as_ref();
        self.items.iter().find_map(|v| {
            if let Item::Link(l) = v {
                if l.ident.raw == ident {
                    Some(l.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn module(&self, ident: impl AsRef<str>) -> Option<Arc<Module>> {
        let ident = ident.as_ref();
        self.items.iter().find_map(|v| {
            if let Item::Module(l) = v {
                if l.ident.raw == ident {
                    Some(l.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}
