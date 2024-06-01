//! Network nodes with custom state.

use crate::net::message::Message;
use std::{
    any::Any,
    fmt,
    sync::atomic::{AtomicU16, Ordering},
};

mod ctx;
pub use self::ctx::ModuleContext;
pub(crate) use self::ctx::*;

mod reference;
pub use reference::*;

mod error;
pub use error::*;

mod api;
pub use api::*;

mod dummy;
pub(crate) use dummy::*;

mod meta;

#[cfg(test)]
mod tests;

use super::processing::{ProcessingStack, Processor};

cfg_async! {
    pub(super) mod rt;
}

/// A unique identifier for a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ModuleId(pub u16);

static MODULE_ID: AtomicU16 = AtomicU16::new(0xff);

impl ModuleId {
    /// A general purpose ID indicating None.
    pub const NULL: ModuleId = ModuleId(0);

    /// Generates a unique module ID.
    pub fn gen() -> Self {
        Self(MODULE_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

///
/// A set of user defined functions for customizing the
/// behaviour of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait Module: Any {
    /// Resets the custom state when a module is restarted.
    fn reset(&mut self) {
        #[cfg(feature = "tracing")]
        tracing::warn!("Module has been shutdown and restarted, but reset() was not defined. This may lead to invalid custom state.");
    }

    ///
    /// A function that assigns the processing stack that support this module.
    ///
    /// As input this function get the default processing stack. The default implemention
    /// just returns this stack unchanged. Users may choose to override or append
    /// the provided base stack.
    fn stack(&self, stack: ProcessingStack) -> ProcessingStack {
        stack
    }

    ///
    /// A message handler for receiving events, user defined.
    ///
    /// # Example
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// struct MyModule {
    ///     my_prop_1: f64,
    ///     my_prop_2: String,
    /// };
    ///
    /// impl Module for MyModule {
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
    /// struct SomeModule {
    ///     config: Config,
    ///     records: Vec<Record>,
    /// };
    ///
    /// impl Module for SomeModule {
    ///     /* ... */
    ///     
    ///     fn at_sim_start(&mut self, _stage: usize) {
    ///         self.config = fetch_config("https://mysimconfig.com/simrun1", current().id());
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
}

pub(crate) trait ModuleExt: Module {
    /// BUILD TODO: Remove
    fn to_processing_chain(self, stack: ProcessingStack) -> Processor
    where
        Self: Sized + 'static,
    {
        Processor::new(self.stack(stack), self)
    }
}

impl<T: Module> ModuleExt for T {}
