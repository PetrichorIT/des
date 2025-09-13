use std::sync::Arc;

use crate::{
    net::runtime::{NetEvents, buf_fail, buf_schedule_event},
    runtime::LikeRuntimeError,
    time::SimTime,
};

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

/// Schedule an event to be executed at the given time.
///
/// This is the lowest level primitive to interact with the simulation runtime.
/// Most public APIs like `schedule_at` or `send` are built on top of this function.
///
/// > *This function should only be called within the simulation*
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`Sim`](super::Sim) exists.
///
pub fn schedule_event(event: NetEvents, time: SimTime) {
    buf_schedule_event(event, time);
}
