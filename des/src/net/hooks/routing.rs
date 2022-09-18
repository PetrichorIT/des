use std::{
    any::Any,
    collections::{BinaryHeap, HashMap},
    net::{IpAddr, Ipv4Addr},
};

use tokio::net::get_ip;

use super::Hook;
use crate::{
    net::globals,
    prelude::{module_id, send, GateRef, Message, MessageType, ModuleRef},
};

/// A hook that provides routing abilities
#[derive(Debug)]
pub struct RoutingHook {
    // opts
    is_router: bool,
    addr: IpAddr,
    fwd: HashMap<IpAddr, GateRef>,
}

impl RoutingHook {
    ///
    /// Creates a new routing hook for either
    /// an endsystem or a router.
    ///
    pub fn new(is_router: bool) -> Self {
        if is_router {
            let mut this = Self {
                is_router,
                addr: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                fwd: HashMap::new(),
            };

            this.create_routing_table_direct();
            this
        } else {
            Self {
                is_router,
                addr: get_ip().expect("Failed to fetch valid ip from tokio-sim context"),
                fwd: HashMap::new(),
            }
        }
    }

    fn create_routing_table_direct(&mut self) {
        let topo = globals().topology.borrow().clone();
        let start = topo
            .nodes()
            .find(|n| n.id() == module_id())
            .expect("Topology does not seem to be up to date");

        let mut active = BinaryHeap::new();
        let mut visited = Vec::new();

        visited.push(start.clone());

        for inital_edge in topo.edges_for(&start).unwrap() {
            active.push(DNode {
                module: inital_edge.target_gate.owner(),
                gate: inital_edge.src_gate.clone(),
                cost: inital_edge.cost,
            })
        }

        while let Some(cur) = active.pop() {
            // let the ip of the node
            if let Some(ip) = cur
                .module
                .ctx
                .async_ext
                .borrow_mut()
                .ctx
                .as_mut()
                .map(|ctx| ctx.io.as_mut().map(|io| io.get_ip()))
                .flatten()
                .flatten()
            {
                // seach for entry
                if self.fwd.get(&ip).is_none() {
                    self.fwd.insert(ip, cur.gate.clone());
                }
            }

            visited.push(cur.module.clone());
            for edge in topo.edges_for(&cur.module).unwrap() {
                let t = edge.target_gate.owner();
                if !visited.iter().any(|v| *v == t) {
                    active.push(DNode {
                        module: t,
                        gate: cur.gate.clone(),
                        cost: cur.cost + edge.cost,
                    })
                }
            }
        }

        log::trace!(
            "<RoutingHook> Created routing table with {} destinations",
            self.fwd.len()
        );
    }
}

impl Hook for RoutingHook {
    fn state(&self) -> &dyn Any {
        &self.fwd
    }

    fn handle_message(&mut self, msg: Message) -> Result<(), Message> {
        if matches!(msg.header().typ(), MessageType::Tcp | MessageType::Udp) {
            if msg.header().dest_addr.ip().is_unspecified() {
                return Err(msg);
            }

            if msg.header().dest_addr.ip().is_loopback() {
                return Err(msg);
            }

            // This should have been catched by the IOContext
            // so if not ... probably a lost packet
            // fwd to handle_message anyways
            if msg.header().dest_addr.ip() == self.addr && !self.is_router {
                return Err(msg);
            }

            // if not
            if let Some(path) = self.fwd.get(&msg.header().dest_addr.ip()) {
                log::trace!(
                    "<RoutingHook> Forwarding a packet from {} to {} via {}",
                    msg.header().src_addr,
                    msg.header.dest_addr,
                    path.path()
                );
                send(msg, path);
                Ok(())
            } else {
                log::error!("Could not route packet");
                Err(msg)
            }
        } else {
            Err(msg)
        }
    }
}

#[derive(Debug, PartialEq)]
struct DNode {
    module: ModuleRef,
    gate: GateRef,
    cost: f64,
}

impl Eq for DNode {}

impl PartialOrd for DNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.partial_cmp(&other.cost).unwrap()
    }
}

// fn dijkstra(topo: &Topology, start: &ModuleRef, target: &ModuleRef) -> Option<GateRef> {
//     let mut active_nodes = BinaryHeap::new();
//     let mut visited = Vec::new();

//     for inital_edge in topo.edges_for(&start).unwrap() {
//         active_nodes.push(DNode {
//             module: inital_edge.target_gate.owner(),
//             gate: inital_edge.src_gate.clone(),
//             cost: inital_edge.cost,
//         })
//     }

//     while let Some(cur) = active_nodes.pop() {
//         visited.push(cur.module.clone());
//         if cur.module == *target {
//             return Some(cur.gate);
//         }

//         let edges = topo.edges_for(&cur.module).unwrap();
//         for edge in edges {
//             let t = edge.target_gate.owner();
//             if !visited.iter().any(|n| *n == t) {
//                 active_nodes.push(DNode {
//                     module: edge.target_gate.owner().clone(),
//                     gate: cur.gate.clone(),
//                     cost: cur.cost + edge.cost,
//                 });
//             }
//         }
//     }

//     None
// }
