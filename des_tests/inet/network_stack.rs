use std::collections::HashMap;

use des_core::{rng, GateId, Message, Module, ModuleCore, ModuleExt, NodeAddress, Packet, SimTime};
use log::info;

use crate::routing_deamon::RandomRoutingDeamon;

pub struct NetworkStack {
    core: ModuleCore,

    address: NodeAddress,

    forwarding_table: HashMap<NodeAddress, GateId>,
    routing_deamon: Option<Box<RandomRoutingDeamon>>,
}

impl NetworkStack {
    pub fn new(address: NodeAddress, mut router: RandomRoutingDeamon) -> Box<Self> {
        let mut obj = Box::new(Self {
            core: ModuleCore::new_with(Some(String::from("NetworkStack"))),
            address,
            forwarding_table: HashMap::new(),
            routing_deamon: None,
        });

        router.set_parent(&mut obj);
        obj.routing_deamon = Some(Box::new(router));
        obj
    }

    pub fn add_route(&mut self, target: NodeAddress, gate_id: GateId) {
        self.forwarding_table.insert(target, gate_id);
    }

    pub fn lookup_route(&mut self, target: NodeAddress) -> Option<&GateId> {
        self.forwarding_table.get(&target)
    }
}

impl Module for NetworkStack {
    fn module_core(&self) -> &ModuleCore {
        &self.core
    }

    fn module_core_mut(&mut self) -> &mut ModuleCore {
        &mut self.core
    }

    fn identifier(&self) -> String {
        format!("{} \"{:x}\"", self.core.identifier(), self.address)
    }

    fn handle_message(&mut self, msg: des_core::Message) {
        let incoming = msg.arrival_gate();
        let mut pkt = msg.extract_content::<Packet>();

        pkt.set_hop_count(pkt.hop_count() + 1);
        self.routing_deamon.as_mut().unwrap().handle(&pkt, incoming);

        // Route packet
        if pkt.target_addr() == self.address {
            info!(target: "Application Layer", "=== Received packet ===");
        } else if let Some(&route) = self.lookup_route(pkt.target_addr()) {
            // PATH ROUTE
            info!(target: "NetworkStack", "Routing over backproc path");
            let msg = Message::new_boxed(2, self.id(), SimTime::now(), pkt);
            self.send(msg, route);
        } else {
            // RANDOM ROUTE
            info!(target: "NetworkStack", "Routing random path");

            let out_size = self.gate("netOut", 0).unwrap().size();
            let idx = rng::<usize>() % out_size;

            let mut gate_id = self.gate("netOut", idx).unwrap().id();
            if gate_id == incoming {
                gate_id = self
                    .gate("netOut", (idx + 1) % self.gates().len())
                    .unwrap()
                    .id()
            }

            let msg = Message::new_boxed(2, self.id(), SimTime::now(), pkt);
            self.send(msg, gate_id);
        }
    }
}

impl ModuleExt for NetworkStack {}
