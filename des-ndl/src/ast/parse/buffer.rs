use std::borrow::Borrow;

use super::{cursor::Cursor, *};
use crate::{ast::TokenStream, Asset};

pub type ParseStream<'a> = &'a ParseBuffer<'a>;

#[derive(Debug)]
pub struct ParseBuffer<'a> {
    pub asset: Asset<'a>,
    pub ts: Cursor,
}

impl<'a> ParseBuffer<'a> {
    pub fn new(asset: Asset<'a>, ts: impl Borrow<TokenStream>) -> Self {
        Self {
            asset,
            ts: Cursor::new(ts.borrow()),
        }
    }

    pub fn parse<T: Parse>(&self) -> Result<T> {
        T::parse(self)
    }

    pub fn call<T>(&self, f: fn(ParseStream<'_>) -> Result<T>) -> Result<T> {
        f(self)
    }

    pub fn substream(&self) -> Option<ParseBuffer<'a>> {
        let ts = self.ts.subcursor()?;
        Some(Self {
            ts,
            asset: self.asset,
        })
    }
}
