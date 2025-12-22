use std::{
    any::Any,
    error::Error as StdError,
    fmt::{Debug, Display},
    ops::Deref,
};

/// An error that occurred during the simulation.
#[must_use]
pub struct RuntimeError {
    inner: Vec<Box<dyn LikeRuntimeError>>,
}

impl RuntimeError {
    /// Creates an empty `RuntimeError` object.
    pub const fn empty() -> Self {
        RuntimeError { inner: Vec::new() }
    }

    /// Creates a new `RuntimeError` instance.
    pub fn new(inner: Vec<impl LikeRuntimeError>) -> Self {
        RuntimeError {
            inner: inner
                .into_iter()
                .map(|v| Box::new(v) as Box<dyn LikeRuntimeError>)
                .collect(),
        }
    }

    /// Merge
    pub fn merge(&mut self, mut other: Self) {
        self.inner.append(&mut other.inner);
    }

    pub(crate) fn into_inner(self) -> Vec<Box<dyn LikeRuntimeError>> {
        self.inner
    }
}

impl<I: LikeRuntimeError + 'static> Extend<I> for RuntimeError {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.inner.extend(
            iter.into_iter()
                .map(|e| Box::new(e) as Box<dyn LikeRuntimeError>),
        );
    }
}

impl Extend<Box<dyn LikeRuntimeError>> for RuntimeError {
    fn extend<T: IntoIterator<Item = Box<dyn LikeRuntimeError>>>(&mut self, iter: T) {
        self.inner.extend(iter);
    }
}

impl Debug for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "simulation failed with some errors:")?;
        let n = self.inner.len();
        for (i, err) in self.inner.iter().enumerate() {
            let lines = err.to_string();
            for line in lines.lines() {
                writeln!(f, "{line}")?;
            }
            if i != n - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl Deref for RuntimeError {
    type Target = [Box<dyn LikeRuntimeError>];
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: StdError + Any + 'static> From<T> for RuntimeError {
    fn from(err: T) -> Self {
        RuntimeError {
            inner: vec![Box::new(err)],
        }
    }
}

/// Runtime error
pub trait LikeRuntimeError: StdError + Any {
    /// Just a helper function, you could archive the same result with trait upcasting
    fn as_any(&self) -> &dyn Any;
}

impl<T: StdError + Any + 'static> LikeRuntimeError for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
