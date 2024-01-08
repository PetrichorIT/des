use super::Par;
use crate::net::module::current;

///
/// Returns a parameter by reference (not parsed).
///
#[must_use]
pub fn par(key: impl AsRef<str>) -> Par {
    Par::new(key.as_ref(), current().path().as_str())
}

///
/// Returns a parameter by reference (not parsed).
///
#[must_use]
pub fn par_for(key: impl AsRef<str>, module: impl AsRef<str>) -> Par {
    Par::new(key.as_ref(), module.as_ref())
}
