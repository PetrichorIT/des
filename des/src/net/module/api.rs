use std::sync::Arc;

use super::{try_with_mod_ctx, ModuleContext};

/// Retuns a handle to the context of the current module. This
/// handle can be used on inspect and change the modules simulation
/// properties, independent of the modules processing elements.
///
/// > *This function requires a node-context within the simulation*
///
/// **This handle is only fully valid, during the execution of the current event,
/// thus is should never be stored.**
///
/// # Example
///
/// ```
/// # use des::prelude::*;
///
/// struct MyModule;
/// impl Module for MyModule {
///     fn handle_message(&mut self, msg: Message) {
///         let id = current().id();
///         if id == msg.header().sender_module_id {
///             println!("Self message received");
///         }
///     }
/// }
/// ```
///
/// # Panics
///
/// This function will panic if not called within a modules context.
#[must_use]
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
