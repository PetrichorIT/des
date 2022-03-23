use std::{
    fmt::{Debug, Display},
    marker::Unsize,
};

use crate::core::*;
use crate::net::*;
use crate::util::{mm::*, spmc::*};

mod events;
pub use events::*;
use log::error;
use log::info;

use super::common::Parameters;

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
    module_list: Vec<ModuleRef>,

    ///
    /// The set of parameters for the module-driven simulation.
    ///
    parameters: SpmcWriter<Parameters>,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    ///
    /// Returns the parameter reader of the entire simulation.
    ///
    pub fn parameters(&self) -> SpmcReader<Parameters> {
        self.parameters.get_reader()
    }

    ///
    /// Creates a new instance by wrapping 'inner' into a empty NetworkRuntime<A>.
    ///
    pub fn new(inner: A) -> Self {
        Self {
            module_list: Vec::new(),
            parameters: SpmcWriter::new(Parameters::new()),

            inner,
        }
    }

    ///
    /// Tries to include a parameter file.
    ///
    pub fn include_par_file(&mut self, file: &str) {
        match std::fs::read_to_string(file) {
            Ok(string) => self.parameters.build(&string),
            Err(e) => error!(target: "ParLoader", "{}", e),
        }
    }

    ///
    /// Registers a boxed module and adds it to the module set.
    /// Returns a mutable refernce to the boxed module.
    /// This reference should be short lived since it blocks any other reference to self.
    ///
    pub fn create_module<T>(&mut self, module: Mrc<T>)
    where
        T: Module + Unsize<dyn Module>,
    {
        let dyned: Mrc<dyn Module> = module;
        self.module_list.push(dyned);
    }

    ///
    /// Returns a reference to the list of all modules.
    ///
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
            .collect::<Vec<&ModulePath>>();

        f.debug_struct("NetworkRuntime")
            .field("modules", &modules)
            .field("parameters", &self.parameters)
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
