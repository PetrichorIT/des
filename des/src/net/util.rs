use std::{
    any::type_name,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NoDebug<T> {
    inner: T,
}

impl<T> NoDebug<T> {
    pub(crate) fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for NoDebug<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for NoDebug<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<T> for NoDebug<T> {
    fn from(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> Debug for NoDebug<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<T>()).finish()
    }
}
