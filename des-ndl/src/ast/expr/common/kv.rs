use crate::{ast::parse::*, error::Result, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValueField<K, V, D> {
    pub key: K,
    pub delim: D,
    pub value: V,
}

impl<K, V, D> Spanned for KeyValueField<K, V, D>
where
    K: Spanned,
    V: Spanned,
{
    fn span(&self) -> crate::Span {
        Span::fromto(self.key.span(), self.value.span())
    }
}

impl<K, V, D> Parse for KeyValueField<K, V, D>
where
    K: Parse,
    V: Parse,
    D: Parse,
{
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key = K::parse(input)?;
        let delim = D::parse(input).map_err(|e| {
            let f = format!("{}", e.internal);
            e.override_internal(format!("missing delimiter in key-value pair: {f}"))
        })?;
        let value = V::parse(input).map_err(|e| {
            let f = format!("{}", e.internal);
            e.override_internal(format!("missing value in key-value pair: {f}"))
        })?;

        Ok(Self { key, delim, value })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{parse::*, Eq, Ident, KeyValueField, Lit, LitKind, TokenStream};
    use crate::resource::SourceMap;

    #[test]
    fn success_single_delim_token() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "first = 123");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let kv = KeyValueField::<Ident, Lit, Eq>::parse(&buf).unwrap();
        assert_eq!(kv.key, "first");
        assert_eq!(kv.value.kind, LitKind::Integer { lit: 123 });

        // # Case 1
        let asset = smap.load_raw("raw:case1", "first = \"first\"");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let kv = KeyValueField::<Ident, Lit, Eq>::parse(&buf).unwrap();
        assert_eq!(kv.key, "first");
        assert_eq!(
            kv.value.kind,
            LitKind::Str {
                lit: "first".to_string()
            }
        );
    }
}
