use crate::lexer::{Base, LiteralKind, Token, TokenKind};

use super::tokenize;

#[test]
fn token_examples() {
    const EX_1: &'static str = "ident./@ \t\nA[]";
    let stream = tokenize(EX_1, 0).collect::<Vec<_>>();
    assert_eq!(
        stream,
        vec![
            Token::new(TokenKind::Ident, 5),
            Token::new(TokenKind::Dot, 1),
            Token::new(TokenKind::Slash, 1),
            Token::new(TokenKind::At, 1),
            Token::new(TokenKind::Whitespace, 3),
            Token::new(TokenKind::Ident, 1),
            Token::new(TokenKind::OpenBracket, 1),
            Token::new(TokenKind::CloseBracket, 1),
        ]
    );

    const EX_2: &'static str = "123, ;#\n// A \t\nident";
    let stream = tokenize(EX_2, 0).collect::<Vec<_>>();
    assert_eq!(
        stream,
        vec![
            Token::new(
                TokenKind::Literal {
                    kind: LiteralKind::Int {
                        base: Base::Decimal,
                        empty_int: false
                    },
                    suffix_start: 3,
                },
                3
            ),
            Token::new(TokenKind::Comma, 1),
            Token::new(TokenKind::Whitespace, 1),
            Token::new(TokenKind::Semi, 1),
            Token::new(TokenKind::Pound, 1),
            Token::new(TokenKind::Whitespace, 1),
            Token::new(TokenKind::Comment, 6),
            Token::new(TokenKind::Whitespace, 1),
            Token::new(TokenKind::Ident, 5),
        ]
    );
}

#[test]
fn token_lex_literal() {
    let token = tokenize("1234", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Decimal,
                    empty_int: false
                },
                suffix_start: 4
            },
            len: 4,
        }
    );

    let token = tokenize("0xa1234", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Hexadecimal,
                    empty_int: false
                },
                suffix_start: 7
            },
            len: 7,
        }
    );

    let token = tokenize("0b101010", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Binary,
                    empty_int: false
                },
                suffix_start: 8
            },
            len: 8,
        }
    );

    let token = tokenize("0x", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Hexadecimal,
                    empty_int: true
                },
                suffix_start: 2
            },
            len: 2,
        }
    );

    let token = tokenize("0.0", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Float {
                    base: Base::Decimal,
                    empty_exp: false
                },
                suffix_start: 3
            },
            len: 3,
        }
    );

    let token = tokenize("1231231230.01231231236", 0)
        .collect::<Vec<_>>()
        .remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Float {
                    base: Base::Decimal,
                    empty_exp: false
                },
                suffix_start: 22
            },
            len: 22,
        }
    );

    let token = tokenize("0b0.0", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Float {
                    base: Base::Binary,
                    empty_exp: false
                },
                suffix_start: 5
            },
            len: 5,
        }
    );
}

#[test]
fn token_lex_literal_str() {
    let token = tokenize("\"ba@#123c\"", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Str { terminated: true },
                suffix_start: 9
            },
            len: 10,
        }
    );
}

#[test]
fn token_lex_ident() {
    let token = tokenize("abc", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Ident,
            len: 3,
        }
    );

    let token = tokenize("abc1", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Ident,
            len: 4,
        }
    );

    let token = tokenize("_abc1", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Ident,
            len: 5,
        }
    );
}

#[test]
fn token_lex_annotation() {
    let token = tokenize("@abc", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Annotation,
            len: 4,
        }
    );

    let token = tokenize("@abc1", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Annotation,
            len: 5,
        }
    );

    let token = tokenize("@_abc", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::Annotation,
            len: 5,
        }
    );

    let token = tokenize("@ abc", 0).collect::<Vec<_>>().remove(0);
    assert_eq!(
        token,
        Token {
            kind: TokenKind::At,
            len: 1,
        }
    );
}
