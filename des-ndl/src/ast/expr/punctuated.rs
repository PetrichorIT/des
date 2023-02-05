use crate::ast::parse::*;

// Eg <Lit, Comma>,

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Punctuated<T, P> {
    inner: Vec<(T, P)>,
    last: Option<Box<T>>,
}

impl<T, P> Punctuated<T, P> {
    pub const fn new() -> Self {
        Self {
            inner: Vec::new(),
            last: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.inner.len() + if self.last.is_some() { 1 } else { 0 }
    }

    pub fn first(&self) -> Option<&T> {
        self.inner.first().map(|v| &v.0)
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.inner.first_mut().map(|v| &mut v.0)
    }

    pub fn trailing_punct(&self) -> bool {
        self.last.is_none()
    }

    pub fn push_value(&mut self, value: T) {
        assert!(self.last.is_none());
        self.last = Some(Box::new(value));
    }

    pub fn push_punct(&mut self, punct: P) {
        assert!(self.last.is_some());
        self.inner.push((*self.last.take().unwrap(), punct))
    }

    pub fn iter(&self) -> Iter<'_, T, P> {
        Iter {
            punct: self,
            idx: 0,
        }
    }
}

impl<T: Parse, P: Parse> Parse for Punctuated<T, P> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut this = Self::new();
        while !input.ts.is_empty() {
            let item = T::parse(input)?;

            if input.ts.is_empty() {
                // no tailing delim needed
                this.last = Some(Box::new(item));
                break;
            } else {
                let delim = P::parse(input)?;
                this.inner.push((item, delim))
            }
        }

        assert!(input.ts.is_empty());
        Ok(this)
    }
}

// # Iter

pub struct Iter<'a, T, P> {
    punct: &'a Punctuated<T, P>,
    idx: usize,
}

impl<'a, T, P> Iterator for Iter<'a, T, P> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering::*;
        match self.idx.cmp(&self.punct.inner.len()) {
            Less => {
                self.idx += 1;
                Some(&self.punct.inner[self.idx - 1].0)
            }
            Equal => {
                self.idx += 1;
                self.punct.last.as_ref().map(|v| &**v)
            }
            Greater => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Comma, Eq, Ident, KeyValueField, Lit, Parse, ParseBuffer, Punctuated, Semi, SourceMap,
        TokenStream,
    };

    #[test]
    fn success_single_token_patterns() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", "first,second,third,");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<Ident, Comma>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 3);
        assert_eq!(punct.trailing_punct(), true);

        assert_eq!(
            punct.iter().cloned().collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );

        // # Case 1
        let asset = smap.load_raw("raw:case1", "first,second,third");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<Ident, Comma>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 3);
        assert_eq!(punct.trailing_punct(), false);

        assert_eq!(
            punct.iter().cloned().collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );

        // # Case 2
        let asset = smap.load_raw("raw:case2", "first,");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<Ident, Comma>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 1);
        assert_eq!(punct.trailing_punct(), true);

        assert_eq!(punct.iter().cloned().collect::<Vec<_>>(), vec!["first"]);

        // # Case 3
        let asset = smap.load_raw("raw:case3", "first");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<Ident, Comma>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 1);
        assert_eq!(punct.trailing_punct(), false);

        assert_eq!(punct.iter().cloned().collect::<Vec<_>>(), vec!["first"]);
    }

    #[test]
    fn success_multi_token_patterns() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw(
            "raw:case0",
            "
        ident = 123;
        key = 123123123;
        comma = 13;
        ",
        );
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<KeyValueField<Ident, Lit, Eq>, Semi>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 3);
        assert_eq!(punct.trailing_punct(), true);

        assert_eq!(
            punct
                .iter()
                .cloned()
                .map(|v| (format!("{}", v.key.raw), format!("{}", v.value.kind), "="))
                .collect::<Vec<_>>(),
            vec![
                ("ident".to_string(), "123".to_string(), "="),
                ("key".to_string(), "123123123".to_string(), "="),
                ("comma".to_string(), "13".to_string(), "="),
            ]
        );
    }

    #[test]
    fn success_pattern_delim_same_type() {
        let mut smap = SourceMap::new();

        // # Case 0
        let asset = smap.load_raw("raw:case0", ",,,,,,");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<Comma, Comma>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 3);
        assert_eq!(punct.trailing_punct(), true);

        // # Case 1
        let asset = smap.load_raw("raw:case1", ",,,,,");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let punct = Punctuated::<Comma, Comma>::parse(&buf).unwrap();
        assert_eq!(punct.len(), 3);
        assert_eq!(punct.trailing_punct(), false);
    }
}
