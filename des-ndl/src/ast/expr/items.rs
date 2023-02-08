use std::sync::Arc;

use super::{EntryStmt, IncludeStmt, LinkStmt, ModuleStmt};
use crate::{
    ast::{parse::*, Keyword, Token, TokenKind, TokenTree},
    Ident,
};

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Include(Arc<IncludeStmt>),
    Link(Arc<LinkStmt>),
    Module(Arc<ModuleStmt>),
    Entry(Arc<EntryStmt>),
}

// # Impl

impl Item {
    pub fn symbol(&self) -> Option<&Ident> {
        match self {
            Item::Include(_) | Item::Entry(_) => None,
            Item::Module(module) => Some(&module.ident),
            Item::Link(link) => Some(&link.ident),
        }
    }
}

// # Spaning

impl Spanned for Item {
    fn span(&self) -> crate::Span {
        match self {
            Self::Entry(entry) => entry.span(),
            Self::Include(include) => include.span(),
            Self::Link(link) => link.span(),
            Self::Module(module) => module.span(),
        }
    }
}

// # Parse

impl Parse for File {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut items = Vec::new();
        while !input.ts.is_empty() {
            items.push(Item::parse(input)?)
        }
        Ok(File { items })
    }
}

impl Parse for Item {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        match input.ts.peek() {
            Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Keyword(keyword),
                    ..
                },
                _,
            )) => match keyword {
                Keyword::Include => Ok(Item::Include(Arc::new(IncludeStmt::parse(input)?))),
                Keyword::Link => Ok(Item::Link(Arc::new(LinkStmt::parse(input)?))),
                Keyword::Module => Ok(Item::Module(Arc::new(ModuleStmt::parse(input)?))),
                Keyword::Entry => Ok(Item::Entry(Arc::new(EntryStmt::parse(input)?))),
                _ => Err(Error::new(
                    ErrorKind::UnexpectedToken,
                    "unexpected keyword, expected top level item",
                )),
            },
            Some(_) => Err(Error::new(
                ErrorKind::UnexpectedToken,
                "unexpected token, expected top level item",
            )),
            None => Err(Error::new(
                ErrorKind::UnexpectedEOF,
                "unexpected end of token stream",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::TokenStream, SourceMap};

    #[test]
    fn simple_top_level_definitions() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "include std; link A {} module B {} entry C;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let file = File::parse(&buf).unwrap();
        assert_eq!(file.items.len(), 4);

        assert!(matches!(file.items[0], Item::Include(_)));
        assert!(matches!(file.items[1], Item::Link(_)));
        assert!(matches!(file.items[2], Item::Module(_)));
        assert!(matches!(file.items[3], Item::Entry(_)));

        // # Case 1
        let asset = smap.load_raw(
            "raw:case1",
            "include std; link A: Super { key: 123, } module B { gates { in, out } } entry C;",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let file = File::parse(&buf).unwrap();
        assert_eq!(file.items.len(), 4);

        assert!(matches!(file.items[0], Item::Include(_)));
        assert!(matches!(file.items[1], Item::Link(_)));
        assert!(matches!(file.items[2], Item::Module(_)));
        assert!(matches!(file.items[3], Item::Entry(_)));
    }
}
