use crate::{
    ast::{parse::*, DelimSpan, Delimiter, TokenTree},
    error::*,
    Span,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delimited<T> {
    pub delim: Delimiter,
    pub delim_span: DelimSpan,
    pub inner: T,
}

impl<T> Spanned for Delimited<T> {
    fn span(&self) -> Span {
        Span::fromto(self.delim_span.open, self.delim_span.close)
    }
}

impl<T: Parse> Delimited<T> {
    pub fn parse_from(delim: Delimiter, input: ParseStream<'_>) -> Result<Delimited<T>> {
        let Some(peek) = input.ts.peek() else {
            return Err(Error::new(ErrorKind::ExpectedDelimited, "expected delimited sequence"));
        };

        let (span, d) = match peek {
            TokenTree::Delimited(span, d, _) => (span, d),
            TokenTree::Token(token, _) => {
                return Err(Error::new(
                    ErrorKind::ExpectedDelimited,
                    format!(
                        "expected delimited sequence, found {}",
                        token.kind.token_kind_err_output()
                    ),
                )
                .spanned(peek.span()));
            }
        };

        if *d == delim {
            let substream = input.substream().unwrap();
            input.ts.bump();
            Ok(Self {
                delim: *d,
                delim_span: *span,
                inner: T::parse(&substream)?,
            })
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedDelim,
                "expected other delimited sequence",
            )
            .spanned(Span::fromto(span.open, span.close)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{Ident, Lit, TokenStream},
        resource::SourceMap,
    };

    #[test]
    fn success_single_token_delimited() {
        // used to test all kind of delimiters

        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "{ ident }");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = Delimited::<Ident>::parse_from(Delimiter::Brace, &buf).unwrap();
        assert_eq!(item.delim, Delimiter::Brace);
        assert_eq!(item.inner, "ident");

        // # Case 1
        let asset = smap.load_raw("raw:case1", "(ident)");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = Delimited::<Ident>::parse_from(Delimiter::Parenthesis, &buf).unwrap();
        assert_eq!(item.delim, Delimiter::Parenthesis);
        assert_eq!(item.inner, "ident");

        // # Case 2
        let asset = smap.load_raw("raw:case2", "[ident]");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = Delimited::<Ident>::parse_from(Delimiter::Bracket, &buf).unwrap();
        assert_eq!(item.delim, Delimiter::Bracket);
        assert_eq!(item.inner, "ident");

        // # Case 2
        let asset = smap.load_raw("raw:case2", "[123]");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = Delimited::<Lit>::parse_from(Delimiter::Bracket, &buf).unwrap();
        assert_eq!(item.delim, Delimiter::Bracket);
        assert_eq!(format!("{}", item.inner.kind), "123");
    }
}
