use std::{cell::Cell, sync::Arc};

use crate::ast::token::{TokenStream, TokenTree};

pub struct Cursor {
    ts: Arc<TokenStream>,
    idx: Cell<usize>,
}

impl Cursor {
    pub(crate) fn root(ts: Arc<TokenStream>) -> Self {
        Self {
            ts,
            idx: Cell::new(0),
        }
    }

    pub(crate) fn peek(&self) -> Option<&TokenTree> {
        if self.idx.get() >= self.ts.items.len() {
            None
        } else {
            Some(&self.ts.items[self.idx.get()])
        }
    }

    pub(crate) fn next(&mut self) -> Option<&TokenTree> {
        let ret = self.peek()?;
        self.bump();
        Some(ret)
    }

    pub(crate) fn bump(&self) {
        self.idx.set(self.idx.get() + 1)
    }
}
