use super::{Cursor, Delimiter, Token};
use crate::{ast::token::TokenKind, lexer, Error, Span};
use std::sync::Arc;

pub struct TokenStream {
    pub(super) items: Arc<Vec<TokenTree>>,
}

pub enum TokenTree {
    Token(Token, Spacing),
    Delimited(DelimSpan, Delimiter, TokenStream),
}

pub enum Spacing {
    Alone,
    Joint,
}

impl Spacing {
    fn infer(token: lexer::TokenKind, next: lexer::TokenKind) -> Spacing {
        todo!()
    }
}

pub struct DelimSpan {
    open: Span,
    close: Span,
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
        // will not be a whitespace
        let Some((token, mut span)) = cursor.next() else {
            unimplemented!()
        };

        if token.kind.is_delim_open() {
            let delim = Delimiter::from(token.kind);
            let open = span;

            let mut sub = cursor.extract_subcursor(delim)?;
            let ts = TokenStream::parse(&mut sub)?;
            sub.bump_back(1);
            let close = sub.peek_span();

            Ok(TokenTree::Delimited(DelimSpan { open, close }, delim, ts))
        } else {
            use crate::lexer::TokenKind::*;
            // normal token
            match token.kind {
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
                        Token::new(TokenKind::RArrow, span),
                        Spacing::Alone,
                    ))
                }
                Lt => {
                    if cursor.peek(0).map(|t| t.kind) != Some(Eq) {
                        return Ok(TokenTree::Token(
                            Token::new(TokenKind::Lt, span),
                            Spacing::Alone,
                        ));
                    };

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
                        Token::new(TokenKind::LArrow, span),
                        Spacing::Alone,
                    ))
                }
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
                    Token::new(
                        TokenKind::Ident(super::Ident::from_span(span, cursor)),
                        span,
                    ),
                    Spacing::Alone,
                )),
                Annotation => Ok(TokenTree::Token(
                    Token::new(
                        TokenKind::Annotation(super::Annotation {
                            ident: super::Ident::from_span(span, cursor),
                        }),
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

                _ => unimplemented!(),
            }
        }
    }
}
