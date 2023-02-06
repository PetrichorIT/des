use crate::{
    ast::parse::*,
    lexer::{Token, TokenKind},
    Asset, Span,
};

use super::Delimiter;

#[derive(Debug)]
pub(super) struct Cursor<'a> {
    pub(super) ts: &'a [Token],
    pub(super) idx: usize,
    pub(super) span_pos: usize,

    pub(super) asset: &'a Asset<'a>,
}

impl Cursor<'_> {
    pub(super) fn extract_subcursor(&mut self, delim: Delimiter) -> Result<Cursor<'_>> {
        let start = self.idx;
        let start_span = self.span_pos;

        let mut c = 1;
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
                ts: &self.ts[start..(self.idx - 1)],
                idx: 0,
                span_pos: start_span,
                asset: self.asset,
            })
        } else {
            return Err(Error::new(ErrorKind::MissingDelim, "missing delim"));
        }
    }

    pub(super) fn end_span(&self) -> Span {
        assert!(self.is_done());
        Span::new(self.span_pos, 1)
    }

    // pub(super) fn rem_stream_span(&self) -> Span {
    //     Span::new(self.span_pos, self.rem_stream_len())
    // }

    // pub(super) fn rem_stream_len(&self) -> usize {
    //     let mut len = 0;
    //     for token in &self.ts[self.idx..] {
    //         len += token.len;
    //     }
    //     len
    // }
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
