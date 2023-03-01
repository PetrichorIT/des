use crate::{
    ast::{
        parse::*, ClusterDefinition, Colon, Comma, Delimited, Delimiter, DynToken, Eq, Ident,
        KeyValueField, Keyword, Punctuated, SubmodulesToken, Token, TokenKind, TokenTree,
    },
    error::Result,
    resource::Span,
};

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
    pub typ: SubmoduleTyp,
    pub dyn_spec: Option<SubmoduleDynSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubmoduleTyp {
    Static(Ident),
    Dynamic(DynToken, Ident),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubmoduleDynSpec {
    pub span: Span,
    pub items: Punctuated<KeyValueField<Ident, Ident, Eq>, Comma>,
}

// # Impl

impl SubmoduleTyp {
    pub fn raw(&self) -> String {
        match self {
            Self::Static(s) => s.raw.clone(),
            Self::Dynamic(_, s) => s.raw.clone(),
        }
    }

    pub fn is_dyn(&self) -> bool {
        matches!(self, Self::Dynamic(_, _))
    }
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

impl Spanned for SubmoduleTyp {
    fn span(&self) -> Span {
        match self {
            Self::Static(ident) => ident.span(),
            Self::Dynamic(dyn_token, ident) => Span::fromto(dyn_token.span(), ident.span()),
        }
    }
}

impl Spanned for SubmoduleDynSpec {
    fn span(&self) -> Span {
        self.span
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
        let typ = SubmoduleTyp::parse(input)?;
        let dyn_spec = Option::<SubmoduleDynSpec>::parse(input)?;
        Ok(SubmoduleDefinition {
            ident,
            cluster,
            colon,
            typ,
            dyn_spec,
        })
    }
}

impl Parse for SubmoduleTyp {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let peek = input.ts.peek();
        match peek {
            Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Keyword(Keyword::Dyn),
                    ..
                },
                _,
            )) => {
                let keyword = DynToken::parse(input)?;
                let ident = Ident::parse(input)?;
                Ok(Self::Dynamic(keyword, ident))
            }
            _ => Ok(Self::Static(Ident::parse(input)?)),
        }
    }
}

impl Parse for Option<SubmoduleDynSpec> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let delim =
            Delimited::<Punctuated<KeyValueField<Ident, Ident, Eq>, Comma>>::parse_option_from(
                Delimiter::Brace,
                input,
            )?;
        if let Some(delim) = delim {
            let span = delim.span();
            Ok(Some(SubmoduleDynSpec {
                span,
                items: delim.inner,
            }))
        } else {
            Ok(None)
        }
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
                    v.typ.raw(),
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
                    v.typ.raw(),
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
    fn clustered_submodules() {
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
                    v.typ.raw(),
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
                    v.typ.raw(),
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

    #[test]
    fn dyn_submodules() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "submodules { parent: dyn P, child: C }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = SubmodulesStmt::parse(&buf).unwrap();
        assert_eq!(
            expr.items
                .iter()
                .cloned()
                .map(|v| (
                    v.ident.raw,
                    v.typ.raw(),
                    v.typ.is_dyn(),
                    v.cluster.map(|v| format!("{}", v.lit.kind))
                ))
                .collect::<Vec<_>>(),
            vec![
                ("parent".to_string(), "P".to_string(), true, None),
                ("child".to_string(), "C".to_string(), false, None)
            ]
        );
    }
}
