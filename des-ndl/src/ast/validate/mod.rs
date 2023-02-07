use crate::error::*;
use std::collections::LinkedList;

mod blanket;
mod items;
mod link;
mod module;

pub trait Validate {
    fn validate(&self, errors: &mut LinkedList<Error>);
}
