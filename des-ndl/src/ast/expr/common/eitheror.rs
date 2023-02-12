use crate::{ast::parse::*, error::Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EitherOr<E, O> {
    Either(E),
    Or(O),
}

impl<E, O> Spanned for EitherOr<E, O>
where
    E: Spanned,
    O: Spanned,
{
    fn span(&self) -> crate::Span {
        match self {
            EitherOr::Either(either) => either.span(),
            EitherOr::Or(or) => or.span(),
        }
    }
}

impl<E, O> Parse for EitherOr<E, O>
where
    E: Parse,
    O: Parse,
{
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let old = input.ts.state();
        match E::parse(input) {
            Ok(either) => return Ok(EitherOr::Either(either)),
            Err(_) => {
                input.ts.set_state(old);
                let or = O::parse(input)?;
                Ok(EitherOr::Or(or))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Comma, Ident, Joined, Lit, Slash, TokenStream};
    use crate::SourceMap;

    use super::*;

    #[test]
    fn single_token() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "ident");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = EitherOr::<Ident, Lit>::parse(&buf).unwrap();
        assert!(matches!(item, EitherOr::Either(_)));
        assert!(buf.ts.is_empty());

        // # Case 1
        let asset = smap.load_raw("raw:case1", "123");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = EitherOr::<Ident, Lit>::parse(&buf).unwrap();
        assert!(matches!(item, EitherOr::Or(_)));
        assert!(buf.ts.is_empty());

        // # Case 2
        let asset = smap.load_raw("raw:case2", "@annot");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let _item = EitherOr::<Ident, Lit>::parse(&buf).unwrap_err();
    }

    #[test]
    fn multiple_token_reset() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "ident/subident");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = EitherOr::<Joined<Ident, Slash>, Lit>::parse(&buf).unwrap();
        assert!(matches!(item, EitherOr::Either(_)));
        assert!(buf.ts.is_empty(), "remaining token: {:#?}..", buf.ts.peek());

        // # Case 0
        let asset = smap.load_raw("raw:case0", "123");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = EitherOr::<Joined<Ident, Slash>, Lit>::parse(&buf).unwrap();
        assert!(matches!(item, EitherOr::Or(_)));
        assert!(buf.ts.is_empty(), "remaining token: {:#?}..", buf.ts.peek());

        // # Case 1
        let asset = smap.load_raw("raw:case1", "ident/subident");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = EitherOr::<Joined<Ident, Slash>, Lit>::parse(&buf).unwrap();
        assert!(matches!(item, EitherOr::Either(_)));
        assert!(buf.ts.is_empty(), "remaining token: {:#?}..", buf.ts.peek());

        // # Case 2
        let asset = smap.load_raw("raw:case2", "123/+");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = EitherOr::<Joined<Lit, Slash>, Lit>::parse(&buf).unwrap();
        assert!(matches!(item, EitherOr::Or(_)));
        assert!(!buf.ts.is_empty());
    }

    #[test]
    fn joined() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "ident, odent, 123, mudent");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let item = Joined::<EitherOr<Ident, Lit>, Comma>::parse(&buf).unwrap();
        // assert!(matches!(item, EitherOr::Either(_)));
        assert_eq!(item.len(), 4);
        assert!(buf.ts.is_empty(), "remaining token: {:#?}..", buf.ts.peek());
    }
}
