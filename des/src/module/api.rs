use std::sync::Arc;

use super::{ModuleContext, try_with_mod_ctx};

/// Retuns a handle to the context of the current module. This
/// handle can be used on inspect and change the modules simulation
/// properties, independent of the modules processing elements.
///
/// > *This function requires a node-context within the simulation*
///
/// # Example
///
/// ```
/// # use des::prelude::*;
///
/// struct MyModule;
/// impl Module for MyModule {
///     fn handle_message(&mut self, msg: Message) {
///         todo!();
///     }
/// }
/// ```
///
/// # Panics
///
/// This function will panic if not called within a modules context.
#[must_use]
#[track_caller]
pub fn current() -> Arc<ModuleContext> {
    try_with_mod_ctx(Arc::clone)
        .expect("cannot retrieve current module context, no module currently in scope")
}

/// Retuns a handle to the context of the current module if some exists.
///
/// See [`current`].
#[must_use]
pub fn try_current() -> Option<Arc<ModuleContext>> {
    try_with_mod_ctx(Arc::clone)
}
