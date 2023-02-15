use super::*;
use crate::ast::{File, Item};

impl Validate for File {
    fn validate(&self, errors: &mut ErrorsMut) {
        let mut symbols = Vec::with_capacity(self.items.len());
        for item in self.items.iter() {
            // (0) Internal validation
            item.validate(errors);

            // (1) Symbol duplication
            if let Some(symbol) = item.symbol() {
                if symbols.contains(&&symbol.raw) {
                    errors.add(Error::new(
                        ErrorKind::SymbolDuplication,
                        format!(
                            "cannot create new symbol '{}', was allready defined",
                            symbol.raw
                        ),
                    ))
                } else {
                    symbols.push(&symbol.raw)
                }
            }
        }
    }
}

impl Validate for Item {
    fn validate(&self, errors: &mut ErrorsMut) {
        match self {
            Self::Entry(entry) => entry.validate(errors),
            Self::Include(include) => include.validate(errors),
            Self::Link(link) => link.validate(errors),
            Self::Module(module) => module.validate(errors),
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

    #[test]
    fn symbol_dupliaction() {
        let mut smap = SourceMap::new();

        let asset = smap.load_raw("raw:case0", "module A {} module B {} module A {}");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let stmt = File::parse(&buf).unwrap();
        let mut errors = Errors::new().as_mut();
        stmt.validate(&mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors.get(0).unwrap().kind, ErrorKind::SymbolDuplication);
    }
}
