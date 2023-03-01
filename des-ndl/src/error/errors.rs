use std::ops::Deref;

use super::*;

#[derive(Debug)]
pub struct Errors {
    pub(crate) list: LinkedList<Error>,
}

pub struct ErrorsMut {
    errors: Errors,
    mappings: Vec<Box<dyn Fn(Error) -> Error>>,
}

impl Errors {
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, n: usize) -> Option<&Error> {
        self.list.iter().nth(n)
    }

    pub fn new() -> Errors {
        Errors {
            list: LinkedList::new(),
        }
    }

    pub fn as_mut(self) -> ErrorsMut {
        ErrorsMut {
            errors: self,
            mappings: Vec::new(),
        }
    }
}

impl ErrorsMut {
    pub fn with_mapping(
        &mut self,
        mapping: impl Fn(Error) -> Error + 'static,
        f: impl FnOnce(&mut ErrorsMut),
    ) {
        self.mappings.push(Box::new(mapping));
        f(self);
        self.mappings.pop();
    }

    pub fn add(&mut self, mut error: Error) {
        for map in self.mappings.iter().rev() {
            error = map(error);
        }
        self.errors.list.push_back(error)
    }

    pub fn into_inner(self) -> Errors {
        self.errors
    }
}

impl Default for Errors {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ErrorsMut {
    type Target = Errors;
    fn deref(&self) -> &Self::Target {
        &self.errors
    }
}

impl Deref for Errors {
    type Target = LinkedList<Error>;
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}
