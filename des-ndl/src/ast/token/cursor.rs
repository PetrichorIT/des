use crate::{
    lexer::{Token, TokenKind},
    Asset, Error, Span,
};

use super::Delimiter;

pub(super) struct Cursor<'a> {
    ts: &'a [Token],
    idx: usize,
    span_pos: usize,

    pub(super) asset: &'a Asset<'a>,
}

impl Cursor<'_> {
    pub(super) fn extract_subcursor(&mut self, delim: Delimiter) -> Result<Cursor<'_>, Error> {
        self.bump_back(1);

        let start = self.idx;
        let start_span = self.span_pos;

        let mut c = 0;
        while c > 0 && self.idx < self.ts.len() {
            let k = self.ts[self.idx].kind;
            if k == delim.open() {
                c += 1;
            }
            if k == delim.close() {
                c -= 1;
            }

            self.span_pos += self.ts[self.idx].len;
            self.idx += 1;
        }

        if c == 0 {
            Ok(Cursor {
                ts: &self.ts[start..self.idx],
                idx: 0,
                span_pos: start_span,
                asset: self.asset,
            })
        } else {
            unreachable!()
        }
    }

    pub(super) fn peek_span(&self) -> Span {
        Span::new(self.span_pos, self.ts[self.idx].len)
    }
}

impl<'a> Cursor<'a> {
    pub(super) fn new(ts: &'a [Token], span_pos: usize, asset: &'a Asset<'a>) -> Self {
        Self {
            ts,
            span_pos,
            idx: 0,
            asset,
        }
    }

    pub(super) fn is_done(&self) -> bool {
        self.idx == self.ts.len()
    }

    pub(super) fn next(&mut self) -> Option<(Token, Span)> {
        if self.idx >= self.ts.len() {
            None
        } else {
            let token = self.ts[self.idx];
            let span = Span::new(self.span_pos, token.len);
            self.bump(1);
            Some((token, span))
        }
    }

    pub(super) fn bump(&mut self, n: usize) {
        for _ in 0..n {
            self.span_pos += self.ts[self.idx].len;
            self.idx += 1;
        }
    }

    pub(super) fn bump_back(&mut self, n: usize) {
        for _ in 0..n {
            self.idx -= 1;
            self.span_pos -= self.ts[self.idx].len;
        }
    }

    pub(super) fn peek(&self, offset: usize) -> Option<Token> {
        let idx = self.idx + offset;
        if idx >= self.ts.len() {
            None
        } else {
            Some(self.ts[idx])
        }
    }

    pub(super) fn eat_while(&mut self, f: impl Fn(&Token) -> bool) -> usize {
        let mut c = 0;
        while self.idx < self.ts.len() && f(&self.ts[self.idx]) {
            self.bump(1);
            c += 1;
        }
        c
    }

    pub(super) fn eat_whitespace(&mut self) -> usize {
        self.eat_while(|t| t.kind == TokenKind::Whitespace)
    }
}
