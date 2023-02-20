use crate::{
    ast::{
        parse::*, Colon, Delimited, Delimiter, Ident, Joined, Keyword, ModuleToken, Plus, Token,
        TokenKind, TokenStream, TokenTree,
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
    pub inheritance: Option<ModuleInheritance>,
    pub gates: Vec<GatesStmt>,
    pub submodules: Vec<SubmodulesStmt>,
    pub connections: Vec<ConnectionsStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleInheritance {
    pub colon: Colon,
    pub symbols: Joined<Ident, Plus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleTypus {
    Primal,
    Inherited,
    Dynamic,
}

impl ModuleStmt {
    pub fn typus(&self) -> ModuleTypus {
        if self.inheritance.is_some() {
            return ModuleTypus::Inherited;
        }
        if self
            .submodules
            .iter()
            .any(|st| st.items.iter().any(|s| s.typ.is_dyn()))
        {
            return ModuleTypus::Dynamic;
        }
        ModuleTypus::Primal
    }
}

impl Spanned for ModuleStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ModuleInheritance {
    fn span(&self) -> Span {
        Span::fromto(self.colon.span(), self.symbols.span())
    }
}

impl Parse for ModuleStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let keyword = ModuleToken::parse(input)?;
        let ident = Ident::parse(input).map_err(|e| {
            let f = format!("{}", e.internal);
            e.override_internal(format!("unexpected token for module symbol: {f}"))
        })?;

        let inheritance = Option::<ModuleInheritance>::parse(input)?;
        let delim = Delimited::<TokenStream>::parse_from(Delimiter::Brace, input)?;
        let inner = ParseBuffer::new(input.asset, delim.inner);
        let span = Span::fromto(keyword.span(), delim.delim_span.close);

        let mut this = ModuleStmt {
            keyword,
            ident,
            inheritance,
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

impl Parse for Option<ModuleInheritance> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let colon = match Colon::parse(input) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        let symbols = Joined::<Ident, Plus>::parse(input)?;
        Ok(Some(ModuleInheritance { colon, symbols }))
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

    #[test]
    fn inheritance() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "module A: B + C { }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = ModuleStmt::parse(&buf).unwrap();
        assert!(stmt.inheritance.is_some())
    }
}
