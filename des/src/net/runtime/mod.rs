use super::common::Parameters;
use super::module::ModuleRef;
use crate::net::{ObjectPath, Topology};
use crate::runtime::{Application, Runtime};
use crate::time::SimTime;
use log::info;
use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::sync::Arc;

mod events;
pub(crate) use events::*;

mod ctx;
pub use self::ctx::*;

///
/// A runtime application for a module/network oriantated simulation.
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct NetworkRuntime<A> {
    ///
    /// The set of module used in the network simulation.
    /// All module must be boxed, since they must conform to the [`Module`] trait.
    ///
    module_list: Vec<ModuleRef>,

    ///
    /// The globals provided by the runtime
    /// that cannot be mutated by the users.
    ///
    globals: Arc<NetworkRuntimeGlobals>,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    ///
    /// Creates a new instance by wrapping 'inner' into a empty `NetworkRuntime<A>`.
    ///
    #[must_use]
    pub fn new(inner: A) -> Self {
        let this = Self {
            module_list: Vec::new(),
            globals: Arc::new(NetworkRuntimeGlobals::new()),

            inner,
        };

        // attack to current buffer
        buf_set_globals(Arc::downgrade(&this.globals));
        this
    }

    ///
    /// Tries to include a parameter file.
    ///
    pub fn include_par_file(&mut self, file: &str) {
        match std::fs::read_to_string(file) {
            Ok(string) => self.globals.parameters.build(&string),
            Err(e) => eprintln!("Failed to load par file: {}", e),
        }
    }

    ///
    /// Registers a boxed module and adds it to the module set.
    /// Returns a mutable refernce to the boxed module.
    /// This reference should be short lived since it blocks any other reference to self.
    ///
    pub fn create_module(&mut self, module: ModuleRef) {
        self.module_list.push(module);
    }

    ///
    /// Returns a reference to the list of all modules.
    ///
    #[must_use]
    pub fn modules(&self) -> &Vec<ModuleRef> {
        &self.module_list
    }

    ///
    /// Searches a module based on this predicate.
    /// Shortcircuits if found and returns a read-only reference.
    ///
    pub fn module<F>(&self, predicate: F) -> Option<ModuleRef>
    where
        F: FnMut(&&ModuleRef) -> bool,
    {
        self.modules().iter().find(predicate).cloned()
    }

    ///
    /// Drops all modules and channels and only returns the inner value.
    ///
    #[must_use]
    pub fn finish(self) -> A {
        self.inner
    }

    /// Returns the network runtime globals
    pub fn globals(&self) -> Arc<NetworkRuntimeGlobals> {
        self.globals.clone()
    }
}

impl<A> Application for NetworkRuntime<A> {
    type EventSet = NetEvents;

    fn at_sim_start(rt: &mut Runtime<Self>) {
        // Add inital event
        // this is done via an event to get the usual module buffer clearing behavoir
        // while the end ignores all send packets.

        rt.app
            .globals
            .topology
            .borrow_mut()
            .build(&rt.app.module_list);

        rt.add_event(NetEvents::SimStartNotif(SimStartNotif()), SimTime::now());
    }

    fn at_sim_end(rt: &mut Runtime<Self>) {
        for module in &mut rt.app.module_list {
            log_scope!(module.ctx.path.path());
            info!("Calling 'at_sim_end'");
            module.activate();
            module.at_sim_end();
            module.deactivate();

            // NOTE: no buf_process since no furthe events will be processed.
        }

        #[cfg(feature = "async")]
        {
            // Ensure all sim_start stages have finished
            for module in &mut rt.app.module_list {
                log_scope!(module.ctx.path.path());
                module.activate();
                module.finish_sim_end();
                module.deactivate();
            }
        }

        log_scope!();
    }
}

impl<A> Debug for NetworkRuntime<A>
where
    A: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let modules = self
            .module_list
            .iter()
            .map(|m| &m.ctx.path)
            .collect::<Vec<&ObjectPath>>();

        f.debug_struct("NetworkRuntime")
            .field("modules", &modules)
            .field("globals", &self.globals)
            .field("app", &self.inner)
            .finish()
    }
}

impl<A> Display for NetworkRuntime<A>
where
    A: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

///
/// The global parameters about a [`NetworkRuntime`] that are publicly
/// exposed.
///
#[derive(Debug)]
pub struct NetworkRuntimeGlobals {
    ///
    /// The current state of the parameter tree, derived from *.par
    /// files and parameter changes at runtime.
    ///
    pub parameters: Parameters,

    ///
    /// The topology of the network from a module viewpoint.
    ///
    pub topology: RefCell<Topology>,

    ///
    /// The total duration spend in the module specific handlers.
    ///
    #[cfg(feature = "metrics-module-time")]
    pub time_elapsed: std::time::Duration,
}

impl NetworkRuntimeGlobals {
    ///
    /// Creates a new instance of Self.
    ///
    #[must_use]
    pub fn new() -> Self {
        Self {
            parameters: Parameters::new(),
            topology: RefCell::new(Topology::new()),

            #[cfg(feature = "metrics-module-time")]
            time_elapsed: std::time::Duration::ZERO,
        }
    }
}

impl Default for NetworkRuntimeGlobals {
    fn default() -> Self {
        Self::new()
    }
}
