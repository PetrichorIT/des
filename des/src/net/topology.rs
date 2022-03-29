use crate::net::*;
use crate::util::*;
use std::fmt::Debug;

///
/// A mapping of the runtimes modules, and their connections.
///
#[derive(Debug)]
pub struct Topology {
    // A mapping (index --> Module)
    nodes: Vec<NodeDefinition>,
}

impl Topology {
    ///
    /// The full set of nodes in the topology.
    ///
    pub fn nodes(&self) -> impl Iterator<Item = &ModuleRefMut> {
        self.nodes.iter().map(|def| &def.node)
    }

    ///
    /// The complete set of edges defined per-source.
    ///
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.nodes.iter().map(|def| def.edges.iter()).flatten()
    }

    ///
    /// Returns the set of edges starting at the given node,
    /// or `None` if the nodes does not exist.
    ///
    pub fn edges_for(&self, node: &ModuleRefMut) -> Option<&Vec<Edge>> {
        self.nodes
            .iter()
            .find(|def| def.node.id() == node.id())
            .map(|def| &def.edges)
    }

    ///
    /// Creates a new empty instance of topology.
    ///
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    ///
    /// Uses the give modules and their associated gates to map out
    /// a new part of the network. Be aware that all nodes that will be found should
    /// either allready be build, or be in the given input `modules`.
    ///
    pub fn build(&mut self, modules: &[ModuleRefMut]) {
        let allready_existing_nodes_offset = self.nodes.len();

        // Setup nodes
        for module in modules {
            self.nodes.push(NodeDefinition {
                node: MrcS::clone(module),
                edges: Vec::new(),
            });
        }

        // created edges
        for i in 0..modules.len() {
            let module = &modules[i];
            let mut outgoing_edges = Vec::new();
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
                    outgoing_edges.push(Edge {
                        src_gate: MrcS::clone(start).make_readonly(),
                        target_gate: current,
                        cost,
                    })
                }
            }

            self.nodes[allready_existing_nodes_offset + i].edges = outgoing_edges;
        }
    }

    ///
    /// Creates a new topology only containing nodes
    /// that conform to the predicate, pruning dangeling edges.
    ///
    pub fn filter_nodes<P>(&self, mut predicate: P) -> Self
    where
        P: FnMut(&ModuleRefMut) -> bool,
    {
        // Remove unwanted nodes
        let mut nodes: Vec<NodeDefinition> = self
            .nodes
            .iter()
            .filter(|e| predicate(&e.node))
            .cloned()
            .collect();

        let ids: Vec<ModuleId> = nodes.iter().map(|def| def.node.id()).collect();

        for m in 0..nodes.len() {
            let node = &mut nodes[m];

            node.edges = node
                .edges
                .iter()
                .filter(|edge| ids.contains(&edge.target_gate.owner().id()))
                .cloned()
                .collect();
        }

        Self { nodes }
    }

    ///
    /// Creates a new topology all previous nodes,
    /// but only edges that conform to the predicate.
    ///
    pub fn filter_edges<P>(&self, mut predicate: P) -> Self
    where
        P: FnMut(&ModuleRefMut, &Edge) -> bool,
    {
        let mut nodes = self.nodes.clone();
        for m in 0..nodes.len() {
            let def = &mut nodes[m];

            def.edges = def
                .edges
                .iter()
                .filter(|edge| predicate(&def.node, edge))
                .cloned()
                .collect();
        }

        Self { nodes }
    }

    ///
    /// Creates a .dot output for visualizing the module graph.
    ///
    pub fn dot_output(&self) -> String {
        let mut nodes_out = String::new();
        for def in self.nodes.iter() {
            nodes_out.push_str(&format!("    \"{}\" [shape=box]\n", def.node.str()))
        }

        let mut edges_out = String::new();
        for NodeDefinition { node, edges } in self.nodes.iter() {
            let from_node = node.str();
            for Edge {
                cost,
                src_gate,
                target_gate,
            } in edges
            {
                let to_node = target_gate.owner().str();
                edges_out.push_str(&format!(
                    "    \"{}\" -> \"{}\" [ headlabel=\"{}\" {} taillabel=\"{}\" ]\n",
                    from_node,
                    to_node,
                    target_gate.str(),
                    if *cost == 0.0 {
                        String::new()
                    } else {
                        format!("label=\"{}\"", cost)
                    },
                    src_gate.str()
                ));
            }
        }

        format!("digraph D {{\n{}\n{}\n}}", nodes_out, edges_out)
    }

    pub fn write_to_svg(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;
        use std::process::Command;
        let str = self.dot_output();
        let mut file = File::create(format!("{}.dot", path))?;
        write!(file, "{}", str)?;

        let output = Command::new("dot")
            .arg("-Tsvg")
            .arg(format!("{}.dot", path))
            .output()?;

        let mut file = File::create(format!("{}.svg", path))?;
        write!(file, "{}", String::from_utf8_lossy(&output.stdout))?;

        Ok(())
    }
}

///
/// A pre-source collection of outgoing edges.
/// All edges should share the same `src_gate.owner()`.
///
#[derive(Clone)]
pub struct NodeDefinition {
    pub node: ModuleRefMut,
    pub edges: Vec<Edge>,
}

impl Debug for NodeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDefinition")
            .field("node", &self.node.str())
            .field("edges", &self.edges)
            .finish()
    }
}

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
