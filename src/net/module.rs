use crate::{
    Channel, Gate, GateDescription, GateId, GateType, IntoModuleGate, Message, SimTime, GATE_NULL,
};

/// A runtime-unqiue identifier for a module / submodule inheritence tree.
pub type ModuleId = u16;

/// A indication that the referenced module does not exist.
pub const MODULE_NULL: ModuleId = 0;

static mut MODULE_ID: ModuleId = 0xff;
fn register_module() -> ModuleId {
    unsafe {
        let r = MODULE_ID;
        MODULE_ID += 1;
        r
    }
}

///
/// A trait that defines a module
pub trait Module {
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
    /// Returns the internal unquie identifier for the given module.
    ///
    fn id(&self) -> ModuleId {
        self.module_core().id
    }

    ///
    /// Returns a ref unstructured list of all gates from the current module.
    ///
    fn gates(&self) -> &Vec<Gate> {
        &self.module_core().gates
    }

    ///
    /// Returns a mutable ref to the all gates list.
    ///
    fn gates_mut(&mut self) -> &mut Vec<Gate> {
        &mut self.module_core_mut().gates
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate(&self, name: &str, pos: usize) -> Option<&Gate> {
        self.gates()
            .iter()
            .find(|&gate| gate.name() == name && gate.pos() == pos)
    }

    ///
    /// Returns a mutable ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    fn gate_mut(&mut self, name: &str, pos: usize) -> Option<&mut Gate> {
        self.gates_mut()
            .iter_mut()
            .find(|gate| gate.name() == name && gate.pos() == pos)
    }

    ///
    /// Creates a gate on the current module, returning its ID.
    ///
    fn create_gate(&mut self, name: String, typ: GateType, channel: &Channel) -> GateId {
        self.create_gate_cluster(name, 1, typ, channel)[0]
    }

    ///
    /// Creates a gate on the current module that points to another gate as its
    /// next hop, returning the ID of the created gate.
    ///
    fn create_gate_into(
        &mut self,
        name: String,
        typ: GateType,
        channel: &Channel,
        next_hop: GateId,
    ) -> GateId {
        self.create_gate_cluster_into(name, 1, typ, channel, vec![next_hop])[0]
    }

    ///
    /// Createas a cluster of gates on the current module returning their IDs.
    ///
    fn create_gate_cluster(
        &mut self,
        name: String,
        size: usize,
        typ: GateType,
        channel: &Channel,
    ) -> Vec<GateId> {
        self.create_gate_cluster_into(name, size, typ, channel, vec![GATE_NULL; size])
    }

    ///
    /// Creates a cluster of gates on the current module, pointing to the given next hops,
    /// returning the new IDs.
    ///
    /// # Panics
    ///
    /// This function will panic should size != next_hops.len()
    ///
    fn create_gate_cluster_into(
        &mut self,
        name: String,
        size: usize,
        typ: GateType,
        channel: &Channel,
        next_hops: Vec<GateId>,
    ) -> Vec<GateId> {
        assert!(size == next_hops.len());

        let descriptor = GateDescription::new(typ, name, size, self.id());
        let mut ids = Vec::new();

        for (i, item) in next_hops.iter().enumerate() {
            let gate = Gate::new(descriptor.clone(), i, channel, *item);
            ids.push(gate.id());
            self.gates_mut().push(gate);
        }

        ids
    }

    /// User message handling

    ///
    /// A message handler for receiving events, user defined.
    ///
    fn handle_message(&mut self, msg: Message);

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
            eprintln!("Error: Could not find gate in current module");
        }
    }

    ///
    /// Enqueues a event that will trigger the [Self::handle_message] function
    /// at the given SimTime
    fn schedule_at(&mut self, msg: Message, time: SimTime) {
        assert!(time >= SimTime::now());
        self.module_core_mut().loopback_buffer.push((msg, time))
    }

    ///
    /// A periodic activity handler.
    ///
    fn activity(&mut self) {}

    ///
    /// Enables the activity corountine using the given period.
    /// This function should only be called from [Self::handle_message].
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
    }
}

///
/// The usecase independent core of a module.
///
#[derive(Debug, Clone)]
pub struct ModuleCore {
    /// A runtime specific but unqiue identifier for a given module.
    pub id: ModuleId,

    /// A collection of all gates register to the current module
    pub gates: Vec<Gate>,

    /// A buffer of messages to be send out, after the current handle messsage terminates.
    pub out_buffer: Vec<(Message, GateId)>,

    /// A buffer of wakeup calls to be enqueued, after the current handle message terminates.
    pub loopback_buffer: Vec<(Message, SimTime)>,

    /// The period of the activity coroutine (if zero than there is no coroutine).
    pub activity_period: SimTime,

    /// An indicator whether a valid activity timeout is existent.
    pub activity_active: bool,
}

impl ModuleCore {
    pub fn new() -> Self {
        Self {
            id: register_module(),
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
        }
    }
}

impl Default for ModuleCore {
    fn default() -> Self {
        Self::new()
    }
}
