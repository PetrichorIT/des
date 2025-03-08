use std::{any::Any, error::Error as StdError, fmt::Display};

/// An error that occurred during the simulation.
#[derive(Debug)]
pub struct RuntimeError {
    inner: Box<dyn StdErrorAny>,
}

impl RuntimeError {
    /// As any
    pub fn as_any(&self) -> &dyn Any {
        self.inner.as_any()
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: StdError + Any + 'static> From<T> for RuntimeError {
    fn from(err: T) -> Self {
        RuntimeError {
            inner: Box::new(err),
        }
    }
}

trait StdErrorAny: StdError + Any {
    fn as_any(&self) -> &dyn Any;
}
impl<T: StdError + Any + 'static> StdErrorAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
