use std::sync::Arc;

use crate::{
    net::runtime::NetEvents,
    prelude::{RuntimeError, current},
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
    current().globals()
}

/// Reports an error that will fail the current simulation. This will
/// NOT terminate the simulation, but rather report the error at the end
/// of the simulation.
///
/// > *This function should only be called within the simulation*
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`Sim`](super::Sim) exists.
///
pub fn report(e: impl LikeRuntimeError) {
    current().exec().report_error(Box::new(e));
}

/// Fail the current simulation with the given error. This will terminate
/// the simulation after the current event has finished executing.
///
/// > *This function should only be called within the simulation*
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`Sim`](super::Sim) exists.
///
pub fn fail(e: impl LikeRuntimeError) {
    current().exec().report_failure(RuntimeError::new(vec![e]));
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
    current().exec().schedule_event(event, time);
}
