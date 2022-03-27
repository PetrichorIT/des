use crate::net::*;
use crate::util::*;
use std::fmt::Debug;

///
/// A mapping of the runtimes modules, and their connections.
///
pub struct Topology {
    // A mapping (index --> Module)
    nodes: Vec<ModuleRefMut>,
    // A mapping (index (srcNode) --> ListOutgoingEdges = List<(cost, target_index)>)
    edges: Vec<OutgoingEdges>,
}

impl Topology {
    ///
    /// The full set of nodes in the topology.
    ///
    pub fn nodes(&self) -> &Vec<ModuleRefMut> {
        &self.nodes
    }

    ///
    /// The complete set of edges defined per-source.
    ///
    pub fn edges(&self) -> &Vec<OutgoingEdges> {
        &self.edges
    }

    ///
    /// Creates a new empty instance of topology.
    ///
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    ///
    /// Uses the give modules and their associated gates to map out
    /// a new part of the network. Be aware that all nodes that will be found should
    /// either allready be build, or be in the given input `modules`.
    ///
    pub fn build(&mut self, modules: &[ModuleRefMut]) {
        // Setup nodes
        for module in modules {
            self.nodes.push(MrcS::clone(module));
        }

        // created edges
        for i in 0..modules.len() {
            let module = &modules[i];
            let mut outgoing_edges = OutgoingEdges(Vec::new());
            let gates = module.gates();

            for start in gates {
                let mut cost = 0.0;
                let mut current = MrcS::clone(start).make_readonly();
                while let Some(next_gate) = current.next_gate() {
                    if let Some(_channel) = current.channel() {
                        // TODO: Compute channel cost
                        cost += 1.0;
                    }
                    current = MrcS::clone(next_gate);
                }

                if *current != **start {
                    outgoing_edges.0.push(Edge {
                        src_gate: MrcS::clone(start).make_readonly(),
                        target_gate: current,
                        cost,
                    })
                }
            }

            match self.edges.get_mut(i) {
                Some(outgoing) => *outgoing = outgoing_edges,
                None => {
                    assert_eq!(self.edges.len(), i);
                    self.edges.push(outgoing_edges)
                }
            }
        }
    }

    ///
    /// Creates a .dot output for visualizing the module graph.
    ///
    pub fn dot_output(&self) -> String {
        let mut nodes = String::new();
        for node in self.nodes.iter() {
            nodes.push_str(&format!("    \"{}\" [shape=circle]\n", node.str()))
        }

        let mut edges = String::new();
        for (idx, outgoing) in self.edges.iter().enumerate() {
            let from_node = self.nodes[idx].str();
            for Edge {
                cost,
                src_gate,
                target_gate,
            } in &outgoing.0
            {
                let to_node = target_gate.owner().str();
                edges.push_str(&format!(
                    "    \"{}\" -> \"{}\" [ headlabel=\"{}\" {} taillabel=\"{}\" ]\n",
                    from_node,
                    to_node,
                    target_gate.name(),
                    if *cost == 0.0 {
                        String::new()
                    } else {
                        format!("label=\"{}\"", cost)
                    },
                    src_gate.name()
                ));
            }
        }

        format!("digraph D {{\n{}\n{}\n}}", nodes, edges)
    }
}

impl Debug for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d: Vec<(usize, &str)> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(idx, m)| (idx, m.str()))
            .collect();
        f.debug_struct("Topology")
            .field("nodes", &d)
            .field("edges", &self.edges)
            .finish()
    }
}

///
/// A pre-source collection of outgoing edges.
/// All edges should share the same `src_gate.owner()`.
///
#[derive(Debug, Clone, PartialEq)]
pub struct OutgoingEdges(pub Vec<Edge>);

///
/// A single edge in the module graph.
///
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    ///
    /// The start point of the connection. This gate should be
    /// called with `send(..., thisgate)`.
    ///
    pub src_gate: GateRef,

    ///
    /// The end point of the connection.
    ///
    pub target_gate: GateRef,

    ///
    /// The cost of the edge. Cost accumulates through all transveresd channels.
    ///
    pub cost: f64,
}
