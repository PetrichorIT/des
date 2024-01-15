use super::{Cursor, Delimiter, Token};
use crate::{
    ast::{token::TokenKind, Lit},
    error::{Error, ErrorKind},
    lexer::{self, LiteralKind},
    Span,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TokenStream {
    pub items: Arc<Vec<TokenTree>>,
}

#[derive(Debug, Clone)]
pub enum TokenTree {
    Token(Token, Spacing),
    Delimited(DelimSpan, Delimiter, TokenStream),
}

impl TokenTree {
    pub fn span(&self) -> Span {
        match self {
            Self::Token(token, _) => token.span,
            Self::Delimited(delim, _, _) => Span::fromto(delim.open, delim.close),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spacing {
    Alone,
    Joint,
}

impl Spacing {
    fn infer(token: lexer::TokenKind, next: lexer::TokenKind) -> Spacing {
        use lexer::TokenKind::*;
        match (token, next) {
            (Eq, Eq) => Spacing::Joint,
            (Plus, Eq) | (Minus, Eq) | (Star, Eq) | (Slash, Eq) => Spacing::Joint,
            _ => Spacing::Alone,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelimSpan {
    pub open: Span,
    pub close: Span,
}

impl TokenStream {
    pub(super) fn parse(cursor: &mut Cursor) -> Result<TokenStream, Error> {
        let mut items = Vec::new();
        loop {
            cursor.eat_whitespace();
            if cursor.is_done() {
                return Ok(Self {
                    items: Arc::new(items),
                });
            }

            let Some(tree) = TokenTree::parse(cursor)? else {
                return Ok(Self {
                    items: Arc::new(items),
                });
            };
            items.push(tree);
        }
    }
}

impl TokenTree {
    pub(super) fn parse(cursor: &mut Cursor) -> Result<Option<TokenTree>, Error> {
        cursor.eat_whitespace();

        // let span = cursor.rem_stream_span();
        // println!("[TokenTree]\n{}", cursor.asset.slice_for(span));

        // will not be a whitespace
        let Some((mut token, mut span)) = cursor.next() else {
            unimplemented!()
        };

        if token.kind.is_delim_open() {
            let delim = Delimiter::from(token.kind);
            let open = span;

            let mut sub = cursor.extract_subcursor(delim)?;
            let ts = TokenStream::parse(&mut sub)?;
            let close = sub.end_span();

            Ok(TokenTree::Delimited(DelimSpan { open, close }, delim, ts).into())
        } else {
            use crate::lexer::TokenKind::*;

            loop {
                // normal token
                return match token.kind {
                    Dot => {
                        // Check for second dot
                        let Some(next) = cursor.peek(0) else {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Dot, span),
                                Spacing::Alone,
                            )
                            .into());
                        };

                        if next.kind != Dot {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Dot, span),
                                Spacing::infer(token.kind, next.kind),
                            )
                            .into());
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        // check for third dot.
                        let Some(next) = cursor.peek(0) else {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::DotDot, span),
                                Spacing::Alone,
                            )
                            .into());
                        };

                        if next.kind == Eq {
                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::DotDotEq, span),
                                Spacing::Alone,
                            )
                            .into());
                        }

                        if next.kind != Dot {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::DotDot, span),
                                Spacing::infer(token.kind, next.kind),
                            )
                            .into());
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(
                            TokenTree::Token(
                                Token::new(TokenKind::DotDotDot, span),
                                Spacing::Alone,
                            )
                            .into(),
                        )
                    }

                    Colon => Ok(TokenTree::Token(
                        Token::new(TokenKind::Colon, span),
                        Spacing::Alone,
                    )
                    .into()),
                    Slash => Ok(TokenTree::Token(
                        Token::new(TokenKind::Slash, span),
                        Spacing::Alone,
                    )
                    .into()),
                    Semi => Ok(
                        TokenTree::Token(Token::new(TokenKind::Semi, span), Spacing::Alone).into(),
                    ),
                    Comma => Ok(TokenTree::Token(
                        Token::new(TokenKind::Comma, span),
                        Spacing::Alone,
                    )
                    .into()),

                    Plus => Ok(
                        TokenTree::Token(Token::new(TokenKind::Plus, span), Spacing::Alone).into(),
                    ),

                    Eq => {
                        if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Eq, span),
                                Spacing::Alone,
                            )
                            .into());
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        if cursor.peek(0).map(|t| t.kind) != Some(Gt) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::EqEq, span),
                                Spacing::Alone,
                            )
                            .into());
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(TokenTree::Token(
                            Token::new(TokenKind::RDoubleArrow, span),
                            Spacing::Alone,
                        )
                        .into())
                    }

                    Minus => {
                        if cursor.peek(0).map(|t| t.kind) != Some(Minus) {
                            if let Some(lexer::Token {
                                kind: Literal { kind, .. },
                                len,
                            }) = cursor.peek(0)
                            {
                                if matches!(
                                    kind,
                                    LiteralKind::Int { .. } | LiteralKind::Float { .. }
                                ) {
                                    span = Span::new(span.pos, span.len + len);
                                    cursor.bump(1);
                                    let lit = Lit::from_span(kind, span, cursor)?;
                                    return Ok(TokenTree::Token(
                                        Token::new(TokenKind::Literal(lit), span),
                                        Spacing::Alone,
                                    )
                                    .into());
                                }
                            }

                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Minus, span),
                                Spacing::Alone,
                            )
                            .into());
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        if cursor.peek(0).map(|t| t.kind) != Some(Gt) {
                            unimplemented!("-- without -->")
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(TokenTree::Token(
                            Token::new(TokenKind::RSingleArrow, span),
                            Spacing::Alone,
                        )
                        .into())
                    }
                    Lt => match cursor.peek(0).map(|t| t.kind) {
                        Some(Eq) => {
                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                                return Ok(TokenTree::Token(
                                    Token::new(TokenKind::Le, span),
                                    Spacing::Alone,
                                )
                                .into());
                            }

                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            Ok(TokenTree::Token(
                                Token::new(TokenKind::LDoubleArrow, span),
                                Spacing::Alone,
                            )
                            .into())
                        }
                        Some(Minus) => {
                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            if cursor.peek(0).map(|t| t.kind) != Some(Minus) {
                                unimplemented!("<- but not <--")
                            }

                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            if cursor.peek(0).map(|t| t.kind) == Some(Gt) {
                                let (_, s) = cursor.next().unwrap();
                                span = Span::fromto(span, s);
                                Ok(TokenTree::Token(
                                    Token::new(TokenKind::LSingleArrowR, span),
                                    Spacing::Alone,
                                )
                                .into())
                            } else {
                                Ok(TokenTree::Token(
                                    Token::new(TokenKind::LSingleArrow, span),
                                    Spacing::Alone,
                                )
                                .into())
                            }
                        }
                        _ => Ok(
                            TokenTree::Token(Token::new(TokenKind::Lt, span), Spacing::Alone)
                                .into(),
                        ),
                    },
                    Gt => {
                        if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Gt, span),
                                Spacing::Alone,
                            )
                            .into());
                        };

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(
                            TokenTree::Token(Token::new(TokenKind::Ge, span), Spacing::Alone)
                                .into(),
                        )
                    }

                    OpenBrace | OpenBracket | OpenParen => {
                        unreachable!("")
                    }
                    CloseBrace | CloseBracket | CloseParen => Err(Error::new(
                        ErrorKind::MissingDelim,
                        format!("missing opening delimiter for {:?}", token),
                    )),

                    Ident => Ok(TokenTree::Token(
                        Token::new(TokenKind::ident_or_keyword(span, cursor), span),
                        Spacing::Alone,
                    )
                    .into()),
                    Annotation => Ok(TokenTree::Token(
                        Token::new(
                            TokenKind::Annotation(super::Annotation::from_span(span, cursor)),
                            span,
                        ),
                        Spacing::Alone,
                    )
                    .into()),

                    Literal { kind, .. } => Ok(TokenTree::Token(
                        Token::new(
                            TokenKind::Literal(super::Lit::from_span(kind, span, cursor)?),
                            span,
                        ),
                        Spacing::Alone,
                    )
                    .into()),

                    Comment | Whitespace => {
                        match cursor.next() {
                            Some(value) => (token, span) = value,
                            None => {
                                return Ok(None);
                            }
                        }
                        continue;
                    }

                    _ => unimplemented!("missing parser for {token:?}"),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{SourceMap, ast::token};
    use super::*;

    #[test]
    fn recognize_arrows() {
        let mut smap = SourceMap::new();
        let asset = smap.load_raw(
            "raw:token:0",
            "module A {{
            connections {{
                a <--> b,
                c <-- Link --> d
            }}
        }}",
        );

        let ts = TokenStream::new(asset).unwrap();
        let TokenTree::Delimited(_, _, ref module_stmt) = ts.items[2] else {
            unreachable!()
        };
        let TokenTree::Delimited(_, _, ref module_stmt) = module_stmt.items[0] else {
            unreachable!()
        };

        dbg!(module_stmt);

        let TokenTree::Delimited(_, _, ref conn_stmt) = module_stmt.items[1] else {
            unreachable!()
        };

        let TokenTree::Delimited(_, _, ref list) = conn_stmt.items[0] else {
            unreachable!()
        };


        let TokenTree::Token(ref arrow, _) = list.items[1] else {
            unreachable!()
        };
        assert_eq!(arrow.kind, token::TokenKind::LSingleArrowR);

        let TokenTree::Token(ref arrow, _) = list.items[5] else {
            unreachable!()
        };
        assert_eq!(arrow.kind, token::TokenKind::LSingleArrow);
    }
}
