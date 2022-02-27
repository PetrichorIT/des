//mod buffer;
//pub use buffer::*;

use crate::net::*;
use crate::util::Indexable;
use crate::*;
use log::error;

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
    fn str(&self) -> String {
        self.module_core().identifier()
    }

    ///
    /// Returns the name of the module instance.
    ///
    fn name(&self) -> Option<&String> {
        self.module_core().name.as_ref()
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
    /// Indicates wether the module has a parent module.
    ///
    fn has_parent(&self) -> bool {
        self.module_core().parent_ptr.is_some()
    }

    ///
    /// Returns the parent element.
    ///
    fn parent<T: StaticModuleCore>(&self) -> Option<&T>
    where
        Self: Sized,
    {
        unsafe {
            let ptr = self.module_core().parent_ptr?;
            let ptr: *const T = ptr as *const T;
            Some(&*ptr)
        }
    }

    ///
    /// Returns the parent element mutablly.
    ///
    fn parent_mut<T: StaticModuleCore>(&mut self) -> Option<&mut T>
    where
        Self: Sized,
    {
        unsafe {
            let ptr = self.module_core_mut().parent_ptr?;
            let ptr: *mut T = ptr as *mut T;
            Some(&mut *ptr)
        }
    }

    ///
    /// Sets the parent element.
    ///
    fn set_parent<T: StaticModuleCore>(&mut self, module: &mut Box<T>)
    where
        Self: Sized,
    {
        let ptr: *mut T = &mut (**module);
        let ptr = ptr as *mut u8;
        self.module_core_mut().parent_ptr = Some(ptr);
    }
}

///
/// A trait that prepares a module to be created from a NDL
/// file.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait NdlCompatableModule: StaticModuleCore {
    ///
    /// Creates a named instance of self without needing any additional parameters.
    ///
    fn named(name: String) -> Self;

    ///
    /// Creates a named instance of self based on the parent hierachical structure.
    ///
    #[allow(clippy::borrowed_box)]
    fn named_with_parent<T: NdlCompatableModule>(name: &str, parent: &Box<T>) -> Self
    where
        Self: Sized,
    {
        // Clippy is just confused .. non box-borrow would throw E0277

        Self::named(format!(
            "{}.{}",
            parent.name().expect("Named entities should have names"),
            name
        ))
    }
}

///
/// A macro-implemented trait that constructs a instance of Self using a NDl
/// description.
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait NdlBuildableModule {
    ///
    /// Builds the given module according to the NDL specification
    /// if any is provided, else doesn't change a thing.
    ///
    fn build<A>(self: Box<Self>, _rt: &mut NetworkRuntime<A>) -> Box<Self>
    where
        Self: Sized,
    {
        self
    }

    fn build_named<A>(name: &str, rt: &mut NetworkRuntime<A>) -> Box<Self>
    where
        Self: NdlCompatableModule + Sized,
    {
        let obj = Box::new(Self::named(name.to_string()));
        Self::build(obj, rt)
    }

    fn build_named_with_parent<A, T>(
        name: &str,
        parent: &mut Box<T>,
        rt: &mut NetworkRuntime<A>,
    ) -> Box<Self>
    where
        T: NdlCompatableModule,
        Self: NdlCompatableModule + Sized,
    {
        let mut obj = Box::new(Self::named_with_parent(name, parent));
        obj.set_parent(parent);
        Self::build(obj, rt)
    }
}

///
/// The usecase independent core of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone)]
pub struct ModuleCore {
    id: ModuleId,

    /// A human readable identifier for the module.
    pub name: Option<String>,

    /// A collection of all gates register to the current module
    pub gates: Vec<GateRef>,

    /// A buffer of messages to be send out, after the current handle messsage terminates.
    pub out_buffer: Vec<(Message, GateId)>,

    /// A buffer of wakeup calls to be enqueued, after the current handle message terminates.
    pub loopback_buffer: Vec<(Message, SimTime)>,

    /// The period of the activity coroutine (if zero than there is no coroutine).
    pub activity_period: SimTime,

    /// An indicator whether a valid activity timeout is existent.
    pub activity_active: bool,

    /// The module identificator for the parent module.
    pub parent_ptr: Option<*mut u8>,
}

impl ModuleCore {
    /// A runtime specific but unqiue identifier for a given module.
    #[inline(always)]
    pub fn id(&self) -> ModuleId {
        self.id
    }

    /// A human readable identifer for a given module.
    pub fn identifier(&self) -> String {
        format!(
            "#{} {}",
            self.id,
            if self.name.is_some() {
                format!("({})", self.name.as_ref().unwrap())
            } else {
                "".into()
            }
        )
    }

    ///
    /// Creates a new optionally named instance
    /// of 'Self'.
    ///
    pub fn new_with(name: Option<String>) -> Self {
        Self {
            id: ModuleId::gen(),
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent_ptr: None,
            name,
        }
    }

    ///
    /// Creates a named instance of 'Self'.
    ///
    #[inline(always)]
    pub fn named(name: String) -> Self {
        Self::new_with(Some(name))
    }

    ///
    /// Creates  a not-named instance of 'Self'.
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self::new_with(None)
    }
}

impl Default for ModuleCore {
    fn default() -> Self {
        Self::new()
    }
}
