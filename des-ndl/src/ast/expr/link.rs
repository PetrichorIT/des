use super::{Colon, Comma, Delimited, KeyValueField, LinkToken, Plus, Punctuated};
use crate::ast::parse::Parse;
use crate::{Delimiter, Ident, Lit, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub link_token: LinkToken,
    pub ident: Ident,
    pub inheritance: Option<LinkInheritance>,
    pub data: LinkData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkInheritance {
    pub colon: Colon,
    pub symbols: LinkInheritanceSymbols,
}

#[derive(Debug, Clone, PartialEq)]

pub struct LinkInheritanceSymbols {
    inner: Vec<(Ident, Plus)>,
    last: Ident,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub items: Punctuated<KeyValueField<Ident, Lit, Colon>, Comma>,
    pub span: Span,
}

// # Impl

impl LinkInheritanceSymbols {
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            symbols: self,
            idx: 0,
        }
    }
}

pub struct Iter<'a> {
    symbols: &'a LinkInheritanceSymbols,
    idx: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Ident;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering::*;
        match self.idx.cmp(&self.symbols.inner.len()) {
            Less => {
                self.idx += 1;
                Some(&self.symbols.inner[self.idx - 1].0)
            }
            Equal => {
                self.idx += 1;
                Some(&self.symbols.last)
            }
            Greater => None,
        }
    }
}

// # Parsing

impl Parse for Link {
    fn parse(input: crate::ParseStream<'_>) -> crate::Result<Self> {
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
    fn parse(input: crate::ParseStream<'_>) -> crate::Result<Self> {
        let colon = match Colon::parse(input) {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        println!("{:#?}", input.ts.peek());
        let symbols = LinkInheritanceSymbols::parse(input).unwrap();
        Ok(Some(LinkInheritance { colon, symbols }))
    }
}

impl Parse for LinkInheritanceSymbols {
    fn parse(input: crate::ParseStream<'_>) -> crate::Result<Self> {
        let mut items = Vec::new();
        loop {
            let item = Ident::parse(input)?;

            match Plus::parse(input) {
                Ok(delim) => items.push((item, delim)),
                Err(_) => {
                    return Ok(Self {
                        inner: items,
                        last: item,
                    });
                }
            }
        }
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

        let expr = Link::parse(&buf).unwrap();
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

        let expr = Link::parse(&buf).unwrap();
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
