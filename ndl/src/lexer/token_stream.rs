use crate::*;
use crate::error::*;
use std::cell::RefCell;

///
/// A stream of NDL tokens referncing an [Asset].
///
#[derive(Debug)]
pub struct TokenStream {
    inner: Vec<Token>,
    head: RefCell<usize>,
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
            head: RefCell::new(0),
        })
    }

    pub fn head(&self) -> usize {
        *self.head.borrow()
    }

    pub fn total_len(&self) -> usize {
        self.inner.len()
    }

    // TODO: rematch this function
    pub fn len(&self) -> usize {
        self.inner.len() - self.head()
    }

    pub fn remaining(&self) -> usize {
        self.inner.len() - self.head()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn peek(&self) -> NdlResult<&Token> {
        if self.head() < self.inner.len() {
            Ok(&self.inner[self.head()])
        } else {
            Err("Unexpected end of token stream while peeking.")
        }
    }

    pub fn prev_non_whitespace(&self, skip: usize) -> NdlResult<&Token> {
        let head = self.head();
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
        let mut head = self.head.borrow_mut();
        if *head < self.inner.len() {
            *head += 1;
            Ok(&self.inner[*head - 1])
        } else {
            Err("Unexpected end of token stream while peeking")
        }
    }

    pub fn bump_back(&self, steps: usize) {
        let mut head = self.head.borrow_mut();
        *head = head.saturating_sub(steps);
    }

    pub fn bump_back_while<P>(&self, mut p: P)
    where
        P: FnMut(&Token) -> bool,
    {
        while p(&self.inner[self.head()]) {
            self.bump_back(1)
        }
    }
}

impl Clone for TokenStream {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            head: RefCell::new(self.head()),
        }
    }
}

impl FromIterator<Token> for TokenStream {
    fn from_iter<T: IntoIterator<Item = Token>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect(),
            head: RefCell::new(0),
        }
    }
}
