use des::{
    net::NetworkRuntimeGlobals,
    prelude::*,
    util::{MrcS, ReadOnly},
};
use log::warn;

use des_derive::Module;

#[derive(Module)]
pub struct NetworkNode {
    core: ModuleCore,
}

impl NetworkNode {
    #[allow(unused)]
    pub fn new(globals: MrcS<NetworkRuntimeGlobals, ReadOnly>) -> Self {
        Self {
            core: ModuleCore::new_with("NetworkNode".parse().unwrap(), globals),
        }
    }

    pub fn named(name: &str, globals: MrcS<NetworkRuntimeGlobals, ReadOnly>) -> Self {
        Self {
            core: ModuleCore::new_with(format!("{}", name).parse().unwrap(), globals),
        }
    }
}

impl Module for NetworkNode {
    fn handle_message(&mut self, msg: Message) {
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
