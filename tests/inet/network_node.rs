use des::{Module, ModuleCore, SpmcReader};
use des::{Parameters, StaticModuleCore};
use log::warn;

use des_derive::Module;

#[derive(Module)]
pub struct NetworkNode {
    core: ModuleCore,
}

impl NetworkNode {
    #[allow(unused)]
    pub fn new(parameters: SpmcReader<Parameters>) -> Self {
        Self {
            core: ModuleCore::new_with("NetworkNode".parse().unwrap(), parameters),
        }
    }

    pub fn named(name: &str, parameters: SpmcReader<Parameters>) -> Self {
        Self {
            core: ModuleCore::new_with(
                format!("NetworkNode - {}", name).parse().unwrap(),
                parameters,
            ),
        }
    }
}

impl Module for NetworkNode {
    fn handle_message(&mut self, msg: des::Message) {
        let incoming = msg.meta().last_gate.as_ref().unwrap();

        let pos = incoming.pos();
        warn!(target: self.str(), "Node incoming at gate {:?}", incoming);
        if incoming.name().eq("channelIncoming") {
            // From channel
            self.send(msg, ("toStack", pos))
        } else {
            self.send(msg, ("channelOutgoing", pos))
        }
    }
}
