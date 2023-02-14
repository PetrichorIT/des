use std::marker::PhantomData;

pub struct IterIter<I, I2, T> {
    iter: I,
    subiter: Option<I2>,
    phantom: PhantomData<(I2, T)>,
}

impl<I, I2, T> IterIter<I, I2, T>
where
    I: Iterator<Item = I2>,
    I2: Iterator<Item = T>,
{
    pub fn new(mut iter: I) -> Self {
        let subiter = iter.next();
        Self {
            iter,
            subiter,
            phantom: PhantomData,
        }
    }
}

impl<I, I2, T> Iterator for IterIter<I, I2, T>
where
    I: Iterator<Item = I2>,
    I2: Iterator<Item = T>,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(subiter) = &mut self.subiter {
            if let Some(item) = subiter.next() {
                Some(item)
            } else {
                self.subiter = self.iter.next();
                self.next()
            }
        } else {
            None
        }
    }
}
