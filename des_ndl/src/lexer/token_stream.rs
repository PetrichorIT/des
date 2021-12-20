use std::cell::UnsafeCell;

use super::{tokenize, Token};
use crate::{error::LexingErrorContext, parser::ParResult, SourceAsset};

#[derive(Debug)]
pub struct TokenStream {
    inner: Vec<Token>,
    head: UnsafeCell<usize>,
}

impl TokenStream {
    pub fn new(asset: &SourceAsset, ectx: &mut LexingErrorContext) -> ParResult<Self> {
        let mut inner = Vec::new();
        for token in tokenize(&asset.data) {
            if token.kind.valid() && !token.kind.reducable() {
                inner.push(token)
            } else if !token.kind.valid() {
                ectx.record(&token)?;
            }
        }

        Ok(Self {
            inner,
            head: UnsafeCell::new(0),
        })
    }

    pub fn total_len(&self) -> usize {
        self.inner.len()
    }

    pub fn len(&self) -> usize {
        unsafe { self.inner.len() - *self.head.get() }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn peek(&self) -> ParResult<&Token> {
        unsafe {
            if *self.head.get() < self.inner.len() {
                Ok(&self.inner[*self.head.get()])
            } else {
                Err("Unexpected end of token stream while peeking")
            }
        }
    }

    pub fn bump(&self) -> ParResult<&Token> {
        unsafe {
            if *self.head.get() < self.inner.len() {
                *self.head.get() += 1;
                Ok(&self.inner[*self.head.get() - 1])
            } else {
                Err("Unexpected end of token stream while peeking")
            }
        }
    }

    pub fn bump_back(&self, steps: usize) {
        unsafe { *self.head.get() = (*self.head.get()).saturating_sub(steps) }
    }
}

impl FromIterator<Token> for TokenStream {
    fn from_iter<T: IntoIterator<Item = Token>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect(),
            head: UnsafeCell::new(0),
        }
    }
}
