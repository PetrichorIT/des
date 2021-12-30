use crate::*;

use crate::error::*;

use std::cell::UnsafeCell;

///
/// A stream of NDL tokens referncing an [Asset].
///
#[derive(Debug)]
pub struct TokenStream {
    inner: Vec<Token>,
    head: UnsafeCell<usize>,
}

impl TokenStream {
    pub fn new(asset: Asset<'_>, ectx: &mut LexingErrorContext) -> NdlResult<Self> {
        let mut inner = Vec::new();
        for token in tokenize(asset.source(), asset.start_pos()) {
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

    pub fn peek(&self) -> NdlResult<&Token> {
        unsafe {
            if *self.head.get() < self.inner.len() {
                Ok(&self.inner[*self.head.get()])
            } else {
                Err("Unexpected end of token stream while peeking")
            }
        }
    }

    pub fn prev_non_whitespace(&self, skip: usize) -> NdlResult<&Token> {
        let head = unsafe { *self.head.get() };
        let mut i = head - skip;
        while i > 0 {
            i -= 1;
            if self.inner[i].kind != TokenKind::Whitespace {
                return Ok(&self.inner[i]);
            }
        }

        Err("Could not find valid prev token")
    }

    pub fn bump(&self) -> NdlResult<&Token> {
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

impl Clone for TokenStream {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            head: UnsafeCell::new(unsafe { *self.head.get() }),
        }
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
