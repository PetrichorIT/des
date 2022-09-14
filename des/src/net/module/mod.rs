use std::any::Any;

use crate::net::Message;

mod ctx;
pub use self::ctx::*;

mod mref;
pub use mref::*;

mod error;
pub use error::*;

cfg_async! {
    mod async_mod;
    pub use self::async_mod::*;
}

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ModuleId(u16) = MODULE_ID;
);

///
/// A set of user defined functions for customizing the
/// behaviour of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait Module: Any {
    /// Creates a new instance of Self.
    fn new() -> Self
    where
        Self: Sized;

    ///
    /// A message handler for receiving events, user defined.
    ///
    /// # Example
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// #[NdlModule]
    /// struct MyModule {
    ///     my_prop_1: f64,
    ///     my_prop_2: String,
    /// };
    ///
    /// impl Module for MyModule {
    /// # fn new() -> Self { todo!() }
    ///     /* ... */    
    ///
    ///     fn handle_message(&mut self, msg: Message) {
    ///         println!("Received {:?}", msg);
    ///     }
    /// }
    /// ```
    ///
    fn handle_message(&mut self, _msg: Message) {}

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
    /// # type Config = ();
    /// # type Record = u8;
    /// # fn fetch_config(s: &str, id: ModuleId) -> Config {}
    ///
    /// #[NdlModule]
    /// struct SomeModule {
    ///     config: Config,
    ///     records: Vec<Record>,
    /// };
    ///
    /// impl Module for SomeModule {
    /// # fn new() -> Self { todo!() }
    ///     /* ... */
    ///     
    ///     fn at_sim_start(&mut self, _stage: usize) {
    ///         self.config = fetch_config("https://mysimconfig.com/simrun1", module_id());
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
    /// A function that is called when all `sim_start` stages of all modules
    /// are done. Used to resolve all async `sim_start_stages`.
    ///
    #[cfg(feature = "async")]
    fn finish_sim_start(&mut self) {}

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

    ///
    /// A function that is called when all `sim_end` stages of all modules
    /// are done. Used to resolve all async `sim_end_stages`.
    ///
    #[cfg(feature = "async")]
    fn finish_sim_end(&mut self) {}

    ///
    /// A callback function that is called should a parameter belonging to
    /// this module be changed.
    ///
    fn handle_par_change(&mut self) {}
}

// ///
// /// A marco-implemented trait that defines the static core components
// /// of a module.
// ///
// /// * This type is only available of DES is build with the `"net"` feature.*
// #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
// pub trait StaticModuleCore: Deref<Target = ModuleCore> + DerefMut<Target = ModuleCore> {
//     ///
//     /// A explicit deref to the Module Core.
//     ///
//     fn module_core(&self) -> &ModuleCore {
//         self
//     }

//     ///
//     /// A explicit `deref_mut` to the Module Core.
//     ///
//     fn module_core_mut(&mut self) -> &mut ModuleCore {
//         self
//     }

//     ///
//     /// Creates a gate on the current module, returning its ID.
//     ///
//     fn create_gate<A>(
//         self: &mut PtrMut<Self>,
//         name: &str,
//         typ: GateServiceType,
//         rt: &mut NetworkRuntime<A>,
//     ) -> GateRefMut
//     where
//         Self: 'static + Sized + Module,
//     {
//         self.create_gate_cluster(name, 1, typ, rt).remove(0)
//     }

//     ///
//     /// Creates a gate on the current module that points to another gate as its
//     /// next hop, returning the ID of the created gate.
//     ///
//     fn create_gate_into<A>(
//         self: &mut PtrMut<Self>,
//         name: &str,
//         typ: GateServiceType,
//         channel: Option<ChannelRefMut>,
//         next_hop: Option<GateRefMut>,
//         rt: &mut NetworkRuntime<A>,
//     ) -> GateRefMut
//     where
//         Self: 'static + Sized + Module,
//     {
//         self.create_gate_cluster_into(name, 1, typ, channel, vec![next_hop], rt)
//             .remove(0)
//     }

//     ///
//     /// Createas a cluster of gates on the current module returning their IDs.
//     ///
//     fn create_gate_cluster<A>(
//         self: &mut PtrMut<Self>,
//         name: &str,
//         size: usize,
//         typ: GateServiceType,
//         rt: &mut NetworkRuntime<A>,
//     ) -> Vec<GateRefMut>
//     where
//         Self: 'static + Sized + Module,
//     {
//         self.create_gate_cluster_into(name, size, typ, None, vec![None; size], rt)
//     }

//     ///
//     /// Creates a cluster of gates on the current module, pointing to the given next hops,
//     /// returning the new IDs.
//     ///
//     /// # Panics
//     ///
//     /// This function will panic should size != `next_hops.len`()
//     ///
//     fn create_gate_cluster_into<A>(
//         self: &mut PtrMut<Self>,
//         name: &str,
//         size: usize,
//         typ: GateServiceType,
//         channel: Option<ChannelRefMut>,
//         next_hops: Vec<Option<GateRefMut>>,
//         _rt: &mut NetworkRuntime<A>,
//     ) -> Vec<GateRefMut>
//     where
//         Self: 'static + Sized + Module,
//     {
//         assert!(
//             size == next_hops.len(),
//             "The value 'next_hops' must be equal to the size of the gate cluster"
//         );

//         let ptr = PtrWeakMut::from_strong(self);
//         let descriptor = GateDescription::new(name.to_owned(), size, ptr, typ);
//         let mut ids = Vec::new();

//         for (i, item) in next_hops.into_iter().enumerate() {
//             let gate = Gate::new(descriptor.clone(), i, channel.clone(), item);
//             ids.push(Ptr::clone(&gate));

//             self.deref_mut().gates.push(gate);
//         }

//         ids
//     }

//     ///
//     /// Adds the given module as a child module, automaticlly seting the childs
//     /// parent property .
//     ///
//     fn add_child<T>(self: &mut PtrMut<Self>, child: &mut PtrMut<T>)
//     where
//         T: 'static + StaticModuleCore,
//         Self: 'static + Sized,
//     {
//         // Self refs mus be set
//         if self.module_core_mut().self_ref.is_none() {
//             warn!(target: self.str(), "Setting self_ref at child assignal (self = parent)");
//             self.module_core_mut().self_ref = Some(PtrWeakVoid::new(PtrWeakMut::from_strong(self)));
//         }
//         if child.module_core_mut().self_ref.is_none() {
//             warn!(target: child.str(), "Setting self_ref at child assignal (self = child)");
//             child.module_core_mut().self_ref =
//                 Some(PtrWeakVoid::new(PtrWeakMut::from_strong(child)));
//         }

//         let self_clone = PtrWeakMut::from_strong(self);
//         child.deref_mut().parent = Some(self_clone);

//         let child_name = child.name().to_string();
//         let owned_child = PtrWeakMut::from_strong(child);
//         self.deref_mut().children.insert(child_name, owned_child);
//     }
// }

// impl<T> StaticModuleCore for T where T: Deref<Target = ModuleCore> + DerefMut<Target = ModuleCore> {}
