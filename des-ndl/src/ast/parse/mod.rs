mod buffer;
mod cursor;

use crate::error::*;
use crate::Span;

pub use buffer::*;

pub trait Parse: Sized {
    fn parse(input: ParseStream<'_>) -> Result<Self>;
}

pub trait Spanned {
    fn span(&self) -> Span;
}
