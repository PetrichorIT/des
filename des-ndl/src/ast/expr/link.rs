use crate::{
    ast::{
        parse::*, Colon, Comma, Delimited, Delimiter, Ident, Joined, KeyValueField, LinkToken, Lit,
        Plus, Punctuated,
    },
    error::*,
    resource::Span,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LinkStmt {
    pub link_token: LinkToken,
    pub ident: Ident,
    pub inheritance: Option<LinkInheritance>,
    pub data: LinkData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkInheritance {
    pub colon: Colon,
    pub symbols: Joined<Ident, Plus>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub items: Punctuated<KeyValueField<Ident, Lit, Colon>, Comma>,
    pub span: Span,
}

// # Spanning

impl Spanned for LinkStmt {
    fn span(&self) -> crate::Span {
        Span::fromto(self.link_token.span(), self.data.span())
    }
}

impl Spanned for LinkInheritance {
    fn span(&self) -> Span {
        Span::fromto(self.colon.span(), self.symbols.span())
    }
}

impl Spanned for LinkData {
    fn span(&self) -> Span {
        self.span
    }
}

// # Parsing

impl Parse for LinkStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let link_token = LinkToken::parse(input)?;
        let ident = Ident::parse(input)?;
        let inheritance = Option::<LinkInheritance>::parse(input)?;
        let data = LinkData::parse(input)?;

        Ok(Self {
            link_token,
            ident,
            inheritance,
            data,
        })
    }
}

impl Parse for Option<LinkInheritance> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let colon = match Colon::parse(input) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        let symbols = Joined::<Ident, Plus>::parse(input).unwrap();
        Ok(Some(LinkInheritance { colon, symbols }))
    }
}

impl Parse for LinkData {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let items = Delimited::<Punctuated<KeyValueField<Ident, Lit, Colon>, Comma>>::parse_from(
            Delimiter::Brace,
            input,
        )?;
        let span = Span::fromto(items.delim_span.open, items.delim_span.close);
        Ok(Self {
            items: items.inner,
            span,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{parse::*, LinkStmt, TokenStream};
    use crate::resource::SourceMap;

    #[test]
    fn multiple_lit_parse() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "link FastLink { ident: 123, other: 1.0 }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = LinkStmt::parse(&buf).unwrap();
        assert_eq!(expr.ident, "FastLink");
        assert_eq!(expr.link_token.span.pos, 0);
        assert_eq!(
            expr.data
                .items
                .iter()
                .cloned()
                .map(|v| (v.key.raw, format!("{}", v.value.kind)))
                .collect::<Vec<_>>(),
            vec![
                ("ident".to_string(), "123".to_string()),
                ("other".to_string(), "1.0".to_string())
            ]
        );

        // # Case 2
        let asset = smap.load_raw("raw:case2", "link FastLink { ident: 123, other: 1.0, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = LinkStmt::parse(&buf).unwrap();
        assert_eq!(expr.ident, "FastLink");
        assert_eq!(
            expr.data
                .items
                .iter()
                .cloned()
                .map(|v| (v.key.raw, format!("{}", v.value.kind)))
                .collect::<Vec<_>>(),
            vec![
                ("ident".to_string(), "123".to_string()),
                ("other".to_string(), "1.0".to_string())
            ]
        );
    }

    #[test]
    fn inheritance_statement() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "link FastLink: pident { ident: 123, other: 1.0 }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = LinkStmt::parse(&buf).unwrap();
        assert_eq!(expr.ident, "FastLink");
        assert_eq!(
            expr.inheritance
                .map(|v| v.symbols.iter().cloned().collect::<Vec<_>>())
                .unwrap(),
            vec!["pident"]
        );
        assert_eq!(expr.link_token.span.pos, 0);
        assert_eq!(
            expr.data
                .items
                .iter()
                .cloned()
                .map(|v| (v.key.raw, format!("{}", v.value.kind)))
                .collect::<Vec<_>>(),
            vec![
                ("ident".to_string(), "123".to_string()),
                ("other".to_string(), "1.0".to_string())
            ]
        );

        // # Case 2
        let asset = smap.load_raw(
            "raw:case2",
            "link FastLink: A + B + C { ident: 123, other: 1.0, }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = LinkStmt::parse(&buf).unwrap();
        assert_eq!(expr.ident, "FastLink");
        assert_eq!(
            expr.inheritance
                .map(|v| v.symbols.iter().cloned().collect::<Vec<_>>())
                .unwrap(),
            vec!["A", "B", "C"]
        );
        assert_eq!(
            expr.data
                .items
                .iter()
                .cloned()
                .map(|v| (v.key.raw, format!("{}", v.value.kind)))
                .collect::<Vec<_>>(),
            vec![
                ("ident".to_string(), "123".to_string()),
                ("other".to_string(), "1.0".to_string())
            ]
        );
    }
}
