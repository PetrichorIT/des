use std::fmt;

use self::cursor::Cursor;
use crate::{
    ast::parse::{Error, ErrorKind},
    lexer::{self, tokenize, LiteralKind},
    Asset, Span,
};

pub use stream::DelimSpan;
pub use stream::Spacing;
pub use stream::TokenStream;
pub use stream::TokenTree;

mod cursor;
mod stream;
mod symbol;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub(super) fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    Comma,
    Minus,
    Plus,
    Semi,
    LDoubleArrow,
    RDoubleArrow,
    LSingleArrow,
    RSingleArrow,
    Colon,
    Slash,
    Keyword(Keyword),
    OpenDelim(Delimiter),
    CloseDelim(Delimiter),
    Literal(Lit),
    Ident(Ident),
    Annotation(Annotation),
}

impl TokenKind {
    fn ident_or_keyword(span: Span, cursor: &mut Cursor) -> TokenKind {
        let ident = Ident::from_span(span, cursor);
        match &ident.raw[..] {
            "module" => TokenKind::Keyword(Keyword::Module),
            "gates" => TokenKind::Keyword(Keyword::Gates),
            "submodules" => TokenKind::Keyword(Keyword::Submodules),
            "connections" => TokenKind::Keyword(Keyword::Connections),
            "link" => TokenKind::Keyword(Keyword::Link),
            "include" => TokenKind::Keyword(Keyword::Include),
            "entry" => TokenKind::Keyword(Keyword::Entry),
            _ => TokenKind::Ident(ident),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyword {
    Module,
    Gates,
    Submodules,
    Connections,
    Link,
    Include,
    Entry,
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => write!(f, "module"),
            Self::Gates => write!(f, "gates"),
            Self::Submodules => write!(f, "submodules"),
            Self::Connections => write!(f, "connections"),
            Self::Link => write!(f, "link"),
            Self::Include => write!(f, "include"),
            Self::Entry => write!(f, "entry"),
        }
    }
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

#[derive(Debug, Clone, PartialEq)]
pub enum LitKind {
    Integer { lit: i32 },
    Float { lit: f64 },
    Str { lit: String },
}

impl LitKind {
    pub(super) fn typ(&self) -> &'static str {
        match self {
            Self::Integer { .. } => "integer",
            Self::Float { .. } => "float",
            Self::Str { .. } => "string",
        }
    }
}

impl fmt::Display for LitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer { lit } => write!(f, "{}", lit),
            Self::Float { lit } => write!(f, "{:.1}", lit),
            Self::Str { lit } => write!(f, "\"{}\"", lit),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
                    lit: source
                        .trim_start_matches('"')
                        .trim_end_matches('"')
                        .to_string(),
                },
                span,
            }),
        }
    }

    pub fn as_float(&self) -> f64 {
        match self.kind {
            LitKind::Float { lit } => lit,
            _ => panic!("unexpected cast"),
        }
    }

    pub fn as_integer(&self) -> i32 {
        match self.kind {
            LitKind::Integer { lit } => lit,
            _ => panic!("unexpected cast"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub raw: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub raw: String,
    pub span: Span,
}

impl Ident {
    fn from_span(span: Span, cursor: &Cursor) -> Self {
        Self {
            raw: cursor.asset.slice_for(span).to_string(),
            span,
        }
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self.raw.as_ref()
    }
}

impl PartialEq<&str> for Ident {
    fn eq(&self, other: &&str) -> bool {
        &self.raw == other
    }
}

impl Annotation {
    fn from_span(span: Span, cursor: &Cursor) -> Self {
        Self {
            raw: cursor
                .asset
                .slice_for(span)
                .trim_start_matches('@')
                .to_string(),
            span,
        }
    }
}

impl PartialEq<&str> for Annotation {
    fn eq(&self, other: &&str) -> bool {
        &self.raw == other
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
