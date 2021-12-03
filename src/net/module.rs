use std::{cell::UnsafeCell, fmt::Display};

use crate::{
    Channel, Gate, GateDescription, GateId, GateType, IntoModuleGate, IntoModuleGateTrait, Message,
    SimTime, GATE_NULL,
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

pub type HandlerFunction = dyn Fn(&mut Module, Message);

///
/// A trait that defines a module
pub trait ModuleTrait {
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
        self.create_gate_cluster(name, 0, typ, channel)[0]
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
        self.create_gate_cluster_into(name, 0, typ, channel, vec![next_hop])[0]
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

        for i in 0..size {
            let gate = Gate::new(descriptor.clone(), i, channel, next_hops[i]);
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
        T: IntoModuleGateTrait<Self>,
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
}

pub struct Module {
    pub id: ModuleId,
    pub gates: Vec<Gate>,
    pub handle: UnsafeCell<Box<HandlerFunction>>,

    out_queue: Vec<(Message, usize)>,
}

impl Module {
    pub fn new(handler: &'static HandlerFunction) -> Self {
        Self {
            id: register_module(),
            gates: Vec::new(),
            handle: UnsafeCell::new(Box::new(handler)),

            out_queue: Vec::new(),
        }
    }

    pub fn gate(&self, name: &str, idx: usize) -> Option<&Gate> {
        self.gates
            .iter()
            .find(|g| g.name() == name && g.pos() == idx)
    }

    pub fn gate_mut(&mut self, name: &str, idx: usize) -> Option<&mut Gate> {
        self.gates
            .iter_mut()
            .find(|g| g.name() == name && g.pos() == idx)
    }

    #[inline(always)]
    pub fn create_gate(&mut self, name: String, typ: GateType, channel: &Channel) -> GateId {
        self.create_gate_cluster(name, 1, typ, channel)[0]
    }

    pub fn create_gate_into(
        &mut self,
        name: String,
        typ: GateType,
        channel: &Channel,
        next_hop: GateId,
    ) -> GateId {
        let id = self.create_gate_cluster(name, 1, typ, channel)[0];
        self.gates.last_mut().unwrap().set_next_gate(next_hop);
        id
    }

    pub fn create_gate_cluster(
        &mut self,
        name: String,
        size: usize,
        typ: GateType,
        channel: &Channel,
    ) -> Vec<GateId> {
        let descriptor = GateDescription::new(typ, name, size, self.id);
        let mut ids = Vec::new();

        for i in 0..size {
            let gate = Gate::new(descriptor.clone(), i, channel, GATE_NULL);
            ids.push(gate.id());
            self.gates.push(gate);
        }
        ids
    }

    pub fn handle_message<A>(&mut self, message: Message) -> Vec<(Message, GateId)> {
        let f = self.handle.get();
        let f = unsafe { &*f };

        // Handler
        f(self, message);

        // Compute results
        self.out_queue
            .drain(0..)
            .map(|(msg, idx)| (msg, self.gates[idx].id()))
            .collect()
    }

    // UTIL

    pub fn send<T>(&mut self, message: Message, gate: T)
    where
        T: IntoModuleGate,
    {
        let gate_idx = gate.into_gate(self);
        if let Some(gate_idx) = gate_idx {
            self.out_queue.push((message, gate_idx))
        } else {
            eprintln!("Error: Could not find gate in current module");
        }
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new(&|_b, _c| {})
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module [{}]", self.id)
    }
}
