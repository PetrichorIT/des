use std::collections::HashMap;

use des::{net::NetworkRuntimeGlobals, prelude::*};
use log::info;

#[NdlModule]
pub struct RandomRoutingDeamon {
    core: ModuleCore,

    hop_counts: HashMap<IpAddr, usize>,
}

impl RandomRoutingDeamon {
    pub fn new(globals: PtrWeakConst<NetworkRuntimeGlobals>) -> Self {
        Self {
            core: ModuleCore::new_with(
                ObjectPath::root_module("RoutingDaemon".to_string()),
                globals,
            ),
            hop_counts: HashMap::new(),
        }
    }

    pub fn handle(&mut self, pkt: &Message, incoming: GateRef) {
        let source = pkt.header().src_addr.ip();
        if let Some(path_cost) = self.hop_counts.get_mut(&source) {
            // Allready knows path
            info!(target: "RandomRoutingDeamon", "Updating backproc path");
            if pkt.header().hop_count < *path_cost {
                *path_cost = pkt.header().hop_count;
                self.parent_mut_as::<super::network_stack::NetworkStack>()
                    .unwrap()
                    .add_route(source, incoming)
            }
        } else {
            // Does not know path
            info!(target: "RandomRoutingDeamon", "Recording new backproc path");
            self.hop_counts.insert(source, pkt.header().hop_count);
            self.parent_mut_as::<super::network_stack::NetworkStack>()
                .unwrap()
                .add_route(source, incoming);
        }
    }
}

impl Module for RandomRoutingDeamon {
    fn handle_message(&mut self, _msg: Message) {}
}
