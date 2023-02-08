use crate::ast::parse::*;
use crate::ast::Annotation;
use crate::ast::Keyword;
use crate::ast::Token;
use crate::ast::TokenKind;
use crate::ast::TokenStream;
use crate::ast::TokenTree;

#[macro_use]
mod macros;

mod cluster;
mod delim;
mod eitheror;
mod joined;
mod kv;
mod punct;

pub use self::cluster::*;
pub use self::delim::*;
pub use self::eitheror::*;
pub use self::joined::*;
pub use self::kv::*;
pub use self::punct::*;

// # Tokens

ast_expect_single_token! {
    pub struct Slash {
        token: TokenKind::Slash,
    }
}

ast_expect_single_token! {
    pub struct Dot {
        token: TokenKind::Dot,
    }
}

ast_expect_single_token! {
    pub struct DotDot {
        token: TokenKind::DotDot,
    }
}

ast_expect_single_token! {
    pub struct DotDotDot {
        token: TokenKind::DotDotDot,
    }
}

ast_expect_single_token! {
    pub struct Eq {
        token: TokenKind::Eq,
    }
}

ast_expect_single_token! {
    pub struct Semi {
        token: TokenKind::Semi,
    }
}

ast_expect_single_token! {
    pub struct Comma {
        token: TokenKind::Comma,
    }
}

ast_expect_single_token! {
    pub struct Colon {
        token: TokenKind::Colon,
    }
}

ast_expect_single_token! {
    pub struct Plus {
        token: TokenKind::Plus,
    }
}

ast_expect_single_token! {
    pub struct Minus {
        token: TokenKind::Minus,
    }
}

ast_expect_single_token! {
    pub struct LeftSingleArrow {
        token: TokenKind::LSingleArrow,
    }
}

ast_expect_single_token! {
    pub struct RightSingleArrow {
        token: TokenKind::RSingleArrow,
    }
}

ast_expect_single_token! {
    pub struct IncludeToken {
        token: TokenKind::Keyword(Keyword::Include),
    }
}

ast_expect_single_token! {
    pub struct ModuleToken {
        token: TokenKind::Keyword(Keyword::Module),
    }
}

ast_expect_single_token! {
    pub struct GatesToken {
        token: TokenKind::Keyword(Keyword::Gates),
    }
}

ast_expect_single_token! {
    pub struct SubmodulesToken {
        token: TokenKind::Keyword(Keyword::Submodules),
    }
}

ast_expect_single_token! {
    pub struct ConnectionsToken {
        token: TokenKind::Keyword(Keyword::Connections),
    }
}

ast_expect_single_token! {
    pub struct LinkToken {
        token: TokenKind::Keyword(Keyword::Link),
    }
}

ast_expect_single_token! {
    pub struct EntryToken {
        token: TokenKind::Keyword(Keyword::Entry),
    }
}

// # EXT

pub use crate::ast::token::Ident;
pub use crate::ast::token::Lit;

impl Spanned for Ident {
    fn span(&self) -> crate::Span {
        self.span
    }
}

impl Parse for Ident {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        match input.ts.peek() {
            Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Ident(ident),
                    ..
                },
                _,
            )) => {
                let ident = ident.clone();
                input.ts.bump();
                Ok(ident)
            }
            Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Keyword(keyword),
                    ..
                },
                _,
            )) => Err(Error::new(
                ErrorKind::ExpectedIdentFoundKeyword,
                format!(
                    "unexpected token, expected identifier, found keyword '{}'",
                    keyword
                ),
            )),
            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "unexpected token, expected ident",
            )),
        }
    }
}

impl Spanned for Annotation {
    fn span(&self) -> crate::Span {
        self.span
    }
}

impl Parse for Annotation {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        match input.ts.peek() {
            Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Annotation(annot),
                    ..
                },
                _,
            )) => {
                let annot = annot.clone();
                input.ts.bump();
                Ok(annot)
            }
            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "unexpected token, expected annotation",
            )),
        }
    }
}

impl Spanned for Lit {
    fn span(&self) -> crate::Span {
        self.span
    }
}

impl Parse for Lit {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        match input.ts.peek() {
            Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Literal(lit),
                    ..
                },
                _,
            )) => {
                let lit = lit.clone();
                input.ts.bump();
                Ok(lit)
            }
            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "unexpected token, expected literal",
            )),
        }
    }
}

impl Parse for TokenStream {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.ts.state() == 0 {
            Ok(TokenStream {
                items: input.ts.raw(),
            })
        } else {
            Err(Error::new(ErrorKind::MissingToken, "missing token"))
        }
    }
}
