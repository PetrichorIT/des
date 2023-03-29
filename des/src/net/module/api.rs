use super::{with_mod_ctx, ModuleContext, ModuleId, ModuleRef, ModuleReferencingError, SETUP_FN};
use crate::{
    net::runtime::buf_schedule_shutdown,
    prelude::{GateRef, ObjectPath},
    time::{Duration, SimTime},
};

/// Overwrite the setup fn all modules run.
///
/// All modules require common functionality based on the baseline setup of the
/// simulation. Such common functionality is usually provided by plugins,
/// but manually creating them on each module type is bothersome, and errorprone.
/// To cirumvent that, a common setup function is provided that initalizes some plugins
/// on all modules.
///
/// When feature 'net' is enabled, but feature 'async' is not, this function is a NOP.
/// With feature 'async' enabled, this function will initalize a `TokioTimePlugin`
/// on async modules.
///
/// When users override this function, they must ensure that the `TokioTimePlugin`
/// will still be initalized within the new setup fn, should the simulation require
/// async time-primitives.
///
/// # Example
///
/// ```rust
/// use des::net::module::{set_setup_fn, ModuleContext};
/// use des::net::plugin::*;
///
/// struct MyDebugPlugin;
/// impl Plugin for MyDebugPlugin {
///     /* ... */
/// }
///
/// fn setup(this: &ModuleContext) {
///     this.add_plugin(
///         TokioTimePlugin::new(this.path().as_str().to_string()),
///         0,
///         PluginPanicPolicy::Abort,
///     );
///     this.add_plugin(
///         MyDebugPlugin,
///         10,
///         PluginPanicPolicy::Abort,
///     );
/// }
///
/// fn main() {
///     # return;
///     set_setup_fn(setup);
///     /* ... */
/// }
/// ```
pub fn set_setup_fn(f: fn(&ModuleContext)) {
    *SETUP_FN.write() = f;
}

/// Returns a runtime-unqiue identifier for the currently active module.
///
/// This function should only be used within the context of a module.
/// Note that outside of the context of a module, this function may provide
/// invalid module-ids or module-ids of modules that are no longer valid.
///
/// # Example
///
/// ```
/// use des::prelude::*;
///
/// struct MyModule;
/// impl Module for MyModule {
///     fn new() -> Self { Self }
///     fn handle_message(&mut self, msg: Message) {
///         let id = module_id();
///         assert_eq!(id, msg.header().receiver_module_id);    
///     }
/// }
/// ```
///
/// [`Module`]: crate::net::module::Module
#[must_use]
pub fn module_id() -> ModuleId {
    with_mod_ctx(|ctx| ctx.id())
}

/// Returns a runtime-unqiue identifier for the currently active module,
/// based on its place in the module graph.
///
/// This function should only be used within the context of a module.
/// Note that outside of the context of a module, this function may provide
/// invalid module-paths or module-paths of modules that are no longer valid.
///
/// ```
/// use des::prelude::*;
///
/// struct MyModule;
/// impl Module for MyModule {
///     fn new() -> Self { Self }
///     fn handle_message(&mut self, msg: Message) {
///         let path = module_path();
///         println!("[{path}] recveived message: {}", msg.str())  
///     }
/// }
/// ```
///
/// [`Module`]: crate::net::module::Module
#[must_use]
pub fn module_path() -> ObjectPath {
    with_mod_ctx(|ctx| ctx.path())
}

/// Returns the name for the currently active module.
///
/// Note that the module name is just the last component of the module
/// path.
///
/// This function should only be used within the context of a module.
/// Note that outside of the context of a module, this function may provide
/// invalid module-names or module-names of modules that are no longer valid.
///
#[must_use]
pub fn module_name() -> String {
    with_mod_ctx(|ctx| ctx.name())
}

// PARENT CHILD

/// Returns a handle parent module in the module graph.
///
/// # Errors
///
/// Returns an error if the module has no parent.
///
pub fn parent() -> Result<ModuleRef, ModuleReferencingError> {
    with_mod_ctx(|ctx| ctx.parent())
}

/// Returns a handle to the child element, with the provided module name.
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
/// Returns a unstructured list of all gates from the current module.
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
///     let rt = Runtime::new(app).run();
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
