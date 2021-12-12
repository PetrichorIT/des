use std::collections::HashMap;

use dse::{rng, GateId, Message, Module, ModuleCore, ModuleExt, NodeAddress, Packet, SimTime};

use crate::routing_deamon::RandomRoutingDeamon;

pub struct NetworkStack {
    core: ModuleCore,

    forwarding_table: HashMap<NodeAddress, GateId>,

    routing_deamon: Option<Box<RandomRoutingDeamon>>,
}

impl NetworkStack {
    pub fn new(mut router: RandomRoutingDeamon) -> Box<Self> {
        let mut obj = Box::new(Self {
            core: ModuleCore::new(),
            forwarding_table: HashMap::new(),
            routing_deamon: None,
        });

        let ptr: *mut Box<NetworkStack> = &mut obj;
        println!("parent module # {:?}", ptr);
        let o = unsafe { &mut *ptr };
        println!("ID {}", o.id());

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

    fn handle_message(&mut self, msg: dse::Message) {
        let incoming = msg.arrival_gate();
        let mut pkt = msg.extract_content::<Packet>();

        pkt.set_hop_count(pkt.hop_count() + 1);

        println!("***");

        self.routing_deamon.as_mut().unwrap().handle(&pkt, incoming);

        println!("###");

        if let Some(&route) = self.lookup_route(pkt.target_addr()) {
            // PATH ROUTE
            let msg = Message::new_boxed(2, self.id(), SimTime::now(), pkt);
            self.send(msg, route);
        } else {
            // RANDOM ROUTE
            let gate_idx = rng::<usize>() % self.gates().len();
            let mut gate_id = self.gate("netOut", gate_idx).unwrap().id();
            if gate_id == incoming {
                gate_id = self
                    .gate("netOut", (gate_idx + 1) % self.gates().len())
                    .unwrap()
                    .id()
            }

            let msg = Message::new_boxed(2, self.id(), SimTime::now(), pkt);
            self.send(msg, gate_id);
        }
    }
}

impl ModuleExt for NetworkStack {}
