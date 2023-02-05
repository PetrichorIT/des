use super::{Colon, Comma, Delimited, KeyValueField, LinkToken, Punctuated};
use crate::ast::parse::Parse;
use crate::{Delimiter, Ident, Lit, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub link_token: LinkToken,
    pub ident: Ident,
    pub data: LinkData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub items: Punctuated<KeyValueField<Ident, Lit, Colon>, Comma>,
    pub span: Span,
}

impl Parse for Link {
    fn parse(input: crate::ParseStream<'_>) -> crate::Result<Self> {
        let link_token = LinkToken::parse(input)?;
        let ident = Ident::parse(input)?;
        let data = LinkData::parse(input)?;

        Ok(Self {
            link_token,
            ident,
            data,
        })
    }
}

impl Parse for LinkData {
    fn parse(input: crate::ParseStream<'_>) -> crate::Result<Self> {
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
    use crate::{Link, Parse, ParseBuffer, SourceMap, TokenStream};

    #[test]
    fn multiple_lit_parse() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "link FastLink { ident: 123, other: 1.0 }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = Link::parse(&buf).unwrap();
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

        let expr = Link::parse(&buf).unwrap();
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
}
