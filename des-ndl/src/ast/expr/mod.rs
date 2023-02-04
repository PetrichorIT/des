use super::parse::*;
use super::token::*;

#[macro_use]
mod macros;

mod delim;
mod include;
mod kv;
mod link;
mod punctuated;

pub use delim::*;
pub use include::*;
pub use kv::*;
pub use link::*;
pub use punctuated::*;

// # Tokens

ast_expect_single_token! {
    pub struct Slash {
        token: TokenKind::Slash,
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
            _ => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "unexpected token, expected ident",
            )),
        }
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
