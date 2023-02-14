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
