use std::collections::HashMap;

use dse::{GateId, Module, ModuleCore, ModuleExt, NodeAddress, Packet};

pub struct RandomRoutingDeamon {
    core: ModuleCore,

    hop_counts: HashMap<NodeAddress, usize>,
}

impl RandomRoutingDeamon {
    pub fn new() -> Self {
        Self {
            core: ModuleCore::new(),
            hop_counts: HashMap::new(),
        }
    }

    pub fn handle(&mut self, pkt: &Packet, incoming: GateId) {
        let source = pkt.source_addr();
        if let Some(path_cost) = self.hop_counts.get_mut(&source) {
            // Allready knows path
            println!("111");
            if pkt.hop_count() < *path_cost {
                *path_cost = pkt.hop_count();
                self.parent_mut::<super::network_stack::NetworkStack>()
                    .unwrap()
                    .add_route(source, incoming)
            }
        } else {
            println!("222");
            // Does not know path
            self.hop_counts.insert(source, pkt.hop_count());
            println!("222# {} {:?}", self.has_parent(), self.core.parent_ptr);
            self.parent_mut::<super::network_stack::NetworkStack>()
                .unwrap()
                .add_route(source, incoming);
            println!("222#*");
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
