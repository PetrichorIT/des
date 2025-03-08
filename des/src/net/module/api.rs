use std::{future::Future, sync::Arc};

use super::{try_with_mod_ctx, ModuleContext};
use crate::{
    net::runtime::buf_schedule_shutdown,
    time::{Duration, SimTime},
};

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

// BUF CTX based

/// Shuts down all activity for the module.
///
/// > *This function requires a node-context within the simulation*
///
/// A module that is shut down, will not longer be able to
/// handle incoming messages, or run any user-defined code.
/// All plugin activity will be suspendend. However the
/// custom state will be kept for debug purposes.
///
/// This function must be used within a module context
/// otherwise its effects should be consider UB.
pub fn shutdown() {
    buf_schedule_shutdown(None);
}

/// Shuts down all activity for the module.
/// Restarts after the given duration.
///
/// > *This function requires a node-context within the simulation*
///
/// On restart the module will be reinitalized
/// using `Module::reset`  and then `Module::at_sim_start`.
/// Use the reset function to get the custom state to a resonable default
/// state, which may or may not be defined by `Module::new`.
/// However you can simulate persistent-beyond-shutdown data
/// by not reseting this data in `Module::reset`.
///
/// ```
/// # use des::prelude::*;
/// # type Data = usize;
/// struct MyModule {
///     volatile: Data,
///     persistent: Data,
/// }
///
/// impl Module for MyModule {
///     fn reset(&mut self) {
///         self.volatile = 0;
///     }
///
///     fn at_sim_start(&mut self, _: usize) {
///         println!(
///             "Start at {} with volatile := {} and persistent := {}",
///             SimTime::now(),
///             self.volatile,
///             self.persistent
///         );
///
///         self.volatile = 42;
///         self.persistent = 1024;
///
///         if SimTime::now() == SimTime::ZERO {
///             shutdow_and_restart_in(Duration::from_secs(10));
///         }
///     }
/// }
///
/// fn main() {
///     let app = /* ... */
/// #    Sim::new(());
///     let rt = Builder::new().build(app).run();
///     // outputs 'Start at 0s with volatile := 0 and persistent := 0'
///     // outputs 'Start at 10s with volatile := 0 and persistent := 1024'
/// }
/// ```
///
/// [`Module::new`]: crate::net::module::Module::new
/// [`Module::reset`]: crate::net::module::Module::reset
/// [`Module::at_sim_start`]: crate::net::module::Module::at_sim_start
pub fn shutdow_and_restart_in(dur: Duration) {
    buf_schedule_shutdown(Some(SimTime::now() + dur));
}

/// Shuts down all activity for the module.
/// Restarts at the given time.
///
/// > *This function requires a node-context within the simulation*
///
/// The user must ensure that the restart time
/// point is greater or equal to the current simtime.
///
/// See [`shutdow_and_restart_in`] for more information.
pub fn shutdow_and_restart_at(restart_at: SimTime) {
    buf_schedule_shutdown(Some(restart_at));
}

cfg_async! {

    /// Schedules a task to be joined when the simulatio ends
    ///
    /// This function will **not** block, but rather defer the joining
    /// to the simulation shutdown phase.
    pub fn join_spawn<F>(fut: F)
    where
        F: Future<Output = ()>,
        F: Send + 'static
    {
        current().async_ext.write().must_join.spawn(fut);
    }

    pub fn tryjoin_spawn<F>(fut: F)
    where
        F: Future<Output = ()>,
        F: Send + 'static
    {
        current().async_ext.write().try_join.spawn(fut);
    }

    pub(crate) fn reset_join_handles() {
        current().async_ext.write().must_join.detach_all();
        current().async_ext.write().try_join.detach_all();
    }
}
