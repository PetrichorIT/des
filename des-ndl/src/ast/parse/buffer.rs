use std::{borrow::Borrow, sync::Arc};

use super::{cursor::Cursor, *};
use crate::{Asset, TokenStream};

pub type ParseStream<'a> = &'a ParseBuffer<'a>;

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
