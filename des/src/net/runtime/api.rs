use std::panic::panic_any;
use std::sync::Arc;

use super::Globals;
use super::SimWideUnwind;
use super::Watcher;

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

/// Returns the watcher for the current module.
///
/// > *This function should only be called within the simulation*
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`Sim`](super::Sim) exists.
///
#[must_use]
pub fn watcher() -> Watcher {
    Watcher::current()
}

/// Panics the entire simulation, cirumventing the unwind catchers that catch module local
/// panics.
pub fn panic(s: impl Into<String>) -> ! {
    let s = s.into();
    eprintln!("{}", s);
    panic_any(SimWideUnwind(Box::new(s)))
}
