use super::super::parse::*;
use super::{IncludeToken, Semi, Slash};
use crate::{Ident, Joined};

#[derive(Debug)]
pub struct IncludeStmt {
    pub include: IncludeToken,
    pub path: Joined<Ident, Slash>,
    pub semi: Semi,
}

impl Joined<Ident, Slash> {
    pub fn path(&self) -> String {
        self.iter()
            .map(|v| &v.raw[..])
            .collect::<Vec<_>>()
            .join("/")
    }
}

// # Parsing

impl Parse for IncludeStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let include = IncludeToken::parse(input)?;
        let path = Joined::<Ident, Slash>::parse(input)?;
        let semi = Semi::parse(input)?;
        Ok(Self {
            include,
            path,
            semi,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{IncludeStmt, Parse, ParseBuffer};
    use crate::{SourceMap, Span, TokenStream};

    #[test]
    fn success_single_path_component() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "include abcde;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(0, 7));
        assert_eq!(include.semi.span, Span::new(13, 1));

        assert_eq!(include.path.len(), 1);
        assert_eq!(include.path.path(), "abcde");

        // # Case 1
        let offset = 14;
        let asset = smap.load_raw("raw:case1", "include _abc1321231_123_acd;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));
        assert_eq!(include.semi.span, Span::new(offset + 27, 1));

        assert_eq!(include.path.len(), 1);
        assert_eq!(include.path.path(), "_abc1321231_123_acd");

        // # Case 2
        let offset = 42;
        let asset = smap.load_raw("raw:case2", "include cdc;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));
        assert_eq!(include.path.len(), 1);
        assert_eq!(include.path.path(), "cdc");

        // # Case 3
        let offset = 54;
        let asset = smap.load_raw("raw:case3", "include \n\t\t// AB\n     cdc;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));

        assert_eq!(include.path.len(), 1);
        assert_eq!(include.path.path(), "cdc");
    }

    #[test]
    fn success_more_path_components() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "include a/b/c;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(0, 7));
        assert_eq!(include.semi.span, Span::new(13, 1));

        assert_eq!(include.path.len(), 3);
        assert_eq!(include.path.path(), "a/b/c");

        // # Case 1
        let offset = 14;
        let asset = smap.load_raw("raw:case1", "include a12312/b12312/_c;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));
        assert_eq!(include.semi.span, Span::new(offset + 24, 1));

        assert_eq!(include.path.len(), 3);
        assert_eq!(include.path.path(), "a12312/b12312/_c");

        // # Case 2
        let offset = 39;
        let asset = smap.load_raw("raw:case2", "include a12312/b12312/_c;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));

        assert_eq!(include.path.len(), 3);
        assert_eq!(include.path.path(), "a12312/b12312/_c");
    }
}
