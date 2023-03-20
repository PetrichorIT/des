use super::Par;
use crate::net::module::module_path;
use std::marker::PhantomData;

///
/// Returns a parameter by reference (not parsed).
///
#[must_use]
pub fn par(key: impl AsRef<str>) -> Par {
    Par {
        key: format!("{}.{}", module_path(), key.as_ref()),
        value: None,
        _phantom: PhantomData,
    }
}

///
/// Returns a parameter by reference (not parsed).
///
#[must_use]
pub fn par_for(key: impl AsRef<str>, module: impl AsRef<str>) -> Par {
    Par {
        key: format!("{}.{}", module.as_ref(), key.as_ref()),
        value: None,
        _phantom: PhantomData,
    }
}
