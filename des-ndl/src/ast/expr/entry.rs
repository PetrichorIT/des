use super::{EntryToken, Ident, Semi};
use crate::ast::parse::*;

#[derive(Debug, Clone, PartialEq)]
pub struct EntryStmt {
    pub entry: EntryToken,
    pub symbol: Ident,
    pub semi: Semi,
}

impl Parse for EntryStmt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let entry = EntryToken::parse(input)?;
        let symbol = Ident::parse(input)?;
        let semi = Semi::parse(input)?;
        Ok(Self {
            entry,
            symbol,
            semi,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::TokenStream, resource::SourceMap};

    #[test]
    fn parse_entry_statement() {
        let mut smap = SourceMap::new();

        // Case #0
        let asset = smap.load_raw("raw:case0", "entry Main;");
        let ts = TokenStream::new(asset).unwrap();
        let buf = ParseBuffer::new(asset, ts);

        let expr = EntryStmt::parse(&buf).unwrap();
        assert_eq!(expr.symbol, "Main")
    }
}
