use crate::core::*;
use crate::net::*;
use crate::util::*;

mod events;
pub use events::*;
use lazy_static::__Deref;
use log::error;

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
    module_buffer: IdBuffer<Box<dyn Module>>,

    ///
    /// The set of parameters for the module-driven simulation.
    ///
    parameters: spmc::SpmcWriter<Parameters>,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    pub fn parameters(&self) -> spmc::SpmcReader<Parameters> {
        self.parameters.get_reader()
    }

    ///
    /// Creates a new instance by wrapping 'inner' into a empty NetworkRuntime<A>.
    ///
    pub fn new(inner: A) -> Self {
        Self {
            module_buffer: IdBuffer::new(),
            parameters: spmc::SpmcWriter::new(Parameters::new()),

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
    pub fn create_module(&mut self, module: Box<dyn Module>) -> &mut Box<dyn Module> {
        self.module_buffer.insert(module)
    }

    ///
    /// Returns a reference to the list of all modules.
    ///
    pub fn modules(&self) -> &Vec<Box<dyn Module>> {
        self.module_buffer.contents()
    }

    ///
    /// Searches a module based on this predicate.
    /// Shortcircuits if found and returns a read-only reference.
    ///
    pub fn module<F>(&self, predicate: F) -> Option<&dyn Module>
    where
        F: FnMut(&&Box<dyn Module>) -> bool,
    {
        self.modules()
            .iter()
            .find(predicate)
            .map(|boxed| boxed.deref())
    }
    ///
    /// Returns a mutable reference to the list of all modules.
    ///
    pub fn modules_mut(&mut self) -> &mut Vec<Box<dyn Module>> {
        self.module_buffer.contents_mut()
    }

    ///
    /// Searches a module based on this predicate.
    /// Shortcircuits if found and returns a mutably reference.
    ///
    pub fn module_mut<F>(&mut self, predicate: F) -> Option<&mut Box<dyn Module>>
    where
        F: FnMut(&&mut Box<dyn Module>) -> bool,
    {
        self.modules_mut().iter_mut().find(predicate)
    }

    ///
    /// Retrieves module by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_by_id(&self, id: ModuleId) -> Option<&dyn Module> {
        self.module_buffer.get(id).map(|boxed| boxed.deref())
    }

    ///
    /// Retrieves module mutably by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_mut_by_id(&mut self, id: ModuleId) -> Option<&mut Box<dyn Module>> {
        self.module_buffer.get_mut(id)
    }

    ///
    /// Registers a channel with a non-null delay.
    ///
    pub fn create_channel(&mut self, metrics: ChannelMetrics) -> ChannelRef {
        // TODO: Depc
        Mrc::new(Channel::new(metrics))
    }

    ///
    /// Locks the buffer to that no new gates can be created-
    ///
    pub fn finish_building(&mut self) {
        // If feature 'static' is active,
        // lock the buffer to activate preformance improvments
        #[cfg(feature = "net-static")]
        {
            self.module_buffer.lock();
        }
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
        rt.add_event(NetEvents::SimStartNotif(SimStartNotif()), SimTime::now());
    }

    fn at_sim_end(rt: &mut Runtime<Self>) {
        for module in rt.app.module_buffer.iter_mut() {
            module.at_sim_end();
        }
    }
}
