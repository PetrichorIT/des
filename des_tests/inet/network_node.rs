use des_core::StaticModuleCore;
use des_core::{Module, ModuleCore};
use log::warn;

use des_macros::Module;

#[derive(Module)]
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
    fn handle_message(&mut self, msg: des_core::Message) {
        let incoming = self.gate_by_id(msg.meta().last_gate).unwrap();

        let pos = incoming.pos();
        warn!(target: &self.str(), "Node incoming at gate {:?}", incoming);
        if incoming.name().eq("channelIncoming") {
            // From channel
            self.send(msg, ("toStack", pos))
        } else {
            self.send(msg, ("channelOutgoing", pos))
        }
    }
}
