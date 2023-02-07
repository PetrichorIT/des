use crate::error::*;
use std::collections::LinkedList;

pub trait Validate {
    fn validate(&self, errors: &mut LinkedList<Error>);
}
