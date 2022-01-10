use crate::core::*;
use crate::net::*;

use lazy_static::__Deref;

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
    modules: Vec<Box<dyn Module>>,

    ///
    /// The set of channels used to connect module. This will NOT include direct connections
    /// which do not contain any delay, thus are bound to no channel.
    ///
    channels: Vec<Channel>,

    /// A buffer to store all gates
    gate_buffer: GateBuffer,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    ///
    /// Creates a new instance by wrapping 'inner' into a empty NetworkRuntime<A>.
    ///
    pub fn new(inner: A) -> Self {
        Self {
            modules: Vec::new(),
            channels: vec![],

            gate_buffer: GateBuffer::new(),

            inner,
        }
    }

    ///
    /// Registers a boxed module and adds it to the module set.
    /// Returns a mutable refernce to the boxed module.
    /// This reference should be short lived since it blocks any other reference to self.
    ///
    pub fn create_module(&mut self, module: Box<dyn Module>) -> &mut Box<dyn Module> {
        let insert_at = match self.modules.binary_search_by_key(&module.id(), |m| m.id()) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.modules.insert(insert_at, module);
        &mut self.modules[insert_at]
    }

    ///
    /// Retrieves a module that staisfies the given predicate.
    /// Shortcircuits once such a element is found.
    ///
    pub fn module<P>(&self, predicate: P) -> Option<&dyn Module>
    where
        P: FnMut(&&Box<dyn Module>) -> bool,
    {
        self.modules
            .iter()
            .find(predicate)
            .map(|boxed| boxed.deref())
    }

    ///
    /// Retrieves module by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_by_id(&self, module_id: ModuleId) -> Option<&dyn Module> {
        let pos = match self.modules.binary_search_by_key(&module_id, |m| m.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(self.modules[pos].deref())
    }

    ///
    /// Retrieves a module mutably that staisfies the given predicate.
    /// Shortcircuits once such a element is found.
    ///
    pub fn module_mut<P>(&mut self, predicate: P) -> Option<&mut Box<dyn Module>>
    where
        P: FnMut(&&mut Box<dyn Module>) -> bool,
    {
        self.modules.iter_mut().find(predicate)
    }

    ///
    /// Retrieves module mutably by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_mut_by_id(&mut self, module_id: ModuleId) -> Option<&mut Box<dyn Module>> {
        let pos = match self.modules.binary_search_by_key(&module_id, |m| m.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.modules[pos])
    }

    ///
    /// Registers a channel with a non-null delay.
    ///
    pub fn create_channel(&mut self, metrics: ChannelMetrics) -> ChannelId {
        let channel = Channel::new(metrics);
        let insert_at = match self
            .channels
            .binary_search_by_key(&channel.id(), |c| c.id())
        {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.channels.insert(insert_at, channel);
        self.channels[insert_at].id()
    }

    ///
    /// Retrieves a channel by id.
    ///
    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        let pos = match self.channels.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&self.channels[pos])
    }

    ///
    /// Retrieves a channel by id mutabliy.
    ///
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        let pos = match self.channels.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.channels[pos])
    }

    pub fn create_gate(&mut self, gate: Gate) -> GateRef {
        self.gate_buffer.insert(gate)
    }

    ///
    /// Retrieves a gate by id from.
    /// This operations should only be done if absuloutly nessecary since it is
    /// expensive, bc gates are stored in their respecitve owner modules.
    ///
    pub fn gate(&self, id: GateId) -> Option<&Gate> {
        self.gate_buffer.gate(id)
    }

    ///
    /// Retrieves a target gate of a gate chain.
    ///
    pub fn gate_dest(&self, source_id: GateId) -> Option<&Gate> {
        self.gate_buffer.gate_dest(source_id)
    }

    ///
    /// Locks the buffer to that no new gates can be created-
    ///
    pub fn finish_building(&mut self) {
        // If feature 'static_gates' is active,
        // lock the buffer to activate preformance improvments.
        #[cfg(feature = "static_gates")]
        self.gate_buffer.lock()
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

    fn at_simulation_start(rt: &mut Runtime<Self>) {
        // Add inital event
        rt.add_event(NetEvents::SimStartNotif(SimStartNotif()), SimTime::now());
    }
}
