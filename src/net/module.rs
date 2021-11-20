use crate::{Channel, Gate, GateDescription, GateId, GateType, GATE_NULL};

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

pub struct Module {
    pub id: ModuleId,
    pub gates: Vec<Gate>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            id: register_module(),
            gates: Vec::new(),
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
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}
