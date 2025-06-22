//! Network nodes with custom state.
//!
//! A module represents an independent computation and communication entity within as simulation, usually any host or
//! other network appliance. It is responsible for managing all internal state of this entity. Nodes may communicate with
//! other nodes via the network abstractions, or export information to the simulation runtime directly, using
//! dedicated simulation APIs.
//!
//! The definitions in this module contain the core abstractions for creating and managing modules and their
//! lifecycle. Abstractions for sending and receiving messages can be found in the [`message` module](crate::net::message).
//!
//! # Custom Implementations using the `Module` trait
//!
//! All nodes are created by providing an object, that implements the [`Module` trait]. This trait provides a set of
//! methods that define the behaviour of the module in response to events and messages. Implementation details of this
//! object are up to the user. This trait consists mainly of the following methods:
//!
//! - `at_sim_start`: A method that will be called whenever the node is started
//! - `at_sim_end`: A method that will be called whenever the simulation is ending
//! - `handle_message`: A method that is called whenever a message is received by the module
//! - `reset`: A message that can reset the module, after a crash or restart.
//!
//! Additionally this trait defines the method `stack` that can modify the plugin stack,
//! provided to the module. This is an advanced feature, that allows modules to inject custom plugins
//! will still respecting the global plugin stack. See [`processing` module](crate::net::processing) for
//! more information on plugins and other advanced processing features.
//!
//! > Note that APIs like [`SimBuilder::node`](crate::net::runtime::SimBuilder::node) require a object of [trait `ModuleBlock`](crate::net::blocks::ModuleBlock). However
//! > all implementors of [`Module`] also implement [`ModuleBlock`](crate::net::blocks::ModuleBlock).
//!
//! # Common features via the `ModuleContext`
//!
//! Modules are a combination of a user-provided implementation (using [`Module`]) and
//! a simulation internal component, the [`ModuleContext`]. The module context
//! represents the topology, properties and hierarchical layout of the module within the simulation context and provides
//! APIs to interact and modify the connecting fabric, the module tree and the property set.
//!
//! A modules (simulation-)context can be access either via a [`ModuleRef`] aquired from
//! various simulation APIs or more commonly the global function [`current`] that returns
//! a handle to the module context of the currently active module.
//!
//! A module context is usually created automatically when using the [`Sim`](crate::net::runtime::Sim) builder,
//! but manual constructors are available for advanced use cases.
//!
//! > The term `within node-context` refers to the presence of a [`ModuleContext`]
//! > in the global scope, accessable via the [`current`] function.
//!
//! # Exposing values using properties
//!
//! Properties are key-value pair attached to modules, that are used to expose internal values to outside observers like
//! other modules, or a simulation-GUI. These values can be set by the module itself using the [`ModuleContext::prop` method]
//! or provided by a configuration files for inital parameter dissemination.
//!
//! # Using Tokio for async-await
//!
//! If the crate-feature `async` is active, all module come with a current-thread tokio runtime. This runtime is already
//! active on any calls to the [`Module`] API, so call to `task::spawn` will be possible. Consider the internal implementation
//! to be like that:
//!
//! ```text
//! let rt = ...;
//! rt.enter(|| {
//!     module.sim_api_fn();
//! });
//! ```
//!
//! Using the [`join`](ModuleContext::join) function, you can schedule a tokio task to be joined once the simulation ends.
//! If that is not possible, an error will be returned from the simulation run.

use crate::{net::message::Message, prelude::RuntimeError};
use std::{
    any::Any,
    fmt,
    sync::atomic::{AtomicU16, Ordering},
};

mod api;
mod ctx;
mod dummy;
mod error;
mod refs;

#[cfg(test)]
mod tests;

pub(crate) use self::ctx::*;
pub use self::ctx::{ModuleContext, Stereotyp};
pub use api::*;
pub(crate) use dummy::*;
pub use error::*;
pub use refs::*;

use super::processing::{ProcessingStack, Processor};
pub use des_net_utils::props::{Prop, PropType, RawProp};

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
    /// # Errors
    ///
    /// May return an error if the module deems the simulation has failed.
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        Ok(())
    }
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
