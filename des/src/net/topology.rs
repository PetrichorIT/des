use crate::net::gate::GateRef;
use std::fmt::Debug;
use std::sync::Arc;

use super::module::{ModuleId, ModuleRef};

///
/// A mapping of the runtimes modules, and their connections.
///
#[derive(Debug, Clone)]
pub struct Topology {
    // A mapping (index --> Module)
    nodes: Vec<TopoNode>,
}

impl Topology {
    ///
    /// Returns the number of nodes in the topology.
    ///
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    ///
    /// Indicates wether the topology is empty.
    ///
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    ///
    /// The full set of nodes in the topology.
    ///
    pub fn nodes(&self) -> impl Iterator<Item = &ModuleRef> {
        self.nodes.iter().map(|def| &def.node)
    }

    ///
    /// The complete set of edges defined per-source.
    ///
    pub fn edges(&self) -> impl Iterator<Item = &TopoEdge> {
        self.nodes.iter().flat_map(|def| def.edges.iter())
    }

    ///
    /// Returns the set of edges starting at the given node,
    /// or `None` if the nodes does not exist.
    ///
    #[must_use]
    pub fn edges_for(&self, node: &ModuleRef) -> Option<&Vec<TopoEdge>> {
        self.nodes
            .iter()
            .find(|def| def.node.ctx.id == node.ctx.id)
            .map(|def| &def.edges)
    }

    ///
    /// Returns the set of edges starting at the given node,
    /// or `None` if the nodes does not exist.
    ///
    #[must_use]
    pub fn edges_mut_for(&mut self, node: &ModuleRef) -> Option<&mut Vec<TopoEdge>> {
        self.nodes
            .iter_mut()
            .find(|def| def.node.ctx.id == node.ctx.id)
            .map(|def| &mut def.edges)
    }

    ///
    /// Creates a new empty instance of topology.
    ///
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    ///
    /// Uses the give modules and their associated gates to map out
    /// a new part of the network. Be aware that all nodes that will be found should
    /// either allready be build, or be in the given input `modules`.
    ///
    pub fn build(&mut self, modules: &[ModuleRef]) {
        let allready_existing_nodes_offset = self.nodes.len();

        // Setup nodes
        for module in modules {
            self.nodes.push(TopoNode {
                node: module.clone(),
                edges: Vec::new(),
            });
        }

        // created edges
        for (i, module) in modules.iter().enumerate() {
            // let module = &modules[i];
            let mut outgoing_edges = Vec::new();
            let gates = module.ctx.gates.read();

            for start in gates.iter() {
                let mut cost = 0.0;
                let mut current = Arc::clone(start);
                while let Some(next_gate) = current.next_gate() {
                    if let Some(channel) = current.channel() {
                        cost += channel.metrics().cost;
                    }
                    current = Arc::clone(&next_gate);

                    if current.owner().ctx.id != start.owner().ctx.id {
                        break;
                    }
                }

                if *current != **start {
                    outgoing_edges.push(TopoEdge {
                        src_gate: Arc::clone(start),
                        target_gate: current,
                        cost,
                    });
                }
            }

            self.nodes[allready_existing_nodes_offset + i].edges = outgoing_edges;
        }
    }

    ///
    /// Creates a new topology only containing nodes
    /// that conform to the predicate, pruning dangeling edges.
    ///
    #[must_use]
    pub fn filter_nodes<P>(&self, mut predicate: P) -> Self
    where
        P: FnMut(&ModuleRef) -> bool,
    {
        // Remove unwanted nodes
        let mut nodes: Vec<TopoNode> = self
            .nodes
            .iter()
            .filter(|e| predicate(&e.node))
            .cloned()
            .collect();

        let ids: Vec<ModuleId> = nodes.iter().map(|def| def.node.ctx.id).collect();

        for node in &mut nodes {
            // let node = &mut nodes[m];
            node.edges
                .retain(|edge| ids.contains(&edge.target_gate.owner().ctx.id));
        }

        Self { nodes }
    }

    ///
    /// Creates a new topology all previous nodes,
    /// but only edges that conform to the predicate.
    ///
    #[must_use]
    pub fn filter_edges<P>(&self, mut predicate: P) -> Self
    where
        P: FnMut(&ModuleRef, &TopoEdge) -> bool,
    {
        let mut nodes = self.nodes.clone();
        for def in &mut nodes {
            // let def = &mut nodes[m];
            def.edges.retain(|edge| predicate(&def.node, edge));
        }

        Self { nodes }
    }

    ///
    /// Creates a .dot output for visualizing the module graph.
    ///
    #[must_use]
    pub fn dot_output(&self) -> String {
        let mut nodes_out = String::new();
        for def in &self.nodes {
            nodes_out.push_str(&format!("    \"{}\" [shape=box]\n", def.node.as_str()));
        }

        let mut edges_out = String::new();
        for TopoNode { node, edges } in &self.nodes {
            let from_node = node.as_str();
            for TopoEdge {
                cost,
                src_gate,
                target_gate,
            } in edges
            {
                let owner = target_gate.owner();
                let to_node = owner.as_str();
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

    ///
    /// Writes the dot output to a *.dot file
    /// and converts this file into a svg.
    ///
    /// # Note
    ///
    /// Be aware that this command relies on the 'dot' command line
    /// programm to generate the svg.
    ///
    /// # Errors
    ///
    /// This operation will return an IO Error if
    /// either the file cannot be created or the operations
    /// using the dot engine wont work.
    ///
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

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}

///
/// A pre-source collection of outgoing edges.
/// All edges should share the same `src_gate.owner()`.
///
#[derive(Clone)]
pub struct TopoNode {
    ///
    /// A reference of the described node.
    ///
    pub node: ModuleRef,
    ///
    /// All edges, starting from this node.
    ///
    pub edges: Vec<TopoEdge>,
}

impl Debug for TopoNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDefinition")
            .field("node", &self.node.as_str())
            .field("edges", &self.edges)
            .finish()
    }
}

///
/// A single edge in the module graph.
///
#[derive(Debug, Clone, PartialEq)]
pub struct TopoEdge {
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
