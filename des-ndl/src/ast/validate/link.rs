use super::*;
use crate::ast::{LinkData, LinkInheritance, LinkStmt, LitKind};

impl Validate for LinkStmt {
    fn validate(&self, errors: &mut LinkedList<Error>) {
        self.inheritance.as_ref().map(|inh| inh.validate(errors));
        self.data.validate(errors);
    }
}

impl Validate for LinkInheritance {
    fn validate(&self, errors: &mut LinkedList<Error>) {
        let mut symbols = Vec::with_capacity(self.symbols.len());
        for symbol in self.symbols.iter() {
            if symbols.contains(&&symbol.raw) {
                errors.push_back(Error::new(
                    ErrorKind::LinkInheritanceDuplicatedSymbols,
                    format!(
                        "found duplicated symbol '{}' in link inheritence statement",
                        symbol.raw
                    ),
                ));
                continue;
            }
            symbols.push(&symbol.raw);
        }
    }
}

impl Validate for LinkData {
    fn validate(&self, errors: &mut LinkedList<Error>) {
        for item in self.items.iter() {
            // Do not make generic to Key<Ident, Lit, _> since known values
            // are only known in linkdata context
            match item.key.raw.as_str() {
                "jitter" => {
                    if !matches!(item.value.kind, LitKind::Float { .. }) {
                        errors.push_back(Error::new(
                            ErrorKind::LinkKnownKeysInvalidValue,
                            format!(
                                "known key 'jitter' expectes a value of type float (not {})",
                                item.value.kind
                            ),
                        ));
                    }
                }
                "latency" => {
                    if !matches!(item.value.kind, LitKind::Float { .. }) {
                        errors.push_back(Error::new(
                            ErrorKind::LinkKnownKeysInvalidValue,
                            format!(
                                "known key 'latency' expectes a value of type float (not {})",
                                item.value.kind
                            ),
                        ));
                    }
                }
                "bitrate" => {
                    if !matches!(item.value.kind, LitKind::Integer { .. }) {
                        errors.push_back(Error::new(
                            ErrorKind::LinkKnownKeysInvalidValue,
                            format!(
                                "known key 'birate' expectes a value of type interger (not {})",
                                item.value.kind
                            ),
                        ));
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
        let mut errors = LinkedList::new();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.front().unwrap().kind,
            ErrorKind::LinkInheritanceDuplicatedSymbols
        );

        // # Case 1
        let link = load_link(&mut smap, "raw:case1", "link A: SomeLink + SomeLink {}");
        let mut errors = LinkedList::new();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.front().unwrap().kind,
            ErrorKind::LinkInheritanceDuplicatedSymbols
        );
    }

    #[test]
    fn known_values_invalid_typ() {
        let mut smap = SourceMap::new();

        // # Case 0
        let link = load_link(&mut smap, "raw:case0", "link A { jitter: 100 }");
        let mut errors = LinkedList::new();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.front().unwrap().kind,
            ErrorKind::LinkKnownKeysInvalidValue
        );

        // # Case 1
        let link = load_link(&mut smap, "raw:case1", "link A { bitrate: 1.0 }");
        let mut errors = LinkedList::new();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.front().unwrap().kind,
            ErrorKind::LinkKnownKeysInvalidValue
        );

        // # Case 1
        let link = load_link(
            &mut smap,
            "raw:case1",
            "link A { latency: \"str\", bitrate: 1.0 }",
        );
        let mut errors = LinkedList::new();
        link.validate(&mut errors);

        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors.front().unwrap().kind,
            ErrorKind::LinkKnownKeysInvalidValue
        );
    }
}
