mod core;
mod ndl;

#[cfg(test)]
mod tests;

use crate::{
    core::SimTime,
    net::*,
    util::{MrcS, Mutable, ReadOnly, UntypedMrc},
};
use log::error;
use std::collections::HashMap;

pub use self::core::*;
pub use self::ndl::*;

///
/// A readonly reference to a module.
///
pub type ModuleRef = MrcS<dyn Module, ReadOnly>;

///
/// A mutable reference to a module.
///
pub type ModuleRefMut = MrcS<dyn Module, Mutable>;

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
    /// use des::prelude::*;
    /// use des_derive::Module;
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
    /// use des::prelude::*;
    /// use des_derive::Module;
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
    /// for each module. The order in which modules are called is not guranteed
    /// but the stage numbers are. That means that all stage-0 calls for all modules
    /// happen before the first (if any) stage-1 calls. Generaly speaking, all stage-i
    /// calls finish before the first stage-i+1 call.
    ///
    /// # Example
    ///
    /// ```
    /// use des::prelude::*;
    /// use des_derive::Module;
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
    ///     fn at_sim_start(&mut self, _stage: usize) {
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

///
/// A marco-implemented trait that defines the static core components
/// of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait StaticModuleCore {
    fn id(&self) -> ModuleId {
        self.module_core().id()
    }

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
        self.module_core().path.path()
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
        self.module_core().pars()
    }

    ///
    /// Returns a ref unstructured list of all gates from the current module.
    ///
    fn gates(&self) -> &Vec<GateRefMut> {
        &self.module_core().gates
    }

    ///
    /// Returns a mutable ref to the all gates list.
    ///
    fn gates_mut(&mut self) -> &mut Vec<GateRefMut> {
        &mut self.module_core_mut().gates
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_cluster(&self, name: &str) -> Vec<&GateRefMut> {
        self.gates()
            .iter()
            .filter(|&gate| gate.name() == name)
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_cluster_mut(&mut self, name: &str) -> Vec<&mut GateRefMut> {
        self.gates_mut()
            .iter_mut()
            .filter(|gate| gate.name() == name)
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        Some(
            MrcS::clone(
                self.gates()
                    .iter()
                    .find(|&gate| gate.name() == name && gate.pos() == pos)?,
            )
            .make_readonly(),
        )
    }

    #[deprecated(since = "0.2.0", note = "GateIDs are no longer supported")]
    fn gate_by_id(&self, _id: ()) -> ! {
        unimplemented!("GateIDs are no longer supported")
    }

    ///
    /// Returns a mutable ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_mut(&mut self, name: &str, pos: usize) -> Option<&mut GateRefMut> {
        self.gates_mut()
            .iter_mut()
            .find(|gate| gate.name() == name && gate.pos() == pos)
    }

    #[deprecated(since = "0.2.0", note = "GateIDs are no longer supported")]
    fn gate_by_id_mut(&mut self, _id: ()) -> ! {
        unimplemented!("GateIDs are no longer supported")
    }

    ///
    /// Creates a gate on the current module, returning its ID.
    ///
    fn create_gate<A>(
        self: &mut MrcS<Self, Mutable>,
        name: &str,
        typ: GateServiceType,
        rt: &mut NetworkRuntime<A>,
    ) -> GateRefMut
    where
        Self: 'static + Sized + Module,
    {
        self.create_gate_cluster(name, 1, typ, rt).remove(0)
    }

    ///
    /// Creates a gate on the current module that points to another gate as its
    /// next hop, returning the ID of the created gate.
    ///
    fn create_gate_into<A>(
        self: &mut MrcS<Self, Mutable>,
        name: &str,
        typ: GateServiceType,
        channel: Option<ChannelRefMut>,
        next_hop: Option<GateRefMut>,
        rt: &mut NetworkRuntime<A>,
    ) -> GateRefMut
    where
        Self: 'static + Sized + Module,
    {
        self.create_gate_cluster_into(name, 1, typ, channel, vec![next_hop], rt)
            .remove(0)
    }

    ///
    /// Createas a cluster of gates on the current module returning their IDs.
    ///
    fn create_gate_cluster<A>(
        self: &mut MrcS<Self, Mutable>,
        name: &str,
        size: usize,
        typ: GateServiceType,
        rt: &mut NetworkRuntime<A>,
    ) -> Vec<GateRefMut>
    where
        Self: 'static + Sized + Module,
    {
        self.create_gate_cluster_into(name, size, typ, None, vec![None; size], rt)
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
        self: &mut MrcS<Self, Mutable>,
        name: &str,
        size: usize,
        typ: GateServiceType,
        channel: Option<ChannelRefMut>,
        next_hops: Vec<Option<GateRefMut>>,
        _rt: &mut NetworkRuntime<A>,
    ) -> Vec<GateRefMut>
    where
        Self: 'static + Sized + Module,
    {
        assert!(
            size == next_hops.len(),
            "The value 'next_hops' must be equal to the size of the gate cluster"
        );

        let mrc = MrcS::clone(self).make_readonly();
        let descriptor = GateDescription::new_with_typ(name.to_owned(), size, mrc, typ);
        let mut ids = Vec::new();

        for (i, item) in next_hops.into_iter().enumerate() {
            let gate = Gate::new(
                descriptor.clone(),
                i,
                channel.clone(),
                item.map(MrcS::make_readonly),
            );
            ids.push(MrcS::clone(&gate));

            self.module_core_mut().gates.push(gate);
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
        let gate = gate.into_gate(self);
        if let Some(gate) = gate {
            self.module_core_mut().out_buffer.push((msg, gate))
        } else {
            error!(target: self.str(),"Error: Could not find gate in current module");
        }
    }

    ///
    /// Enqueues a event that will trigger the [Module::handle_message] function
    /// at the given SimTime
    ///
    fn schedule_at(&mut self, msg: Message, time: SimTime) {
        assert!(time >= SimTime::now(), "Sorry, we can not timetravel yet!");
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
    /// Adds the given module as a child module, automaticlly seting the childs
    /// parent property .
    ///
    fn add_child<T>(self: &mut MrcS<Self, Mutable>, child: &mut MrcS<T, Mutable>)
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        let self_clone = MrcS::clone(self);
        child.module_core_mut().parent = Some(UntypedMrc::new(self_clone));

        let child_name = child.name().to_string();
        let owned_child = MrcS::clone(child);
        self.module_core_mut()
            .children
            .insert(child_name, UntypedMrc::new(owned_child));
    }

    ///
    /// Returns the parent module by reference if a parent exists
    /// and is of type `T`.
    ///
    fn parent<T>(&self) -> Result<MrcS<T, ReadOnly>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        self.module_core().parent()
    }

    ///
    /// Returns the parent module by mutable reference if a parent exists
    /// and is of type `T`.
    ///
    fn parent_mut<T>(&mut self) -> Result<MrcS<T, Mutable>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        self.module_core_mut().parent_mut()
    }

    ///
    /// Returns the child module by reference if any child with
    /// the given name exists and is of type `T`.
    ///
    fn child<T>(&self, name: &str) -> Result<MrcS<T, ReadOnly>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        self.module_core().child(name)
    }

    ///
    /// Returns the child module by mutable reference if any child with
    /// the given name exists and is of type `T`.
    ///
    fn child_mut<T>(&mut self, name: &str) -> Result<MrcS<T, Mutable>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        self.module_core_mut().child_mut(name)
    }

    ///
    /// Returns a readonly reference to the runtimes globals.
    ///
    fn globals(&self) -> MrcS<NetworkRuntimeGlobals, ReadOnly> {
        self.module_core().globals()
    }
}
