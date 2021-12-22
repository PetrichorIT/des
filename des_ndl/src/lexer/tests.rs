#[test]
fn test_token_lexer() {
    use super::{Base, LiteralKind};
    use crate::{tokenize, Token, TokenKind};

    let contents = std::fs::read_to_string("./tests/LexTest.ndl")
        .expect("Failed to read static test file 'LexTest.ndl'");

    let token_stream = tokenize(&contents, 0).collect::<Vec<Token>>();

    assert_eq!(
        token_stream,
        vec![
            Token::new(TokenKind::Ident, 0, 6, 1),
            Token::new(TokenKind::Whitespace, 6, 1, 1),
            Token::new(TokenKind::And, 7, 1, 1),
            Token::new(TokenKind::Ident, 8, 1, 1),
            Token::new(TokenKind::Whitespace, 9, 1, 1),
            Token::new(TokenKind::OpenBrace, 10, 1, 1),
            Token::new(TokenKind::Whitespace, 11, 6, 1),
            Token::new(TokenKind::Plus, 17, 1, 2),
            Token::new(TokenKind::Minus, 18, 1, 2),
            Token::new(TokenKind::Whitespace, 19, 1, 2),
            Token::new(TokenKind::Colon, 20, 1, 2),
            Token::new(TokenKind::Whitespace, 21, 1, 2),
            Token::new(
                TokenKind::Literal {
                    kind: LiteralKind::Int {
                        base: Base::Decimal,
                        empty_int: false
                    },
                    suffix_start: 6
                },
                22,
                6,
                2
            ),
            Token::new(TokenKind::Whitespace, 28, 2, 2),
            Token::new(TokenKind::CloseBrace, 30, 1, 3),
        ]
    );

    let token_stream = token_stream
        .into_iter()
        .filter(|t| t.kind.valid() && !t.kind.reducable())
        .collect::<Vec<Token>>();

    assert_eq!(
        token_stream,
        vec![
            Token::new(TokenKind::Ident, 0, 6, 1),
            Token::new(TokenKind::Whitespace, 6, 1, 1),
            Token::new(TokenKind::Ident, 8, 1, 1),
            Token::new(TokenKind::Whitespace, 9, 1, 1),
            Token::new(TokenKind::OpenBrace, 10, 1, 1),
            Token::new(TokenKind::Whitespace, 11, 6, 1),
            Token::new(TokenKind::Minus, 18, 1, 2),
            Token::new(TokenKind::Whitespace, 19, 1, 2),
            Token::new(TokenKind::Colon, 20, 1, 2),
            Token::new(TokenKind::Whitespace, 21, 1, 2),
            Token::new(
                TokenKind::Literal {
                    kind: LiteralKind::Int {
                        base: Base::Decimal,
                        empty_int: false
                    },
                    suffix_start: 6
                },
                22,
                6,
                2
            ),
            Token::new(TokenKind::Whitespace, 28, 2, 2),
            Token::new(TokenKind::CloseBrace, 30, 1, 3),
        ]
    )
}
