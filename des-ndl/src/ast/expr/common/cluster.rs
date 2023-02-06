use std::fmt;

use super::{Delimited, Lit};
use crate::{ast::parse::*, Delimiter, Span, TokenTree};

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterDefinition {
    pub span: Span,
    pub lit: Lit,
}

impl fmt::Display for ClusterDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.lit.kind)
    }
}

impl Parse for Option<ClusterDefinition> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let Some(peek) = input.ts.peek() else {
            return Ok(None);
        };
        let TokenTree::Delimited(_, delim, _) = peek else {
            return Ok(None);
        };
        if *delim == Delimiter::Bracket {
            Ok(Some(ClusterDefinition::parse(input)?))
        } else {
            Ok(None)
        }
    }
}

impl Parse for ClusterDefinition {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let def = Delimited::<Lit>::parse_from(Delimiter::Bracket, input)?;
        Ok(ClusterDefinition {
            lit: def.inner,
            span: Span::fromto(def.delim_span.open, def.delim_span.close),
        })
    }
}
