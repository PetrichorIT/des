mod build;
mod core;
mod ndl;

#[cfg(test)]
mod tests;

use crate::{
    net::*,
    util::{MrcS, Mutable, ReadOnly, UntypedMrc},
};

use std::ops::{Deref, DerefMut};

pub use self::build::*;
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
    ///         println!("Received {:?} with metadata {:?}", pkt, meta);
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
pub trait StaticModuleCore: Deref<Target = ModuleCore> + DerefMut<Target = ModuleCore> {
    ///
    /// A explicit deref to the Module Core.
    ///
    fn module_core(&self) -> &ModuleCore {
        self.deref()
    }

    ///
    /// A explicit deref_mut to the Module Core.
    ///
    fn module_core_mut(&mut self) -> &mut ModuleCore {
        self.deref_mut()
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

        let mrc = MrcS::clone(self);
        let descriptor = GateDescription::new(name.to_owned(), size, mrc, typ);
        let mut ids = Vec::new();

        for (i, item) in next_hops.into_iter().enumerate() {
            let gate = Gate::new(descriptor.clone(), i, channel.clone(), item);
            ids.push(MrcS::clone(&gate));

            self.deref_mut().gates.push(gate);
        }

        ids
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
        child.deref_mut().parent = Some(UntypedMrc::new(self_clone));

        let child_name = child.name().to_string();
        let owned_child = MrcS::clone(child);
        self.deref_mut()
            .children
            .insert(child_name, UntypedMrc::new(owned_child));
    }
}

impl<T> StaticModuleCore for T where T: Deref<Target = ModuleCore> + DerefMut<Target = ModuleCore> {}
