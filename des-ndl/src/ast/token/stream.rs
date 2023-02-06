use super::{Cursor, Delimiter, Token};
use crate::{ast::parse::Error, ast::token::TokenKind, lexer, Span};
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

            let tree = TokenTree::parse(cursor)?;
            items.push(tree);
        }
    }
}

impl TokenTree {
    pub(super) fn parse(cursor: &mut Cursor) -> Result<TokenTree, Error> {
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

            // println!(">> {:?}", delim);

            let mut sub = cursor.extract_subcursor(delim)?;
            // let sub_span = sub.rem_stream_span();
            // println!("[Delim]\n{}", sub.asset.slice_for(sub_span));

            let ts = TokenStream::parse(&mut sub)?;

            let close = sub.end_span();

            Ok(TokenTree::Delimited(DelimSpan { open, close }, delim, ts))
        } else {
            use crate::lexer::TokenKind::*;

            loop {
                // normal token
                return match token.kind {
                    Dot => {
                        // Check for second dot
                        let Some(next) = cursor.peek(0) else {
                            return Ok(TokenTree::Token(Token::new(TokenKind::Dot, span), Spacing::Alone))
                        };

                        if next.kind != Dot {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Dot, span),
                                Spacing::infer(token.kind, next.kind),
                            ));
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        // check for third dot.
                        let Some(next) = cursor.peek(0) else {
                            return Ok(TokenTree::Token(Token::new(TokenKind::DotDot, span), Spacing::Alone))
                        };

                        if next.kind == Eq {
                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::DotDotEq, span),
                                Spacing::Alone,
                            ));
                        }

                        if next.kind != Dot {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::DotDot, span),
                                Spacing::infer(token.kind, next.kind),
                            ));
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(TokenTree::Token(
                            Token::new(TokenKind::DotDotDot, span),
                            Spacing::Alone,
                        ))
                    }

                    Colon => Ok(TokenTree::Token(
                        Token::new(TokenKind::Colon, span),
                        Spacing::Alone,
                    )),
                    Slash => Ok(TokenTree::Token(
                        Token::new(TokenKind::Slash, span),
                        Spacing::Alone,
                    )),
                    Semi => Ok(TokenTree::Token(
                        Token::new(TokenKind::Semi, span),
                        Spacing::Alone,
                    )),
                    Comma => Ok(TokenTree::Token(
                        Token::new(TokenKind::Comma, span),
                        Spacing::Alone,
                    )),

                    Plus => Ok(TokenTree::Token(
                        Token::new(TokenKind::Plus, span),
                        Spacing::Alone,
                    )),

                    Eq => {
                        if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Eq, span),
                                Spacing::Alone,
                            ));
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        if cursor.peek(0).map(|t| t.kind) != Some(Gt) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::EqEq, span),
                                Spacing::Alone,
                            ));
                        }

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(TokenTree::Token(
                            Token::new(TokenKind::RDoubleArrow, span),
                            Spacing::Alone,
                        ))
                    }
                    Minus => {
                        if cursor.peek(0).map(|t| t.kind) != Some(Minus) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Minus, span),
                                Spacing::Alone,
                            ));
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
                        ))
                    }
                    Lt => match cursor.peek(0).map(|t| t.kind) {
                        Some(Eq) => {
                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                                return Ok(TokenTree::Token(
                                    Token::new(TokenKind::Le, span),
                                    Spacing::Alone,
                                ));
                            }

                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            Ok(TokenTree::Token(
                                Token::new(TokenKind::LDoubleArrow, span),
                                Spacing::Alone,
                            ))
                        }
                        Some(Minus) => {
                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            if cursor.peek(0).map(|t| t.kind) != Some(Minus) {
                                unimplemented!("<- but not <--")
                            }

                            let (_, s) = cursor.next().unwrap();
                            span = Span::fromto(span, s);

                            Ok(TokenTree::Token(
                                Token::new(TokenKind::LSingleArrow, span),
                                Spacing::Alone,
                            ))
                        }
                        _ => Ok(TokenTree::Token(
                            Token::new(TokenKind::Lt, span),
                            Spacing::Alone,
                        )),
                    },
                    Gt => {
                        if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                            return Ok(TokenTree::Token(
                                Token::new(TokenKind::Gt, span),
                                Spacing::Alone,
                            ));
                        };

                        let (_, s) = cursor.next().unwrap();
                        span = Span::fromto(span, s);

                        Ok(TokenTree::Token(
                            Token::new(TokenKind::Ge, span),
                            Spacing::Alone,
                        ))
                    }

                    OpenBrace | OpenBracket | OpenParen => {
                        todo!();
                    }
                    CloseBrace | CloseBracket | CloseParen => {
                        todo!()
                    }

                    Ident => Ok(TokenTree::Token(
                        Token::new(TokenKind::ident_or_keyword(span, cursor), span),
                        Spacing::Alone,
                    )),
                    Annotation => Ok(TokenTree::Token(
                        Token::new(
                            TokenKind::Annotation(super::Annotation::from_span(span, cursor)),
                            span,
                        ),
                        Spacing::Alone,
                    )),

                    Literal { kind, .. } => Ok(TokenTree::Token(
                        Token::new(
                            TokenKind::Literal(super::Lit::from_span(kind, span, cursor)?),
                            span,
                        ),
                        Spacing::Alone,
                    )),

                    Comment | Whitespace => {
                        match cursor.next() {
                            Some(value) => (token, span) = value,
                            None => unimplemented!(),
                        }
                        continue;
                    }

                    _ => unimplemented!("missing parser for {token:?}"),
                };
            }
        }
    }
}
