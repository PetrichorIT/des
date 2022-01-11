use crate::core::*;
use crate::net::*;

mod events;
pub use events::*;
use lazy_static::__Deref;

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
    module_buffer: ModuleBuffer,

    ///
    /// The set of channels used to connect module. This will NOT include direct connections
    /// which do not contain any delay, thus are bound to no channel.
    ///
    channel_buffer: ChannelBuffer,

    ///
    ///  A buffer to store all gates.
    ///
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
            module_buffer: ModuleBuffer::new(),
            channel_buffer: ChannelBuffer::new(),
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
        self.module_buffer.insert(module)
    }

    ///
    /// Returns a reference to the list of all modules.
    ///
    pub fn modules(&self) -> &Vec<Box<dyn Module>> {
        self.module_buffer.modules()
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
        self.module_buffer.modules_mut()
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
        self.module_buffer.module(id)
    }

    ///
    /// Retrieves module mutably by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_mut_by_id(&mut self, id: ModuleId) -> Option<&mut Box<dyn Module>> {
        self.module_buffer.module_mut(id)
    }

    ///
    /// Registers a channel with a non-null delay.
    ///
    pub fn create_channel(&mut self, metrics: ChannelMetrics) -> ChannelId {
        let channel = Channel::new(metrics);
        self.channel_buffer.insert(channel)
    }

    ///
    /// Retrieves a channel by id.
    ///
    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channel_buffer.channel(id)
    }

    ///
    /// Retrieves a channel by id mutabliy.
    ///
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channel_buffer.channel_mut(id)
    }

    ///
    /// Registers a new gate into the global buffer returning
    /// a reference to the gate.
    ///
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
        // If feature 'static' is active,
        // lock the buffer to activate preformance improvments

        #[cfg(feature = "static_modules")]
        self.module_buffer.lock();
        #[cfg(feature = "static_channels")]
        self.channel_buffer.lock();
        #[cfg(feature = "static_gates")]
        self.gate_buffer.lock();
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
