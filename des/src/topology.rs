//! Graph-based tooling for exploring simulation topology.
use crate::{
    gate::GateKind,
    prelude::{GateRef, ModuleRef},
    runtime::Globals,
};
use petgraph::{
    Graph, Undirected,
    graph::{EdgeReference, UnGraph},
};
use std::sync::Arc;

/// A network topology.
pub type Topology = Graph<ModuleRef, EdgeRef, Undirected, u32>;

/// The node type for the topology graph, containing a module reference.
pub type NodeRef = ModuleRef;

/// The edge type for the topology graph, containing source and target gate of
/// a gate chain. Since the graph is undirected, the source and target are
/// interchangeable in regards to `PartialEq`.
#[derive(Debug, Clone, Eq)]
pub struct EdgeRef {
    /// The source of the gate chain.
    pub source: GateRef,
    /// The target of the gate chain.
    pub target: GateRef,
}

impl PartialEq for EdgeRef {
    fn eq(&self, other: &Self) -> bool {
        let direct =
            Arc::ptr_eq(&self.source, &other.source) && Arc::ptr_eq(&self.target, &other.target);
        let reverse =
            Arc::ptr_eq(&self.source, &other.target) && Arc::ptr_eq(&self.target, &other.source);
        direct || reverse
    }
}

impl Globals {
    /// Extracts the topology of the network from its current state.
    ///
    /// This function constructs a graph over all nodes known to the simulation.
    /// It does not perform any spanning-tree like checks to find hidden nodes.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn topology(&self) -> Topology {
        self.with(|modules| {
            let mut graph = UnGraph::new_undirected();

            for module in modules.nodes() {
                graph.add_node(module);
            }

            for node_index in graph.node_indices() {
                let node = graph.node_weight(node_index).expect("illegal state");
                for gate in node.gates() {
                    if let GateKind::Endpoint = gate.kind() {
                        let path_end = gate
                            .path_end()
                            .expect("cannot be empty, since this is an endpoint");

                        let end_node = path_end.owner();
                        let Some(end_index) = graph
                            .node_indices()
                            .find(|idx| graph.node_weight(*idx).unwrap() == &end_node)
                        else {
                            continue;
                        };

                        // check whether to add the edge or its already done
                        let exist =
                            graph
                                .edges(node_index)
                                .any(|edge: EdgeReference<'_, EdgeRef>| {
                                    Arc::ptr_eq(&edge.weight().source, &path_end)
                                        && Arc::ptr_eq(&edge.weight().target, &gate)
                                });
                        if exist {
                            continue;
                        }

                        graph.add_edge(
                            node_index,
                            end_index,
                            EdgeRef {
                                source: gate,
                                target: path_end,
                            },
                        );
                    }
                }
            }

            graph
        })
    }
}

impl ModuleRef {
    /// Extracts a topology from the spanning tree, starting from this module.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn spanning_tree(&self) -> Topology {
        let mut graph = UnGraph::new_undirected();
        let mut queue = vec![graph.add_node(self.clone())];

        while let Some(module_index) = queue.pop() {
            let module = graph.node_weight(module_index).expect("illegal state");
            let gates = module.gates();

            for gate in gates {
                if gate.kind() == GateKind::Endpoint {
                    let path_end = gate.path_end().expect("should exist on an endpoint");
                    let end_node = path_end.owner();

                    let end_index = graph
                        .node_indices()
                        .find(|&idx| graph.node_weight(idx).unwrap() == &end_node)
                        .unwrap_or_else(|| {
                            // New node found
                            let index = graph.add_node(end_node);
                            queue.push(index);
                            index
                        });

                    graph.add_edge(
                        module_index,
                        end_index,
                        EdgeRef {
                            source: gate,
                            target: path_end,
                        },
                    );
                }
            }
        }
        graph
    }
}
