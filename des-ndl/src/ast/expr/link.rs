use super::LinkToken;
use crate::ast::parse::Parse;
use crate::{DelimSpan, Delimiter, Ident};

pub struct Link {
    pub link_token: LinkToken,
    pub ident: Ident,
    pub delim: Delimiter,
    pub delim_span: DelimSpan,
}

impl Parse for Link {
    fn parse(input: crate::ParseStream<'_>) -> crate::Result<Self> {
        let link_token = LinkToken::parse(input)?;
        let ident = Ident::parse(input)?;

        todo!()
    }
}
