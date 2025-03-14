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
    pub fn merge(&mut self, other: Self) {
        self.inner.extend(other.inner);
    }

    /// Extend
    pub fn extend<T, I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
        T: LikeRuntimeError + 'static,
    {
        self.inner.extend(
            iter.into_iter()
                .map(|e| Box::new(e) as Box<dyn LikeRuntimeError>),
        );
    }
}

impl Debug for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RuntimeErrors {:?}", self.inner)
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RuntimeErrors:")?;
        for err in &self.inner {
            writeln!(f, "- {err}")?;
        }
        Ok(())
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

/// Runtime rrro
pub trait LikeRuntimeError: StdError + Any {
    /// As any
    fn as_any(&self) -> &dyn Any;
}

impl<T: StdError + Any + 'static> LikeRuntimeError for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
