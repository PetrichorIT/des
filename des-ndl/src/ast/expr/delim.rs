use crate::ast::parse::*;
use crate::{DelimSpan, Delimiter, TokenTree};

#[derive(Debug, Clone)]
pub struct Delimited<T> {
    pub delim: Delimiter,
    pub delim_span: DelimSpan,
    pub inner: T,
}

impl<T: Parse> Delimited<T> {
    pub fn parse_from(delim: Delimiter, input: ParseStream<'_>) -> Result<Delimited<T>> {
        let Some(peek) = input.ts.peek() else {
            return Err(Error::new(ErrorKind::ExpectedDelimited, "expected delimited sequence"));
        };

        let TokenTree::Delimited(span, d, _) = peek else { 
            return Err(Error::new(ErrorKind::ExpectedDelimited, "expected delimited sequence"));
        };

        if *d == delim {
            let substream = input.substream().unwrap();
            input.ts.bump();
            Ok(Self {
                delim: *d,
                delim_span: *span,
                inner: T::parse(&substream)?,
            })
        } else {
            Err(Error::new(ErrorKind::UnexpectedDelim, "expected other delimited sequence"))
        }
    }
}

#[cfg(test)]
mod tests {
    
}