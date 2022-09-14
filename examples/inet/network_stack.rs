use std::collections::HashMap;

use des::prelude::*;
use log::info;

use crate::routing_deamon::RandomRoutingDeamon;

#[NdlModule]
pub struct NetworkStack {
    core: ModuleCore,

    address: IpAddr,

    forwarding_table: HashMap<IpAddr, GateRef>,
    routing_deamon: Option<PtrMut<RandomRoutingDeamon>>,
}

impl NetworkStack {
    pub fn new(name: &str, address: IpAddr, router: RandomRoutingDeamon) -> PtrMut<Self> {
        let mut obj = PtrMut::new(Self {
            core: ModuleCore::new_with(
                ObjectPath::new(name.to_string()).unwrap(),
                router.module_core().globals(),
            ),
            address,
            forwarding_table: HashMap::new(),
            routing_deamon: None,
        });

        let mut router = PtrMut::new(router);

        obj.add_child(&mut router);
        obj.routing_deamon = Some(router);
        obj
    }

    pub fn add_route(&mut self, target: IpAddr, gate_id: GateRef) {
        self.forwarding_table.insert(target, gate_id);
    }

    pub fn lookup_route(&mut self, target: IpAddr) -> Option<&GateRef> {
        self.forwarding_table.get(&target)
    }
}

impl Module for NetworkStack {
    fn handle_message(&mut self, msg: Message) {
        // let (mut pkt, meta) = msg.cast::<Packet>();
        let mut pkt = msg;

        pkt.register_hop();
        self.routing_deamon
            .as_mut()
            .unwrap()
            .handle(&pkt, pkt.header().last_gate.clone().unwrap());

        // Route packet
        if pkt.header().dest_addr.ip() == self.address {
            info!(target: "Application Layer", "=== Received packet ===");
        } else if let Some(route) = self.lookup_route(pkt.header().dest_addr.ip()) {
            let route = route.clone();

            // PATH ROUTE
            info!(target: "NetworkStack", "Routing over backproc path");
            // let msg = Message::legacy_new_interned(0, 2, self.id(), SimTime::now(), pkt);
            self.send(pkt, route);
        } else {
            // RANDOM ROUTE
            info!(target: "NetworkStack", "Routing random path");

            let out_size = self.gate("netOut", 0).unwrap().size();
            let idx = random::<usize>() % out_size;

            let mut gate_id = self.gate("netOut", idx).unwrap();
            if gate_id == *pkt.header().last_gate.as_ref().unwrap() {
                gate_id = self.gate("netOut", (idx + 1) % self.gates().len()).unwrap()
            }

            // let msg = Message::new()
            //     .kind(2)
            //     .timestamp(SimTime::now())
            //     .content(pkt)
            //     .build();
            // let msg = Message::legacy_new_interned(0, 2, self.id(), SimTime::now(), pkt);
            self.send(pkt, gate_id);
        }
    }
}
