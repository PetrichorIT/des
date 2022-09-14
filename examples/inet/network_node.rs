use des::{net::NetworkRuntimeGlobals, prelude::*};
use log::info;

#[NdlModule]
pub struct NetworkNode {
    core: ModuleCore,
}

impl NetworkNode {
    #[allow(unused)]
    pub fn new(globals: PtrWeakConst<NetworkRuntimeGlobals>) -> Self {
        Self {
            core: ModuleCore::new_with("NetworkNode".parse().unwrap(), globals),
        }
    }

    pub fn named(name: &str, globals: PtrWeakConst<NetworkRuntimeGlobals>) -> Self {
        Self {
            core: ModuleCore::new_with(name.parse().unwrap(), globals),
        }
    }
}

impl Module for NetworkNode {
    fn handle_message(&mut self, msg: Message) {
        let incoming = msg.header().last_gate.as_ref().unwrap();

        let pos = incoming.pos();
        info!("Node incoming at gate {:?}", incoming);
        if incoming.name().eq("channelIncoming") {
            // From channel
            self.send(msg, ("toStack", pos))
        } else {
            self.send(msg, ("channelOutgoing", pos))
        }
    }
}
