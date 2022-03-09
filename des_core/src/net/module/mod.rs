mod core;
mod ndl;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::net::*;
use crate::util::Indexable;
use crate::*;
use log::error;

pub use self::core::*;
pub use self::ndl::*;

///
/// A set of user defined functions for customizing the
/// behaviour of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait Module: StaticModuleCore {
    ///
    /// A message handler for receiving events, user defined.
    ///
    /// # Example
    ///
    /// ```
    /// use des_core::*;
    /// use des_macros::Module;
    ///
    /// #[derive(Module)]
    /// struct MyModule {
    ///     core: ModuleCore,
    ///
    ///     my_prop_1: f64,
    ///     my_prop_2: String,
    /// };
    ///
    /// impl Module for MyModule {
    ///     fn handle_message(&mut self, msg: Message) {
    ///         let (pkt, meta) = msg.cast::<Packet>();
    ///         println!("Received {:?} with metadata {:?}", *pkt, meta);
    ///     }
    /// }
    /// ```
    ///
    fn handle_message(&mut self, msg: Message);

    ///
    /// A periodic activity handler.
    ///
    /// # Example
    ///
    /// ```
    /// use des_core::*;
    /// use des_macros::Module;
    /// # fn is_good_packet<T>(_t: T) -> bool { true }
    ///
    /// #[derive(Module)]
    /// struct OurModule {
    ///     core: ModuleCore,
    ///
    ///     good_packets: f64,
    ///     bad_packets: f64,
    ///
    ///     records: Vec<f64>,
    /// };
    ///
    /// impl Module for OurModule {
    ///     fn handle_message(&mut self, msg: Message) {
    ///         let (pkt, _meta) = msg.cast::<Packet>();
    ///         if is_good_packet(pkt) {
    ///             self.good_packets += 1.0;
    ///         } else {
    ///             self.bad_packets += 1.0;
    ///         }
    ///     }
    ///
    ///     fn activity(&mut self) {
    ///         // Record accummulated percentage over time
    ///         self.records.push(self.good_packets / self.bad_packets);
    ///     }
    /// }
    /// ```
    fn activity(&mut self) {}

    ///
    /// A function that is run at the start of each simulation,
    /// for each module.
    ///
    /// # Example
    ///
    /// ```
    /// use des_core::*;
    /// use des_macros::Module;
    /// # type Config = ();
    /// # type Record = u8;
    /// # fn fetch_config(s: &str, id: ModuleId) -> Config {}
    ///
    /// #[derive(Module)]
    /// struct SomeModule {
    ///     core: ModuleCore,
    ///
    ///     config: Config,
    ///     records: Vec<Record>,
    /// };
    ///
    /// impl Module for SomeModule {
    ///     fn at_sim_start(&mut self) {
    ///         self.config = fetch_config("https://mysimconfig.com/simrun1", self.id());
    ///         self.records.clear();
    ///     }
    ///
    ///     fn handle_message(&mut self, msg: Message) {
    ///         todo!()
    ///     }
    /// }
    /// ```
    ///
    fn at_sim_start(&mut self) {}

    ///
    /// A callback function that is invoked should the simulation finish.
    /// All events emitted by this function will NOT be processed.
    ///
    fn at_sim_end(&mut self) {}
}

///
/// A marco-implemented trait that defines the static core components
/// of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait StaticModuleCore: Indexable<Id = ModuleId> {
    ///
    /// Returns a pointer to the modules core, used for handling event and
    /// buffers that are use case unspecific.
    ///
    fn module_core(&self) -> &ModuleCore;

    ///
    /// Returns a mutable pointer to the modules core, used for handling event and
    /// buffers that are use case unspecific.
    ///
    fn module_core_mut(&mut self) -> &mut ModuleCore;

    ///
    /// Returns a human readable representation of the modules identity.
    ///
    fn str(&self) -> &str {
        self.module_core().identifier()
    }

    ///
    /// Returns the name of the module instance.
    ///
    fn path(&self) -> &ModulePath {
        &self.module_core().path
    }

    ///
    /// Returns the name of the module instance.
    ///
    fn name(&self) -> &str {
        self.module_core().path.name()
    }

    fn pars(&self) -> HashMap<String, String> {
        self.module_core()
            .parameters
            .get(self.module_core().path.path())
    }

    ///
    /// Returns a ref unstructured list of all gates from the current module.
    ///
    fn gates(&self) -> Vec<&Gate> {
        self.module_core().gates.iter().map(|r| r.get()).collect()
    }

    ///
    /// Returns a mutable ref to the all gates list.
    ///
    fn gates_mut(&mut self) -> Vec<&mut Gate> {
        self.module_core_mut()
            .gates
            .iter_mut()
            .map(|r| r.get_mut())
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_cluster(&self, name: &str) -> Vec<&Gate> {
        self.gates()
            .into_iter()
            .filter(|&gate| gate.name() == name)
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_cluster_mut(&mut self, name: &str) -> Vec<&mut Gate> {
        self.gates_mut()
            .into_iter()
            .filter(|gate| gate.name() == name)
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate(&self, name: &str, pos: usize) -> Option<&Gate> {
        self.gates()
            .into_iter()
            .find(|&gate| gate.name() == name && gate.pos() == pos)
    }

    fn gate_by_id(&self, id: GateId) -> Option<&Gate> {
        self.gates().into_iter().find(|&gate| gate.id() == id)
    }

    ///
    /// Returns a mutable ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_mut(&mut self, name: &str, pos: usize) -> Option<&mut Gate> {
        self.gates_mut()
            .into_iter()
            .find(|gate| gate.name() == name && gate.pos() == pos)
    }

    fn gate_by_id_mut(&mut self, id: GateId) -> Option<&mut Gate> {
        self.gates_mut().into_iter().find(|gate| gate.id() == id)
    }

    ///
    /// Creates a gate on the current module, returning its ID.
    ///
    fn create_gate<A>(&mut self, name: &str, rt: &mut NetworkRuntime<A>) -> GateId
    where
        Self: Sized,
    {
        self.create_gate_cluster(name, 1, rt)[0]
    }

    ///
    /// Creates a gate on the current module that points to another gate as its
    /// next hop, returning the ID of the created gate.
    ///
    fn create_gate_into<A>(
        &mut self,
        name: &str,
        channel: Option<ChannelRef>,
        next_hop: GateId,
        rt: &mut NetworkRuntime<A>,
    ) -> GateId
    where
        Self: Sized,
    {
        self.create_gate_cluster_into(name, 1, channel, vec![next_hop], rt)[0]
    }

    ///
    /// Createas a cluster of gates on the current module returning their IDs.
    ///
    fn create_gate_cluster<A>(
        &mut self,
        name: &str,
        size: usize,
        rt: &mut NetworkRuntime<A>,
    ) -> Vec<GateId>
    where
        Self: Sized,
    {
        self.create_gate_cluster_into(name, size, None, vec![GateId::NULL; size], rt)
    }

    ///
    /// Creates a cluster of gates on the current module, pointing to the given next hops,
    /// returning the new IDs.
    ///
    /// # Panics
    ///
    /// This function will panic should size != next_hops.len()
    ///
    fn create_gate_cluster_into<A>(
        &mut self,
        name: &str,
        size: usize,
        channel: Option<ChannelRef>,
        next_hops: Vec<GateId>,
        rt: &mut NetworkRuntime<A>,
    ) -> Vec<GateId>
    where
        Self: Sized,
    {
        assert!(size == next_hops.len());

        let descriptor = GateDescription::new(name.to_owned(), size, self.id());
        let mut ids = Vec::new();

        for (i, item) in next_hops.iter().enumerate() {
            let gate = Gate::new(descriptor.clone(), i, channel.clone(), *item);
            ids.push(gate.id());

            let reference = rt.create_gate(gate);
            self.module_core_mut().gates.push(reference);
        }

        ids
    }

    /// User message handling

    ///
    /// Sends a message onto a given gate. This operation will be performed after
    /// handle_message finished.
    ///
    fn send<T>(&mut self, msg: Message, gate: T)
    where
        T: IntoModuleGate<Self>,
        Self: Sized,
    {
        let gate_idx = gate.into_gate(self);
        if let Some(gate_idx) = gate_idx {
            self.module_core_mut().out_buffer.push((msg, gate_idx))
        } else {
            error!(target: &self.str(),"Error: Could not find gate in current module");
        }
    }

    ///
    /// Enqueues a event that will trigger the [Module::handle_message] function
    /// at the given SimTime
    fn schedule_at(&mut self, msg: Message, time: SimTime) {
        assert!(time >= SimTime::now());
        self.module_core_mut().loopback_buffer.push((msg, time))
    }

    ///
    /// Enables the activity corountine using the given period.
    /// This function should only be called from [Module::handle_message].
    ///
    fn enable_activity(&mut self, period: SimTime) {
        self.module_core_mut().activity_period = period;
        self.module_core_mut().activity_active = false;
    }

    ///
    /// Disables the activity coroutine cancelling the next call.
    ///
    fn disable_activity(&mut self) {
        self.module_core_mut().activity_period = SimTime::ZERO;
        self.module_core_mut().activity_active = false;
    }

    ///
    /// Returns the parent element.
    ///
    fn parent<T>(&self) -> Result<&T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: Sized,
    {
        self.module_core().parent()
    }

    ///
    /// Returns the parent element mutablly.
    ///
    fn parent_mut<T>(&mut self) -> Result<&mut T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: Sized,
    {
        self.module_core_mut().parent_mut()
    }

    ///
    /// Returns a mutable reference to a child, assuming the module exists under this name
    /// and has the type T.
    ///
    fn child<T>(&self, name: &str) -> Result<&T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: Sized,
    {
        self.module_core().child(name)
    }

    ///
    /// Returns a mutable reference to a child, assuming the module exists under this name
    /// and has the type T.
    ///
    fn child_mut<T>(&mut self, name: &str) -> Result<&mut T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: Sized,
    {
        self.module_core_mut().child_mut(name)
    }

    ///
    /// Adds the given module as a child module, automaticly writing
    /// the childs parent.
    ///
    fn add_child<T>(&mut self, module: &mut T)
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        self.module_core_mut().add_child::<Self, T>(module)
    }
}
