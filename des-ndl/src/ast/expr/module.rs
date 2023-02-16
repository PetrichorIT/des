use crate::{
    ast::{
        parse::*, Delimited, Delimiter, Ident, Keyword, ModuleToken, Token, TokenKind, TokenStream,
        TokenTree,
    },
    error::*,
    Span,
};

mod connections;
mod gates;
mod submodules;

pub use connections::*;
pub use gates::*;
pub use submodules::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleStmt {
    pub keyword: ModuleToken,
    pub ident: Ident,
    pub gates: Vec<GatesStmt>,
    pub submodules: Vec<SubmodulesStmt>,
    pub connections: Vec<ConnectionsStmt>,
    pub span: Span,
}

impl Spanned for ModuleStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Parse for ModuleStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let keyword = ModuleToken::parse(input)?;
        let ident = Ident::parse(input).map_err(|e| {
            let f = format!("{}", e.internal);
            e.override_internal(format!("unexpected token for module symbol: {f}"))
        })?;

        let delim = Delimited::<TokenStream>::parse_from(Delimiter::Brace, input)?;
        let inner = ParseBuffer::new(input.asset, delim.inner);
        let span = Span::fromto(keyword.span(), delim.delim_span.close);

        let mut this = ModuleStmt {
            keyword,
            ident,
            gates: Vec::new(),
            submodules: Vec::new(),
            connections: Vec::new(),
            span,
        };

        while !inner.ts.is_empty() {
            match inner.ts.peek() {
                Some(TokenTree::Token(
                    Token {
                        kind: TokenKind::Keyword(Keyword::Gates),
                        ..
                    },
                    _,
                )) => this.gates.push(GatesStmt::parse(&inner)?),

                Some(TokenTree::Token(
                    Token {
                        kind: TokenKind::Keyword(Keyword::Submodules),
                        ..
                    },
                    _,
                )) => this.submodules.push(SubmodulesStmt::parse(&inner)?),

                Some(TokenTree::Token(
                    Token {
                        kind: TokenKind::Keyword(Keyword::Connections),
                        ..
                    },
                    _,
                )) => this.connections.push(ConnectionsStmt::parse(&inner)?),

                Some(_other) => {
                    return Err(Error::new(
                        ErrorKind::ExpectedInModuleKeyword,
                        "expected keyword 'gates', 'submodules' or 'connections'",
                    ))
                }
                None => unreachable!(),
            }
        }

        Ok(this)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceMap;

    #[test]
    fn empty_module() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "module A {}");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ModuleStmt::parse(&buf).unwrap();
        assert_eq!(stmt.ident, "A");
        assert_eq!(stmt.gates, vec![]);
        assert_eq!(stmt.submodules, vec![]);
        assert_eq!(stmt.connections, vec![]);
    }

    #[test]
    fn keyword_triggered_modules() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "module A { gates {} connections {} submodules {}}",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ModuleStmt::parse(&buf).unwrap();
        assert_eq!(stmt.ident, "A");
        assert!(stmt.gates.len() > 0);
        assert!(stmt.submodules.len() > 0);
        assert!(stmt.connections.len() > 0);
    }

    #[test]
    fn invalid_tokens() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "module A { 123 }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = ModuleStmt::parse(&buf).unwrap_err();
    }
}
