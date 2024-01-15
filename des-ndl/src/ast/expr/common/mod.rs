use crate::ast::parse::*;
use crate::ast::Annotation;
use crate::ast::Keyword;
use crate::ast::Token;
use crate::ast::TokenKind;
use crate::ast::TokenStream;
use crate::ast::TokenTree;
use crate::error::*;
use crate::Span;

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
    pub struct Dot {
        token: TokenKind::Dot,
    }
    pub struct DotDot {
        token: TokenKind::DotDot,
    }
    pub struct DotDotDot {
        token: TokenKind::DotDotDot,
    }
    pub struct Eq {
        token: TokenKind::Eq,
    }
    pub struct Semi {
        token: TokenKind::Semi,
    }
    pub struct Comma {
        token: TokenKind::Comma,
    }
    pub struct Colon {
        token: TokenKind::Colon,
    }
    pub struct Plus {
        token: TokenKind::Plus,
    }
    pub struct Minus {
        token: TokenKind::Minus,
    }
    pub struct LeftSingleArrow {
        token: TokenKind::LSingleArrow,
    }
    pub struct LeftRightSingleArrow {
        token: TokenKind::LSingleArrowR,
    }
    pub struct RightSingleArrow {
        token: TokenKind::RSingleArrow,
    }
    pub struct IncludeToken {
        token: TokenKind::Keyword(Keyword::Include),
    }
    pub struct ModuleToken {
        token: TokenKind::Keyword(Keyword::Module),
    }
    pub struct GatesToken {
        token: TokenKind::Keyword(Keyword::Gates),
    }
    pub struct SubmodulesToken {
        token: TokenKind::Keyword(Keyword::Submodules),
    }
    pub struct ConnectionsToken {
        token: TokenKind::Keyword(Keyword::Connections),
    }
    pub struct LinkToken {
        token: TokenKind::Keyword(Keyword::Link),
    }
    pub struct EntryToken {
        token: TokenKind::Keyword(Keyword::Entry),
    }
    pub struct DynToken {
        token: TokenKind::Keyword(Keyword::Dyn),
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
                format!("expected <ident>, found <keyword> '{}'", keyword),
            )),

            Some(TokenTree::Token(token, _)) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                format!(
                    "expected <ident>, found {}",
                    token.kind.token_kind_err_output()
                ),
            )
            .spanned(token.span)),

            Some(TokenTree::Delimited(delim, _, _)) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "expected <ident>, found delim",
            )
            .spanned(Span::fromto(delim.open, delim.close))),

            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "expected <ident>, found EOF",
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

            Some(TokenTree::Token(token, _)) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                format!(
                    "expected <annotation>, found {}",
                    token.kind.token_kind_err_output()
                ),
            )
            .spanned(token.span)),

            Some(TokenTree::Delimited(delim, _, _)) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "expected <annotation>, found delim",
            )
            .spanned(Span::fromto(delim.open, delim.close))),

            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "expected <annotation>, found EOF",
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

            Some(TokenTree::Token(token, _)) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                format!(
                    "expected <literal>, found {}",
                    token.kind.token_kind_err_output()
                ),
            )
            .spanned(token.span)),

            Some(TokenTree::Delimited(delim, _, _)) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "expected <literal>, found delim",
            )
            .spanned(Span::fromto(delim.open, delim.close))),

            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "expected <literal>, found EOF",
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
