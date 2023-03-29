use super::module::ModuleRef;
use super::par::ParMap;
use super::Topology;
use crate::net::ObjectPath;
use crate::runtime::{Application, EventLifecycle, Runtime};
use log::info;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

mod events;
pub(crate) use events::*;

mod ctx;
pub use self::ctx::*;

///
/// A runtime application for a module/network oriantated simulation.
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct NetworkApplication<A> {
    ///
    /// The set of module used in the network simulation.
    /// All module must be boxed, since they must conform to the
    /// [`Module`](crate::net::module::Module) trait.
    ///
    module_list: Vec<ModuleRef>,

    ///
    /// The globals provided by the runtime
    /// that cannot be mutated by the users.
    ///
    globals: Arc<NetworkApplicationGlobals>,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkApplication<A> {
    ///
    /// Creates a new instance by wrapping 'inner' into a empty `NetworkApplication<A>`.
    ///
    #[must_use]
    pub fn new(inner: A) -> Self {
        let this = Self {
            module_list: Vec::new(),
            globals: Arc::new(NetworkApplicationGlobals::new()),

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
            Err(e) => eprintln!("Failed to load par file: {e}"),
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
    pub fn globals(&self) -> Arc<NetworkApplicationGlobals> {
        self.globals.clone()
    }
}

impl<A> Application for NetworkApplication<A>
where
    A: EventLifecycle<NetworkApplication<A>>,
{
    type EventSet = NetEvents;
    type Lifecycle = NetworkApplicationLifecycle<A>;
}

#[doc(hidden)]
#[derive(Debug)]
pub struct NetworkApplicationLifecycle<A: EventLifecycle<NetworkApplication<A>>> {
    _phantom: PhantomData<A>,
}

impl<A> EventLifecycle<NetworkApplication<A>> for NetworkApplicationLifecycle<A>
where
    A: EventLifecycle<NetworkApplication<A>>,
{
    fn at_sim_start(rt: &mut Runtime<NetworkApplication<A>>) {
        // Add inital event
        // this is done via an event to get the usual module buffer clearing behavoir
        // while the end ignores all send packets.

        // (0) Start the contained application for lifetime behaviour
        // - in case of NDL build topology with this call
        A::at_sim_start(rt);

        // (1) Initalize globals acoording to build network topology
        rt.app
            .globals
            .topology
            .lock()
            .unwrap()
            .build(&rt.app.module_list);

        // (2) Run network-node sim_starting stages
        // - inline this to ensure this is run before any possible events

        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        let max_stage = rt
            .app
            .modules()
            .iter()
            .fold(1, |acc, module| acc.max(module.num_sim_start_stages()));

        // (2.1) Call the stages in order, parallel over all modules
        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            for i in 0..rt.app.modules().len() {
                // Use cloned handles to appease the brwchk
                let module = rt.app.modules()[i].clone();
                log_scope!(module.ctx.logger_token);

                if stage < module.num_sim_start_stages() {
                    info!("Calling at_sim_start({}).", stage);

                    module.activate();
                    module.at_sim_start(stage);
                    module.deactivate();

                    super::buf_process(&module, rt);
                }
            }
        }

        // (2.2) Ensure all sim_start stages have finished, in an async context
        #[cfg(feature = "async")]
        {
            for i in 0..rt.app.modules().len() {
                let module = rt.app.modules()[i].clone();
                log_scope!(module.ctx.logger_token);

                module.activate();
                module.finish_sim_start();
                module.deactivate();

                super::buf_process(&module, rt);
            }
        }

        // (2.3) Reset the logging scope.
        log_scope!();
    }

    fn at_sim_end(rt: &mut Runtime<NetworkApplication<A>>) {
        for module in &mut rt.app.module_list {
            log_scope!(module.ctx.logger_token);
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
                log_scope!(module.ctx.logger_token);
                module.activate();
                module.finish_sim_end();
                module.deactivate();
            }
        }

        A::at_sim_end(rt);

        log_scope!();
    }
}

impl<A> Debug for NetworkApplication<A>
where
    A: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let modules = self
            .module_list
            .iter()
            .map(|m| &m.ctx.path)
            .collect::<Vec<&ObjectPath>>();

        f.debug_struct("NetworkApplication")
            .field("modules", &modules)
            .field("globals", &self.globals)
            .field("app", &self.inner)
            .finish()
    }
}

impl<A> Display for NetworkApplication<A>
where
    A: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

///
/// The global parameters about a [`NetworkApplication`] that are publicly
/// exposed.
///
#[derive(Debug)]
pub struct NetworkApplicationGlobals {
    ///
    /// The current state of the parameter tree, derived from *.par
    /// files and parameter changes at runtime.
    ///
    pub parameters: Arc<ParMap>,

    ///
    /// The topology of the network from a module viewpoint.
    ///
    pub topology: Mutex<Topology>,
}

impl NetworkApplicationGlobals {
    ///
    /// Creates a new instance of Self.
    ///
    #[must_use]
    pub fn new() -> Self {
        Self {
            parameters: Arc::new(ParMap::new()),
            topology: Mutex::new(Topology::new()),
        }
    }
}

impl Default for NetworkApplicationGlobals {
    fn default() -> Self {
        Self::new()
    }
}
