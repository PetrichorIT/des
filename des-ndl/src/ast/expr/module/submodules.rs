use crate::ast::parse::*;
use crate::ast::{
    ClusterDefinition, Colon, Comma, Delimited, Delimiter, Ident, Punctuated, SubmodulesToken,
};
use crate::resource::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct SubmodulesStmt {
    pub keyword: SubmodulesToken,
    pub span: Span,
    pub items: Punctuated<SubmoduleDefinition, Comma>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubmoduleDefinition {
    pub ident: Ident,
    pub cluster: Option<ClusterDefinition>,
    pub colon: Colon,
    pub typ: Ident,
}

// # Spanning

impl Spanned for SubmodulesStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for SubmoduleDefinition {
    fn span(&self) -> Span {
        Span::fromto(self.ident.span(), self.typ.span())
    }
}

// # Parsing

impl Parse for SubmodulesStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let keyword = SubmodulesToken::parse(input)?;
        let delim = Delimited::<Punctuated<SubmoduleDefinition, Comma>>::parse_from(
            Delimiter::Brace,
            input,
        )?;
        let span = Span::fromto(delim.delim_span.open, delim.delim_span.close);
        Ok(SubmodulesStmt {
            keyword,
            span,
            items: delim.inner,
        })
    }
}

impl Parse for SubmoduleDefinition {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident = Ident::parse(input)?;
        let cluster = Option::<ClusterDefinition>::parse(input)?;
        let colon = Colon::parse(input)?;
        let typ = Ident::parse(input)?;
        Ok(SubmoduleDefinition {
            ident,
            cluster,
            colon,
            typ,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::TokenStream;
    use crate::resource::SourceMap;

    #[test]
    fn single_submodules() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "submodules { parent: P, child: C }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = SubmodulesStmt::parse(&buf).unwrap();
        assert_eq!(
            expr.items
                .iter()
                .cloned()
                .map(|v| (
                    v.ident.raw,
                    v.typ.raw,
                    v.cluster.map(|v| format!("{}", v.lit.kind))
                ))
                .collect::<Vec<_>>(),
            vec![
                ("parent".to_string(), "P".to_string(), None),
                ("child".to_string(), "C".to_string(), None),
            ]
        );

        // # Case 1
        let asset = smap.load_raw("raw:case1", "submodules { parent: P, child: _C, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = SubmodulesStmt::parse(&buf).unwrap();
        assert_eq!(
            expr.items
                .iter()
                .cloned()
                .map(|v| (
                    v.ident.raw,
                    v.typ.raw,
                    v.cluster.map(|v| format!("{}", v.lit.kind))
                ))
                .collect::<Vec<_>>(),
            vec![
                ("parent".to_string(), "P".to_string(), None),
                ("child".to_string(), "_C".to_string(), None),
            ]
        );
    }

    #[test]
    fn clusteed_submodules() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "submodules { parent[1]: P, child[\"str\"]: C }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = SubmodulesStmt::parse(&buf).unwrap();
        assert_eq!(
            expr.items
                .iter()
                .cloned()
                .map(|v| (
                    v.ident.raw,
                    v.typ.raw,
                    v.cluster.map(|v| format!("{}", v.lit.kind))
                ))
                .collect::<Vec<_>>(),
            vec![
                ("parent".to_string(), "P".to_string(), Some("1".to_string())),
                (
                    "child".to_string(),
                    "C".to_string(),
                    Some("\"str\"".to_string())
                ),
            ]
        );

        // # Case 1
        let asset = smap.load_raw("raw:case1", "submodules { parent[10]: P, child: _C, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = SubmodulesStmt::parse(&buf).unwrap();
        assert_eq!(
            expr.items
                .iter()
                .cloned()
                .map(|v| (
                    v.ident.raw,
                    v.typ.raw,
                    v.cluster.map(|v| format!("{}", v.lit.kind))
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "parent".to_string(),
                    "P".to_string(),
                    Some("10".to_string())
                ),
                ("child".to_string(), "_C".to_string(), None),
            ]
        );
    }
}
