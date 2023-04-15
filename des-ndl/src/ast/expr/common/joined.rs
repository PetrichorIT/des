use crate::{ast::parse::*, error::Result, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Joined<T, P> {
    items: Vec<(T, P)>,
    last: Box<T>,
}

impl<T, P> Joined<T, P> {
    pub fn len(&self) -> usize {
        self.items.len() + 1
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn iter(&self) -> JoinedIter<'_, T, P> {
        JoinedIter {
            joined: self,
            idx: 0,
        }
    }
}

pub struct JoinedIter<'a, T, P> {
    joined: &'a Joined<T, P>,
    idx: usize,
}

impl<'a, T, P> Iterator for JoinedIter<'a, T, P> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        use std::cmp::Ordering::*;
        match self.idx.cmp(&self.joined.items.len()) {
            Less => {
                self.idx += 1;
                Some(&self.joined.items[self.idx - 1].0)
            }
            Equal => {
                self.idx += 1;
                Some(&self.joined.last)
            }
            Greater => None,
        }
    }
}

impl<T, P> Spanned for Joined<T, P>
where
    T: Spanned,
{
    fn span(&self) -> crate::Span {
        Span::fromto(
            self.items
                .first()
                .map(|i| i.0.span())
                .unwrap_or(self.last.span()),
            self.last.span(),
        )
    }
}

impl<T, P> Parse for Joined<T, P>
where
    T: Parse,
    P: Parse,
{
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut items = Vec::new();
        loop {
            let item = T::parse(input).map_err(|e| {
                let f = format!("{}", e.internal);
                e.override_internal(format!("failed to parse value in joined statement: {f}"))
            })?;
            match P::parse(input) {
                Ok(delim) => items.push((item, delim)),
                Err(_) => {
                    return Ok(Self {
                        items,
                        last: Box::new(item),
                    });
                }
            }
        }
    }
}
