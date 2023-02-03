mod buffer;
pub use buffer::*;

mod error;
pub use error::*;

mod cursor;

pub trait Parse: Sized {
    fn parse(input: ParseStream<'_>) -> Result<Self>;
}
