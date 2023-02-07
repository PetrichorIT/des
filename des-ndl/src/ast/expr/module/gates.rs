use crate::ast::parse::*;
use crate::ast::{
    Annotation, ClusterDefinition, Comma, Delimited, Delimiter, GatesToken, Ident, Lit, Punctuated,
    TokenKind, TokenTree,
};
use crate::resource::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct GatesStmt {
    pub keyword: GatesToken,
    pub span: Span,
    pub items: Punctuated<GateDefinition, Comma>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GateDefinition {
    pub ident: Ident,
    pub cluster: Option<ClusterDefinition>,
    pub annotation: Option<Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GateClusterDefinition {
    pub span: Span,
    pub lit: Lit,
}

impl Parse for GatesStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let keyword = GatesToken::parse(input)?;
        let items =
            Delimited::<Punctuated<GateDefinition, Comma>>::parse_from(Delimiter::Brace, input)?;
        let span = Span::fromto(items.delim_span.open, items.delim_span.close);
        Ok(Self {
            keyword,
            span,
            items: items.inner,
        })
    }
}

impl Parse for GateDefinition {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident = Ident::parse(input)?;
        let cluster = Option::<ClusterDefinition>::parse(input)?;
        let annotation = Option::<Annotation>::parse(input)?;
        Ok(Self {
            ident,
            cluster,
            annotation,
        })
    }
}

impl Parse for Option<Annotation> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let Some(peek) = input.ts.peek() else {
            return Ok(None);
        };
        let TokenTree::Token(token, _) = peek else {
            return Ok(None);
        };
        if matches!(token.kind, TokenKind::Annotation(_)) {
            Ok(Some(Annotation::parse(input)?))
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
    fn simple_gates() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "gates { in, out, debug }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| (d.ident.raw, d.cluster, d.annotation))
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 3);
        assert_eq!(
            defs,
            vec![
                ("in".to_string(), None, None),
                ("out".to_string(), None, None),
                ("debug".to_string(), None, None)
            ]
        );

        // # Case 1
        let asset = smap.load_raw("raw:case1", "gates { __ident, _hid3, debug, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| (d.ident.raw, d.cluster, d.annotation))
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 3);
        assert_eq!(
            defs,
            vec![
                ("__ident".to_string(), None, None),
                ("_hid3".to_string(), None, None),
                ("debug".to_string(), None, None)
            ]
        );

        // # Case 2
        let asset = smap.load_raw("raw:case2", "gates { __ident, 123, debug, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _stmt = GatesStmt::parse(&buf).unwrap_err();
    }

    #[test]
    fn annotated_gates() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "gates { in @input, out @output, debug }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| (d.ident.raw, d.cluster, d.annotation.map(|v| v.raw)))
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 3);
        assert_eq!(
            defs,
            vec![
                ("in".to_string(), None, Some("input".to_string())),
                ("out".to_string(), None, Some("output".to_string())),
                ("debug".to_string(), None, None)
            ]
        );

        // # Case 1
        let asset = smap.load_raw("raw:case1", "gates { __ident, _hid3 @ident, debug @o123, }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| (d.ident.raw, d.cluster, d.annotation.map(|v| v.raw)))
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 3);
        assert_eq!(
            defs,
            vec![
                ("__ident".to_string(), None, None),
                ("_hid3".to_string(), None, Some("ident".to_string())),
                ("debug".to_string(), None, Some("o123".to_string()))
            ]
        );
    }

    #[test]
    fn clusted_gates() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "gates { in[5], out[0], debug }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| {
                (
                    d.ident.raw,
                    d.cluster.map(|v| format!("{}", v.lit.kind)),
                    d.annotation,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 3);
        assert_eq!(
            defs,
            vec![
                ("in".to_string(), Some("5".to_string()), None),
                ("out".to_string(), Some("0".to_string()), None),
                ("debug".to_string(), None, None)
            ]
        );

        // # Case 1
        let asset = smap.load_raw(
            "raw:case1",
            "gates { __ident[5], _hid3[1.0], debug[\"str\"], }",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| {
                (
                    d.ident.raw,
                    d.cluster.map(|v| format!("{}", v.lit.kind)),
                    d.annotation,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 3);
        assert_eq!(
            defs,
            vec![
                ("__ident".to_string(), Some("5".to_string()), None),
                ("_hid3".to_string(), Some("1.0".to_string()), None),
                ("debug".to_string(), Some("\"str\"".to_string()), None)
            ]
        );
    }

    #[test]
    fn full_gates() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "gates { in[6] @output }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = GatesStmt::parse(&buf).unwrap();
        let defs = stmt
            .items
            .iter()
            .cloned()
            .map(|d| {
                (
                    d.ident.raw,
                    d.cluster.map(|v| format!("{}", v.lit.kind)),
                    d.annotation.map(|v| v.raw),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(defs.len(), 1);
        assert_eq!(
            defs,
            vec![(
                "in".to_string(),
                Some("6".to_string()),
                Some("output".to_string())
            ),]
        );
    }
}
