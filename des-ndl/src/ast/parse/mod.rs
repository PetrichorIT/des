mod buffer;
mod cursor;

pub use crate::error::*;
pub use buffer::*;

pub trait Parse: Sized {
    fn parse(input: ParseStream<'_>) -> Result<Self>;
}
