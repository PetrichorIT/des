use crate::error::*;

mod blanket;
mod items;
mod link;
mod module;

pub trait Validate {
    fn validate(&self, errors: &mut ErrorsMut);
}
