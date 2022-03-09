use std::collections::HashMap;

use des_core::{GateId, Module, ModuleCore, NodeAddress, Packet, Parameters, SpmcReader};
use des_core::{ModulePath, StaticModuleCore};
use des_macros::Module;
use log::info;

#[derive(Module)]
pub struct RandomRoutingDeamon {
    core: ModuleCore,

    hop_counts: HashMap<NodeAddress, usize>,
}

impl RandomRoutingDeamon {
    pub fn new(parameters: SpmcReader<Parameters>) -> Self {
        Self {
            core: ModuleCore::new_with(ModulePath::root("RoutingDaemon".to_string()), parameters),
            hop_counts: HashMap::new(),
        }
    }

    pub fn handle(&mut self, pkt: &Packet, incoming: GateId) {
        let source = pkt.header().source_node;
        if let Some(path_cost) = self.hop_counts.get_mut(&source) {
            // Allready knows path
            info!(target: "RandomRoutingDeamon", "Updating backproc path");
            if pkt.header().hop_count < *path_cost {
                *path_cost = pkt.header().hop_count;
                self.parent_mut::<super::network_stack::NetworkStack>()
                    .unwrap()
                    .add_route(source, incoming)
            }
        } else {
            // Does not know path
            info!(target: "RandomRoutingDeamon", "Recording new backproc path");
            self.hop_counts.insert(source, pkt.header().hop_count);
            self.parent_mut::<super::network_stack::NetworkStack>()
                .unwrap()
                .add_route(source, incoming);
        }
    }
}

impl Module for RandomRoutingDeamon {
    fn handle_message(&mut self, _msg: des_core::Message) {}
}
