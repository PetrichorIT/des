use std::collections::HashMap;

use dse::{GateId, Module, ModuleCore, ModuleExt, NodeAddress, Packet};
use log::info;

pub struct RandomRoutingDeamon {
    core: ModuleCore,

    hop_counts: HashMap<NodeAddress, usize>,
}

impl RandomRoutingDeamon {
    pub fn new() -> Self {
        Self {
            core: ModuleCore::new_with(Some(String::from("RoutingDaemon"))),
            hop_counts: HashMap::new(),
        }
    }

    pub fn handle(&mut self, pkt: &Packet, incoming: GateId) {
        let source = pkt.source_addr();
        if let Some(path_cost) = self.hop_counts.get_mut(&source) {
            // Allready knows path
            info!(target: "RandomRoutingDeamon", "Updating backproc path");
            if pkt.hop_count() < *path_cost {
                *path_cost = pkt.hop_count();
                self.parent_mut::<super::network_stack::NetworkStack>()
                    .unwrap()
                    .add_route(source, incoming)
            }
        } else {
            // Does not know path
            info!(target: "RandomRoutingDeamon", "Recording new backproc path");
            self.hop_counts.insert(source, pkt.hop_count());
            self.parent_mut::<super::network_stack::NetworkStack>()
                .unwrap()
                .add_route(source, incoming);
        }
    }
}

impl Module for RandomRoutingDeamon {
    fn module_core(&self) -> &ModuleCore {
        &self.core
    }

    fn module_core_mut(&mut self) -> &mut ModuleCore {
        &mut self.core
    }

    fn handle_message(&mut self, _msg: dse::Message) {}
}

impl ModuleExt for RandomRoutingDeamon {}
