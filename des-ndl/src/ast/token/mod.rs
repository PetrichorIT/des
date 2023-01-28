use self::cursor::Cursor;
use crate::{
    lexer::{self, tokenize, LiteralKind},
    Asset, Error, ErrorKind, Span,
};

pub use stream::TokenStream;

mod cursor;
mod stream;
mod symbol;

pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub(super) fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}

pub enum TokenKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    EqEq,
    Dot,
    DotDot,
    DotDotDot,
    DotDotEq,
    Comman,
    Semi,
    LArrow,
    RArrow,
    Colon,
    Slash,
    OpenDelim(Delimiter),
    CloseDelim(Delimiter),
    Literal(Lit),
    Ident(Ident),
    Annotation(Annotation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    Parenthesis,
    Brace,
    Bracket,
    Invisible,
}

impl Delimiter {
    fn from(kind: lexer::TokenKind) -> Self {
        match kind {
            lexer::TokenKind::OpenParen => Delimiter::Parenthesis,
            lexer::TokenKind::OpenBrace => Delimiter::Brace,
            lexer::TokenKind::OpenBracket => Delimiter::Bracket,
            _ => unimplemented!(),
        }
    }

    fn open(&self) -> lexer::TokenKind {
        match self {
            Delimiter::Parenthesis => lexer::TokenKind::OpenParen,
            Delimiter::Brace => lexer::TokenKind::OpenBrace,
            Delimiter::Bracket => lexer::TokenKind::OpenBracket,
            _ => unimplemented!(),
        }
    }

    fn close(&self) -> lexer::TokenKind {
        match self {
            Delimiter::Parenthesis => lexer::TokenKind::CloseParen,
            Delimiter::Brace => lexer::TokenKind::CloseBrace,
            Delimiter::Bracket => lexer::TokenKind::CloseBracket,
            _ => unimplemented!(),
        }
    }
}

pub enum LitKind {
    Integer { lit: i32 },
    Float { lit: f64 },
    Str { lit: String },
}

pub struct Lit {
    pub kind: LitKind,
    pub span: Span,
}

impl Lit {
    fn from_span(kind: LiteralKind, span: Span, cursor: &Cursor) -> Result<Self, Error> {
        let source = cursor.asset.slice_for(span);
        match kind {
            LiteralKind::Int { .. } => Ok(Lit {
                kind: LitKind::Integer {
                    lit: source
                        .parse()
                        .map_err(|e| Error::new(ErrorKind::ParseLitError, e))?,
                },
                span,
            }),
            LiteralKind::Float { .. } => Ok(Lit {
                kind: LitKind::Float {
                    lit: source
                        .parse()
                        .map_err(|e| Error::new(ErrorKind::ParseLitError, e))?,
                },
                span,
            }),
            LiteralKind::Str { .. } => Ok(Lit {
                kind: LitKind::Str {
                    lit: source.to_string(),
                },
                span,
            }),
        }
    }
}

pub struct Ident {
    pub raw: String,
    pub span: Span,
}

pub struct Annotation {
    pub ident: Ident,
}

impl Ident {
    fn from_span(span: Span, cursor: &Cursor) -> Self {
        Self {
            raw: cursor.asset.slice_for(span).to_string(),
            span,
        }
    }
}
// # main

impl TokenStream {
    pub fn new(asset: Asset) -> Result<TokenStream, Error> {
        let ts = tokenize(asset.source(), 0).collect::<Vec<_>>();
        let mut cursor = Cursor::new(&ts, asset.source_span().pos, &asset);

        TokenStream::parse(&mut cursor)
    }
}
