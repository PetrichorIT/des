use super::Globals;
use std::sync::Arc;

/// Returns the globals of the runtime.
///
/// > *This function should only be called within the simulation*
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`Sim`](super::Sim) exists.
///
#[must_use]
pub fn globals() -> Arc<Globals> {
    Globals::current()
}
