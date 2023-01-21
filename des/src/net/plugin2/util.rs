use std::panic::UnwindSafe;

#[repr(transparent)]
pub(crate) struct UnwindSafeBox<T>(pub(crate) T);
impl<T> UnwindSafe for UnwindSafeBox<T> {}
