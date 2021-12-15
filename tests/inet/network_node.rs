use dse::{Module, ModuleCore};
use log::warn;

pub struct NetworkNode {
    core: ModuleCore,
}

impl NetworkNode {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            core: ModuleCore::new_with(Some("NetworkNode".to_string())),
        }
    }

    pub fn named(name: &str) -> Self {
        Self {
            core: ModuleCore::new_with(Some(format!("NetworkNode - {}", name))),
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
        warn!(target: &self.identifier(), "Node incoming at gate {:?}", incoming);
        if incoming.name().eq("channelIncoming") {
            // From channel
            self.send(msg, ("toStack", pos))
        } else {
            self.send(msg, ("channelOutgoing", pos))
        }
    }
}
