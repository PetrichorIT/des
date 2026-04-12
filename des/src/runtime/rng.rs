use std::cell::UnsafeCell;

use rand::{
    Rng, RngCore,
    distr::{Distribution, StandardUniform},
};

use crate::macros::support::SyncWrap;

pub(crate) static RNG: SyncWrap<UnsafeCell<Option<Box<dyn RngCore>>>> =
    SyncWrap::new(UnsafeCell::new(None));

pub(super) fn set_rng(rng: Box<dyn RngCore>) {
    *unsafe { &mut *RNG.get() } = Some(rng);
}

///
/// Returns a reference to a given rng.
///
/// # Panics
///
/// This function will panic if the RNG has not been initalized.
/// This will be done once the `Runtime` was created.
///
#[must_use]
#[track_caller]
pub fn rng() -> &'static mut dyn RngCore {
    unsafe { &mut *RNG.get() }
        .as_mut()
        .expect("RNG not yet initalized")
}

///
/// Generates a random instance of type T with a Standard distribution.
///
#[must_use]
#[track_caller]
pub fn random<T>() -> T
where
    StandardUniform: Distribution<T>,
{
    rng().random::<T>()
}

///
/// Generates a random instance of type T with a distribution
/// of type D.
///
#[must_use]
#[track_caller]
pub fn sample<T, D>(distr: D) -> T
where
    D: Distribution<T>,
{
    rng().sample::<T, D>(distr)
}
