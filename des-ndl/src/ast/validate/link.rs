use super::*;
use crate::ast::{LinkData, LinkInheritance, LinkStmt, LitKind, Spanned};

impl Validate for LinkStmt {
    fn validate(&self, errors: &mut ErrorsMut) {
        if let Some(ref inh) = self.inheritance {
            inh.validate(errors)
        }
        self.data.validate(errors);
    }
}

impl Validate for LinkInheritance {
    fn validate(&self, errors: &mut ErrorsMut) {
        let mut symbols = Vec::with_capacity(self.symbols.len());
        for symbol in self.symbols.iter() {
            if symbols.contains(&&symbol.raw) {
                errors.add(
                    Error::new(
                        ErrorKind::LinkInheritanceDuplicatedSymbols,
                        format!(
                            "found duplicated symbol '{}' in link inheritance statement",
                            symbol.raw
                        ),
                    )
                    .spanned(self.span()),
                );
                continue;
            }
            symbols.push(&symbol.raw);
        }
    }
}

impl Validate for LinkData {
    fn validate(&self, errors: &mut ErrorsMut) {
        for item in self.items.iter() {
            // Do not make generic to Key<Ident, Lit, _> since known values
            // are only known in linkdata context
            match item.key.raw.as_str() {
                "jitter" => {
                    if !matches!(item.value.kind, LitKind::Float { .. }) {
                        errors.add(
                            Error::new(
                                ErrorKind::LinkKnownKeysInvalidValue,
                                "known key 'jitter' expects a value of type float",
                            )
                            .spanned(item.span()),
                        );
                    }
                }
                "latency" => {
                    if !matches!(item.value.kind, LitKind::Float { .. }) {
                        errors.add(
                            Error::new(
                                ErrorKind::LinkKnownKeysInvalidValue,
                                "known key 'latency' expects a value of type float ",
                            )
                            .spanned(item.span()),
                        );
                    }
                }
                "bitrate" => {
                    if !matches!(item.value.kind, LitKind::Integer { .. }) {
                        errors.add(
                            Error::new(
                                ErrorKind::LinkKnownKeysInvalidValue,
                                "known key 'bitrate' expects a value of type interger",
                            )
                            .spanned(item.span()),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{Parse, ParseBuffer, TokenStream},
        SourceMap,
    };

    use super::*;

    fn load_link(smap: &mut SourceMap, asset: &str, s: &str) -> LinkStmt {
        let asset = smap.load_raw(asset, s);
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);
        LinkStmt::parse(&buf).unwrap()
    }

    #[test]
    fn inheritance_dup() {
        let mut smap = SourceMap::new();

        // # Case 0
        let link = load_link(&mut smap, "raw:case0", "link A: B + C + D + B {}");
        let mut errors = Errors::new().as_mut();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::LinkInheritanceDuplicatedSymbols
        );

        // # Case 1
        let link = load_link(&mut smap, "raw:case1", "link A: SomeLink + SomeLink {}");
        let mut errors = Errors::new().as_mut();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::LinkInheritanceDuplicatedSymbols
        );
    }

    #[test]
    fn known_values_invalid_typ() {
        let mut smap = SourceMap::new();

        // # Case 0
        let link = load_link(&mut smap, "raw:case0", "link A { jitter: 100 }");
        let mut errors = Errors::new().as_mut();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::LinkKnownKeysInvalidValue
        );

        // # Case 1
        let link = load_link(&mut smap, "raw:case1", "link A { bitrate: 1.0 }");
        let mut errors = Errors::new().as_mut();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::LinkKnownKeysInvalidValue
        );

        // # Case 1
        let link = load_link(
            &mut smap,
            "raw:case1",
            "link A { latency: \"str\", bitrate: 1.0 }",
        );
        let mut errors = Errors::new().as_mut();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors.get(0).unwrap().kind,
            ErrorKind::LinkKnownKeysInvalidValue
        );
    }
}
