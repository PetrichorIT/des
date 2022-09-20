use std::any::Any;

use crate::net::Message;

mod ctx;
pub use self::ctx::*;

mod mref;
pub use mref::*;

mod error;
pub use error::*;

cfg_async! {
    mod async_mod;
    pub use self::async_mod::*;
}

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ModuleId(u16) = MODULE_ID;
);

///
/// A set of user defined functions for customizing the
/// behaviour of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait Module: Any {
    /// Creates a new instance of Self.
    fn new() -> Self
    where
        Self: Sized;

    /// Resets the custom state when a module is restarted.
    fn reset(&mut self) {}

    ///
    /// A message handler for receiving events, user defined.
    ///
    /// # Example
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// #[NdlModule]
    /// struct MyModule {
    ///     my_prop_1: f64,
    ///     my_prop_2: String,
    /// };
    ///
    /// impl Module for MyModule {
    /// # fn new() -> Self { todo!() }
    ///     /* ... */    
    ///
    ///     fn handle_message(&mut self, msg: Message) {
    ///         println!("Received {:?}", msg);
    ///     }
    /// }
    /// ```
    ///
    fn handle_message(&mut self, _msg: Message) {}

    ///
    /// A function that is run at the start of each simulation,
    /// for each module. The order in which modules are called is not guranteed
    /// but the stage numbers are. That means that all stage-0 calls for all modules
    /// happen before the first (if any) stage-1 calls. Generaly speaking, all stage-i
    /// calls finish before the first stage-i+1 call.
    ///
    /// # Example
    ///
    /// ```
    /// use des::prelude::*;
    /// # type Config = ();
    /// # type Record = u8;
    /// # fn fetch_config(s: &str, id: ModuleId) -> Config {}
    ///
    /// #[NdlModule]
    /// struct SomeModule {
    ///     config: Config,
    ///     records: Vec<Record>,
    /// };
    ///
    /// impl Module for SomeModule {
    /// # fn new() -> Self { todo!() }
    ///     /* ... */
    ///     
    ///     fn at_sim_start(&mut self, _stage: usize) {
    ///         self.config = fetch_config("https://mysimconfig.com/simrun1", module_id());
    ///         self.records.clear();
    ///     }
    ///
    ///     fn handle_message(&mut self, msg: Message) {
    ///         todo!()
    ///     }
    /// }
    /// ```
    ///
    fn at_sim_start(&mut self, _stage: usize) {}

    ///
    /// A function that is called when all `sim_start` stages of all modules
    /// are done. Used to resolve all async `sim_start_stages`.
    ///
    #[cfg(feature = "async")]
    fn finish_sim_start(&mut self) {}

    ///
    /// The number of stages used for the module initalization.
    ///
    fn num_sim_start_stages(&self) -> usize {
        1
    }

    ///
    /// A callback function that is invoked should the simulation finish.
    /// All events emitted by this function will NOT be processed.
    ///
    fn at_sim_end(&mut self) {}

    ///
    /// A function that is called when all `sim_end` stages of all modules
    /// are done. Used to resolve all async `sim_end_stages`.
    ///
    #[cfg(feature = "async")]
    fn finish_sim_end(&mut self) {}

    ///
    /// A callback function that is called should a parameter belonging to
    /// this module be changed.
    ///
    fn handle_par_change(&mut self) {}
}
