use super::super::parse::*;
use super::{IncludeToken, Semi, Slash};
use crate::Ident;

#[derive(Debug)]
pub struct IncludeStmt {
    pub include: IncludeToken,
    pub path: IncludePath,
    pub semi: Semi,
}

#[derive(Debug)]
pub struct IncludePath {
    pub ident: Ident,
    pub next: Option<(Slash, Box<IncludePath>)>,
}

impl IncludePath {
    pub fn path(&self) -> String {
        let next = self.next.as_ref().map(|v| v.1.path());
        if let Some(next) = next {
            format!("{}/{}", self.ident.raw, next)
        } else {
            self.ident.raw.to_string()
        }
    }

    pub fn path_len(&self) -> usize {
        1 + self.next.as_ref().map(|(_, n)| n.path_len()).unwrap_or(0)
    }
}

// # Parsing

impl Parse for IncludeStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let include = IncludeToken::parse(input)?;
        let path = IncludePath::parse(input)?;
        let semi = Semi::parse(input)?;
        Ok(Self {
            include,
            path,
            semi,
        })
    }
}

impl Parse for IncludePath {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident = Ident::parse(input)?;
        match Slash::parse(input) {
            Ok(slash) => Ok(IncludePath {
                ident,
                next: Some((slash, Box::new(IncludePath::parse(input)?))),
            }),
            Err(_) => Ok(IncludePath { ident, next: None }),
        }
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

        assert_eq!(include.path.path_len(), 1);
        assert_eq!(include.path.path(), "abcde");

        // # Case 1
        let offset = 14;
        let asset = smap.load_raw("raw:case1", "include _abc1321231_123_acd;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));
        assert_eq!(include.semi.span, Span::new(offset + 27, 1));

        assert_eq!(include.path.path_len(), 1);
        assert_eq!(include.path.path(), "_abc1321231_123_acd");

        // # Case 2
        let offset = 42;
        let asset = smap.load_raw("raw:case2", "include cdc;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));
        assert_eq!(include.path.path_len(), 1);
        assert_eq!(include.path.path(), "cdc");

        // # Case 3
        let offset = 54;
        let asset = smap.load_raw("raw:case3", "include \n\t\t// AB\n     cdc;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));

        assert_eq!(include.path.path_len(), 1);
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

        assert_eq!(include.path.path_len(), 3);
        assert_eq!(include.path.path(), "a/b/c");

        // # Case 1
        let offset = 14;
        let asset = smap.load_raw("raw:case1", "include a12312/b12312/_c;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));
        assert_eq!(include.semi.span, Span::new(offset + 24, 1));

        assert_eq!(include.path.path_len(), 3);
        assert_eq!(include.path.path(), "a12312/b12312/_c");

        // # Case 2
        let offset = 39;
        let asset = smap.load_raw("raw:case2", "include a12312/b12312/_c;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let include = IncludeStmt::parse(&buf).unwrap();
        assert_eq!(include.include.span, Span::new(offset, 7));

        assert_eq!(include.path.path_len(), 3);
        assert_eq!(include.path.path(), "a12312/b12312/_c");
    }
}
