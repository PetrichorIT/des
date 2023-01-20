use std::panic::UnwindSafe;

pub(crate) struct UnwindSafeBox<T>(pub(crate) T);
impl<T> UnwindSafe for UnwindSafeBox<T> {}
