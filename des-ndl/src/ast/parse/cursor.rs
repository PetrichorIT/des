use std::{cell::Cell, sync::Arc};

use crate::ast::token::{TokenStream, TokenTree};

#[derive(Debug)]
pub struct Cursor {
    ts: Arc<Vec<TokenTree>>,
    idx: Cell<usize>,
}

impl Cursor {
    pub(crate) fn new(ts: &TokenStream) -> Self {
        Self {
            ts: ts.items.clone(),
            idx: Cell::new(0),
        }
    }

    pub(crate) fn state(&self) -> usize {
        self.idx.get()
    }

    pub(crate) fn set_state(&self, state: usize) {
        self.idx.set(state);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.idx.get() >= self.ts.len()
    }

    pub(crate) fn peek(&self) -> Option<&TokenTree> {
        if self.idx.get() >= self.ts.len() {
            None
        } else {
            Some(&self.ts[self.idx.get()])
        }
    }

    pub(crate) fn bump(&self) {
        self.idx.set(self.idx.get() + 1)
    }

    pub(crate) fn subcursor(&self) -> Option<Cursor> {
        let cur = &self.ts.get(self.idx.get())?;
        let TokenTree::Delimited(_, _, sub) = cur else {
            return None;
        };
        Some(Cursor::new(sub))
    }
}
