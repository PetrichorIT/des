use std::sync::Arc;

use crate::{net::runtime::buf_fail, runtime::LikeRuntimeError};

use super::Globals;

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

/// Fail the current simulation with the given error.
///
/// > *This function should only be called within the simulation*
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`Sim`](super::Sim) exists.
///
pub fn fail(e: impl LikeRuntimeError) {
    buf_fail(e);
}
