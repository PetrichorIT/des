use std::collections::HashMap;

use super::{with_mod_ctx, ModuleId, ModuleRef, ModuleReferencingError};
use crate::{
    net::{common::Optional, globals, runtime::buf_schedule_shutdown, ParHandle},
    prelude::{GateRef, ObjectPath},
    time::{Duration, SimTime},
};

/// A runtime-unqiue identifier for this module-core and by extension this module.
#[must_use]
pub fn module_id() -> ModuleId {
    with_mod_ctx(|ctx| ctx.id())
}

/// A runtime-unqiue (not enforced) identifier for this module, based on its place in the module tree.
#[must_use]
pub fn module_path() -> ObjectPath {
    with_mod_ctx(|ctx| ctx.path())
}

/// Returns the name of the module instance.
#[must_use]
pub fn module_name() -> String {
    with_mod_ctx(|ctx| ctx.name())
}

// PARENT CHILD

/// Returns the parent element
///
/// # Errors
///
/// Returns an error if the module has no parent.
///
pub fn parent() -> Result<ModuleRef, ModuleReferencingError> {
    with_mod_ctx(|ctx| ctx.parent())
}

/// Returns the child element.
///
/// # Errors
///
/// Returns an error if no child was found under the given name.
///
pub fn child(name: &str) -> Result<ModuleRef, ModuleReferencingError> {
    with_mod_ctx(|ctx| ctx.child(name))
}

// GATE RELATED

///
/// Returns a ref unstructured list of all gates from the current module.
///
#[must_use]
pub fn gates() -> Vec<GateRef> {
    with_mod_ctx(|ctx| ctx.gates())
}

///
/// Returns a ref to a gate of the current module dependent on its name and cluster position
/// if possible.
///
#[must_use]
pub fn gate(name: &str, pos: usize) -> Option<GateRef> {
    with_mod_ctx(|ctx| ctx.gate(name, pos))
}

// BUF CTX based

///
/// Shuts down all activity for the module.
///
pub fn shutdown() {
    buf_schedule_shutdown(None);
}

///
/// Shuts down all activity for the module.
/// Restarts after the given duration.
///
pub fn shutdow_and_restart_in(dur: Duration) {
    self::shutdow_and_restart_at(SimTime::now() + dur);
}

///
/// Shuts down all activity for the module.
/// Restarts at the given time.
///
pub fn shutdow_and_restart_at(restart_at: SimTime) {
    buf_schedule_shutdown(Some(restart_at));
}

///
/// Returns the parameters for the current module.
///
#[must_use]
pub fn pars() -> HashMap<String, String> {
    let path = self::module_path();
    globals().parameters.get_def_table(path.path())
}

///
/// Returns a parameter by reference (not parsed).
///
#[must_use]
pub fn par(key: &str) -> ParHandle<Optional> {
    globals()
        .parameters
        .get_handle(self::module_path().path(), key)
}
