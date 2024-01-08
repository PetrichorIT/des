use std::sync::Arc;

use super::{with_mod_ctx, ModuleContext, SETUP_FN};
use crate::{
    net::runtime::buf_schedule_shutdown,
    time::{Duration, SimTime},
};

/// Overwrite the setup fn all modules run.
///
/// All modules require common functionality based on the baseline setup of the
/// simulation. Such common functionality is usually provided by plugins,
/// but manually creating them on each module type is bothersome, and errorprone.
/// To cirumvent that, a common setup function is provided that initalizes some plugins
/// on all modules.
pub fn set_setup_fn(f: fn(&ModuleContext)) {
    *SETUP_FN.write() = f;
}

/// Retuns a handle to the context of the current module. This
/// handle can be used on inspect and change the modules simulation 
/// properties, independent of the modules processing elements.
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
///     fn new() -> Self { Self }
///     fn handle_message(&mut self, msg: Message) {
///         let id = current().id();
///         if id == msg.header().sender_module_id {
///             println!("Self message received");
///         }
///     }
/// }
/// ```
#[must_use]
pub fn current() -> Arc<ModuleContext> {
    with_mod_ctx(Arc::clone)
}


// BUF CTX based

/// Shuts down all activity for the module.
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


///
/// Shuts down all activity for the module.
/// Restarts after the given duration.
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
///     fn new() -> Self {
///         Self {
///             volatile: 0,
///             persistent: 0,
///         }
///     }
///
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
/// #    NetworkApplication::new(());
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

///
/// Shuts down all activity for the module.
/// Restarts at the given time.
///
/// The user must ensure that the restart time
/// point is greater or equal to the current simtime.
///
/// See [`shutdow_and_restart_in`] for more information.
pub fn shutdow_and_restart_at(restart_at: SimTime) {
    buf_schedule_shutdown(Some(restart_at));
}
