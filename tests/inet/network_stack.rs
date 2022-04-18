use std::collections::HashMap;

use des::prelude::*;
use des_derive::Module;
use log::info;

use crate::routing_deamon::RandomRoutingDeamon;

#[derive(Module)]
pub struct NetworkStack {
    core: ModuleCore,

    address: NodeAddress,

    forwarding_table: HashMap<NodeAddress, GateRef>,
    routing_deamon: Option<Mrc<RandomRoutingDeamon>>,
}

impl NetworkStack {
    pub fn new(name: &str, address: NodeAddress, router: RandomRoutingDeamon) -> Mrc<Self> {
        let mut obj = Mrc::new(Self {
            core: ModuleCore::new_with(
                ModulePath::root(name.to_string()),
                router.module_core().globals(),
            ),
            address,
            forwarding_table: HashMap::new(),
            routing_deamon: None,
        });

        let mut router = Mrc::new(router);

        obj.add_child(&mut router);
        obj.routing_deamon = Some(router);
        obj
    }

    pub fn add_route(&mut self, target: NodeAddress, gate_id: GateRef) {
        self.forwarding_table.insert(target, gate_id);
    }

    pub fn lookup_route(&mut self, target: NodeAddress) -> Option<&GateRef> {
        self.forwarding_table.get(&target)
    }
}

impl Module for NetworkStack {
    fn handle_message(&mut self, msg: Message) {
        let (mut pkt, meta) = msg.cast::<Packet>();

        pkt.register_hop();
        self.routing_deamon
            .as_mut()
            .unwrap()
            .handle(&pkt, meta.last_gate.clone().unwrap());

        // Route packet
        if pkt.header().dest_node == self.address {
            info!(target: "Application Layer", "=== Received packet ===");
        } else if let Some(route) = self.lookup_route(pkt.header().dest_node) {
            let route = route.clone();

            // PATH ROUTE
            info!(target: "NetworkStack", "Routing over backproc path");
            let msg = Message::new()
                .kind(2)
                .timestamp(SimTime::now())
                .content(pkt)
                .build();
            // let msg = Message::legacy_new_interned(0, 2, self.id(), SimTime::now(), pkt);
            self.send(msg, route);
        } else {
            // RANDOM ROUTE
            info!(target: "NetworkStack", "Routing random path");

            let out_size = self.gate("netOut", 0).unwrap().size();
            let idx = rng::<usize>() % out_size;

            let mut gate_id = self.gate("netOut", idx).unwrap();
            if gate_id == meta.last_gate.unwrap() {
                gate_id = self.gate("netOut", (idx + 1) % self.gates().len()).unwrap()
            }

            let msg = Message::new()
                .kind(2)
                .timestamp(SimTime::now())
                .content(pkt)
                .build();
            // let msg = Message::legacy_new_interned(0, 2, self.id(), SimTime::now(), pkt);
            self.send(msg, gate_id);
        }
    }
}
