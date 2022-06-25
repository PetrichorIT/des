use super::common::Parameters;
use crate::net::*;
use crate::runtime::*;
use crate::time::*;
use crate::util::*;
use log::error;
use log::info;
use std::{
    fmt::{Debug, Display},
    marker::Unsize,
};

mod events;
pub use events::*;

///
/// A runtime application for a module/network oriantated simulation.
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct NetworkRuntime<A> {
    ///
    /// The set of module used in the network simulation.
    /// All module must be boxed, since they must conform to the [Module] trait.
    ///
    module_list: Vec<PtrMut<dyn Module>>,

    ///
    /// The globals provided by the runtime
    /// that cannot be mutated by the users.
    ///
    globals: PtrMut<NetworkRuntimeGlobals>,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    ///
    /// Returns the globals (readonly) of the entire simulation.
    ///
    #[doc(hidden)]
    pub fn globals_weak(&self) -> PtrWeakConst<NetworkRuntimeGlobals> {
        PtrWeak::from_strong(&self.globals).make_const()
    }

    ///
    /// Returns the globals (readonly) of the entire simulation.
    ///
    pub fn globals(&self) -> PtrConst<NetworkRuntimeGlobals> {
        Ptr::clone(&self.globals).make_const()
    }

    ///
    /// Creates a new instance by wrapping 'inner' into a empty NetworkRuntime<A>.
    ///
    pub fn new(inner: A) -> Self {
        Self {
            module_list: Vec::new(),
            globals: PtrMut::new(NetworkRuntimeGlobals::new()),

            inner,
        }
    }

    ///
    /// Tries to include a parameter file.
    ///
    pub fn include_par_file(&mut self, file: &str) {
        match std::fs::read_to_string(file) {
            Ok(string) => self.globals.parameters.build(&string),
            Err(e) => error!(target: "ParLoader", "{}", e),
        }
    }

    ///
    /// Registers a boxed module and adds it to the module set.
    /// Returns a mutable refernce to the boxed module.
    /// This reference should be short lived since it blocks any other reference to self.
    ///
    pub fn create_module<T>(&mut self, module: PtrMut<T>)
    where
        T: Module + Unsize<dyn Module>,
    {
        let dyned: PtrMut<dyn Module> = module;
        self.module_list.push(dyned);
    }

    ///
    /// Returns a reference to the list of all modules.
    ///
    pub fn modules(&self) -> &Vec<PtrMut<dyn Module>> {
        &self.module_list
    }

    ///
    /// Searches a module based on this predicate.
    /// Shortcircuits if found and returns a read-only reference.
    ///
    pub fn module<F>(&self, predicate: F) -> Option<PtrMut<dyn Module>>
    where
        F: FnMut(&&PtrMut<dyn Module>) -> bool,
    {
        self.modules().iter().find(predicate).cloned()
    }

    ///
    /// Drops all modules and channels and only returns the inner value.
    ///
    pub fn finish(self) -> A {
        self.inner
    }
}

impl<A> Application for NetworkRuntime<A> {
    type EventSet = NetEvents;

    fn at_sim_start(rt: &mut Runtime<Self>) {
        // Add inital event
        // this is done via an event to get the usual module buffer clearing behavoir
        // while the end ignores all send packets.

        rt.app.globals.topology.build(&rt.app.module_list);

        rt.add_event(NetEvents::SimStartNotif(SimStartNotif()), SimTime::now());
    }

    fn at_sim_end(rt: &mut Runtime<Self>) {
        for module in rt.app.module_list.iter_mut() {
            module.at_sim_end();
            info!(
                target: &format!("Module: {}", module.str()),
                "Calling at_sim_end."
            );
        }
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
            .map(|m| m.path())
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
/// The global parameters about a [NetworkRuntime] that are publicly
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
    pub topology: Topology,
}

impl NetworkRuntimeGlobals {
    ///
    /// Creates a new instance of Self.
    ///
    pub fn new() -> Self {
        Self {
            parameters: Parameters::new(),
            topology: Topology::new(),
        }
    }
}

impl Default for NetworkRuntimeGlobals {
    fn default() -> Self {
        Self::new()
    }
}
