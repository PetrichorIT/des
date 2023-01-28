use super::{Delimiter, Token};
use crate::Span;
use std::sync::Arc;

pub struct TokenStream {
    items: Arc<Vec<TokenTree>>,
}

pub enum TokenTree {
    Token(Token, Spacing),
    Delimited(DelimSpan, Delimiter, TokenStream),
}

pub enum Spacing {
    Alone,
    Joint,
}

pub struct DelimSpan {
    open: Span,
    close: Span,
}
