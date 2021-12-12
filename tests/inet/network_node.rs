use dse::{Module, ModuleCore};

pub struct NetworkNode {
    core: ModuleCore,
}

impl NetworkNode {
    pub fn new() -> Self {
        Self {
            core: ModuleCore::new(),
        }
    }
}

impl Module for NetworkNode {
    fn module_core(&self) -> &ModuleCore {
        &self.core
    }

    fn module_core_mut(&mut self) -> &mut ModuleCore {
        &mut self.core
    }

    fn handle_message(&mut self, msg: dse::Message) {
        let incoming = self.gate_by_id(msg.arrival_gate()).unwrap();

        let pos = incoming.pos();
        if incoming.name().eq("channelIncoming") {
            // From channel
            self.send(msg, ("toStack", pos))
        } else {
            self.send(msg, ("channelOutgoing", pos))
        }
    }
}
