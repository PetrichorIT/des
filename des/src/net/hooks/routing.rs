use std::{
    any::Any,
    collections::{BinaryHeap, HashMap},
    net::IpAddr,
};

use super::Hook;
use crate::{
    net::globals,
    prelude::{module_id, send, GateRef, Message, ModuleId, ModuleRef},
};

/// options for configuring the routing hook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoutingHookOptions {
    /// If this flag is set all TCP / UDP packtes will be routed
    /// based on the `dest_node`. If no destionation was set the packet
    /// will NOT be consumed by the hook.
    ///
    /// If no route was found, the packet will NOT be consumed by the hook.
    #[cfg(feature = "async")]
    pub route_tcp_udp: bool,
    /// If this flag is set all packtes will be routed
    /// based on the `target_module_id`. If no destionation was set the packet
    /// will NOT be consumed by the hook.
    ///
    /// If no route was found, the packet will NOT be consumed by the hook.
    /// This routing mechanism is secondary to TCP / UDP routing.
    pub route_module_id: bool,
    /// If this flag is set, the router will decrease the ttl of all arriving
    /// routed packets, dropping those with a ttl == 0. This will only
    /// affect packets that would have been routed by the hook.
    pub ttl: bool,
}

impl RoutingHookOptions {
    /// A config used for most internet cases
    #[cfg(feature = "async")]
    pub const INET: RoutingHookOptions = RoutingHookOptions {
        route_tcp_udp: true,
        route_module_id: false,
        ttl: true,
    };
}

/// A hook that provides routing abilities
#[derive(Debug)]
pub struct RoutingHook {
    opts: RoutingHookOptions,

    #[cfg(feature = "async")]
    addr: Option<IpAddr>,

    mod_id_fwd: HashMap<ModuleId, GateRef>,
    tcp_udp_fwd: HashMap<IpAddr, GateRef>,
}

impl RoutingHook {
    ///
    /// Creates a new routing hook for either
    /// an endsystem or a router.
    ///
    pub fn new(opts: RoutingHookOptions) -> Self {
        let mut this = Self {
            opts,
            mod_id_fwd: HashMap::new(),
            tcp_udp_fwd: HashMap::new(),

            #[cfg(feature = "async")]
            addr: tokio::net::get_ip(),
        };

        this.create_routing_table_direct();
        this
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
            #[cfg(feature = "async")]
            if self.opts.route_tcp_udp {
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
                    if self.tcp_udp_fwd.get(&ip).is_none() {
                        self.tcp_udp_fwd.insert(ip, cur.gate.clone());
                    }
                }
            }

            if self.opts.route_module_id {
                if self.mod_id_fwd.get(&cur.module.ctx.id()).is_none() {
                    self.mod_id_fwd
                        .insert(cur.module.ctx.id(), cur.gate.clone());
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
            "<RoutingHook> Created routing table with {}/{} destinations",
            self.tcp_udp_fwd.len(),
            self.mod_id_fwd.len()
        );
    }

    #[cfg(feature = "async")]
    fn route_tcp(&self, msg: Message) -> Result<(), Message> {
        use crate::net::MessageType;

        if matches!(msg.header().typ(), MessageType::Tcp | MessageType::Udp) {
            if msg.header().dest_addr.ip().is_unspecified() {
                return Err(msg);
            }

            if msg.header().dest_addr.ip().is_loopback() {
                return Err(msg);
            }

            if Some(msg.header().dest_addr.ip()) == self.addr {
                return Err(msg);
            }

            // if not
            if let Some(path) = self.tcp_udp_fwd.get(&msg.header().dest_addr.ip()) {
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

    fn route_module_id(&self, msg: Message) -> Result<(), Message> {
        if msg.header().receiver_module_id == ModuleId::NULL {
            return Err(msg);
        }

        // if not
        if let Some(path) = self.mod_id_fwd.get(&msg.header().receiver_module_id) {
            log::trace!(
                "<RoutingHook> Forwarding a packet from #{} to #{} via {}",
                msg.header().sender_module_id,
                msg.header.receiver_module_id,
                path.path()
            );
            send(msg, path);
            Ok(())
        } else {
            log::error!("Could not route packet");
            Err(msg)
        }
    }
}

impl Hook for RoutingHook {
    fn state(&self) -> &dyn Any {
        &self.tcp_udp_fwd
    }

    fn handle_message(&mut self, mut msg: Message) -> Result<(), Message> {
        // TTL check
        if self.opts.ttl {
            msg.register_hop();
            if msg.header().ttl == 0 {
                log::debug!("<RoutingHook> Dropped packet because TTL reached zero");
                return Ok(());
            }
        }

        #[cfg(feature = "async")]
        let msg = if self.opts.route_tcp_udp {
            if let Err(msg) = self.route_tcp(msg) {
                msg
            } else {
                return Ok(());
            }
        } else {
            msg
        };

        let msg = if self.opts.route_module_id {
            if let Err(msg) = self.route_module_id(msg) {
                msg
            } else {
                return Ok(());
            }
        } else {
            msg
        };

        Err(msg)
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
