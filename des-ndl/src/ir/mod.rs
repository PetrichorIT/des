mod link;
mod symbol;

use std::sync::Arc;

pub use self::link::*;
pub use self::symbol::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Items {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Link(Arc<Link>),
    Module(),
}
