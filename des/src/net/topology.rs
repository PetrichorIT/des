use std::sync::Arc;

use crate::{net::module::ModuleRef, prelude::GateRef};

///
/// A mapping of all connections in a module connection graph.
///
#[derive(Debug, Clone)]
pub struct Topology {
    nodes: Vec<TopoNode>,
    edges: Vec<Vec<TopoEdge>>,
}

///
/// A node in the module connection graph, representing a module and its
/// connection state.
///
#[derive(Debug, Clone)]
pub struct TopoNode {
    /// A reference to the module itself, including its custom state.
    pub module: ModuleRef,
    /// The number of incoming connections. (NOT nessicarily the number of input gates)
    pub incount: usize,
    /// The number of outgoing connections. (nessicarily the number of input gates)
    pub outcount: usize,
    /// An indicate whether the module is at all connected to the rest of of the network.
    pub alive: bool,
}

///
/// A connection in the module connection graph.
///
#[derive(Debug, Clone)]

pub struct TopoEdge {
    /// The start of the gate chain.
    pub src: GateRef,
    /// The end of the gate chain.
    pub dst: GateRef,
    /// The node-id of the destination node.
    pub dst_id: usize,
    /// The accumulated cost according to the channel metrics.
    pub cost: f64,
}

impl Topology {
    /// Creates a new empty instance.
    #[must_use]
    pub const fn new() -> Topology {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// All nodes if the current topology.
    #[must_use]
    pub fn nodes(&self) -> &[TopoNode] {
        &self.nodes
    }

    /// An iterator over all edges in the entries network, annotated with the
    /// node-id of the starting node.
    pub fn edges(&self) -> impl Iterator<Item = (usize, &TopoEdge)> {
        self.edges
            .iter()
            .enumerate()
            .flat_map(|(i, edges)| edges.iter().map(move |e| (i, e)))
    }

    /// All outgoing edges associated with a single
    /// module.
    #[must_use]
    pub fn edges_for(&self, i: usize) -> &[TopoEdge] {
        &self.edges[i]
    }

    /// Adds the given modules and their connections to the connection graph.
    ///
    /// Note that this only adds connections withing the new conenction set,
    /// or from the new set to the old set.
    /// To add connections from the old set to the new one, recreate
    /// the topology from the ground up using the `ModuleRef` stored in the
    /// node information.
    pub fn build(&mut self, modules: &[ModuleRef]) {
        let offset = self.nodes.len();

        for module in modules {
            self.nodes.push(TopoNode {
                module: module.clone(),
                incount: 0,
                outcount: 0,
                alive: false,
            });
            self.edges.push(Vec::new());
        }

        for (i, module) in modules.iter().enumerate() {
            let mut outgoing = Vec::new();
            let gates = module.ctx.gates.read();

            'outer: for gate in &*gates {
                // Ingore path if we are not at the path start.
                if gate.previous_gate().is_some() {
                    continue;
                }

                let mut cost = 0.0;
                let mut cur = gate.clone();
                let mut itr = 0;

                while let Some(next) = cur.next_gate() {
                    if let Some(ch) = cur.channel() {
                        cost += ch.metrics().cost;
                    }
                    cur = next;

                    // Set alive notes
                    let transit_id = cur.owner().id;
                    let Some(transit) = self.nodes.iter_mut().find(|n| n.module.ctx.id == transit_id) else {
                        break 'outer;
                    };
                    transit.alive |= true;

                    // CHANGE:
                    // No longer break once another module was reached,
                    // only break at the end of a gate chain.

                    // CHANGE:
                    // Add a breaking condition to ensure termination
                    // even in misconfigured simulations.
                    itr += 1;
                    if itr > 16 {
                        break;
                    }
                }

                if !Arc::ptr_eq(&cur, gate) {
                    // Featch the topo info for faster algorithms later on
                    let dst_id = cur.owner().ctx.id;
                    let Some((id, dst)) = self
                        .nodes
                        .iter_mut()
                        .enumerate()
                        .find(|(_, n)| n.module.ctx.id == dst_id) else {
                            continue;
                        };

                    dst.incount += 1;
                    dst.alive |= true;

                    outgoing.push(TopoEdge {
                        dst_id: id,
                        src: gate.clone(),
                        dst: cur,
                        cost,
                    });
                }
            }

            self.nodes[offset + i].alive |= !outgoing.is_empty();
            self.nodes[offset + i].outcount = outgoing.len();
            self.edges[offset + i] = outgoing;
        }
    }

    ///
    /// Filters out nodes that do not comply with the given predicate.
    ///
    /// Note that this may change node-ids so all previouisly
    /// compiled information that relies on node-ids is to be considered
    /// invalid.
    ///
    pub fn filter_nodes<P>(&mut self, predicate: P)
    where
        P: FnMut(&TopoNode) -> bool,
    {
        let keeps = self.nodes.iter().map(predicate).collect::<Vec<_>>();
        let n = keeps.len();

        for (i, &keep) in keeps.iter().enumerate() {
            if keep {
                // Do nothing this node will be kept so no links must be pruned
            } else {
                // Remove outgoing edges
                for edge in self.edges[i].drain(..) {
                    self.nodes[edge.dst_id].incount -= 1;
                    self.nodes[i].outcount -= 1;
                }
                // Remove incoming edges
                for j in 0..n {
                    if j == i {
                        continue;
                    }

                    let mut k = 0;
                    while k < self.edges[j].len() {
                        if self.edges[j][k].dst_id == i {
                            self.nodes[i].incount -= 1;
                            self.nodes[j].outcount -= 1;
                            self.edges[j].remove(k);
                        } else {
                            k += 1;
                        }
                    }
                }
                debug_assert_eq!(self.nodes[i].outcount, 0);
                debug_assert_eq!(self.nodes[i].incount, 0);
            }
        }

        // Remove elements
        let mut ptr = 0;
        let mut mapping = (0..n).collect::<Vec<_>>();
        for (i, keep) in keeps.into_iter().enumerate() {
            if keep {
                mapping[i] = ptr;
                ptr += 1;
            } else {
                self.nodes.remove(ptr);
                self.edges.remove(ptr);
                mapping[i] = usize::MAX;
            }
        }

        // Update edge ids
        for edges in &mut self.edges {
            for edge in edges {
                edge.dst_id = mapping[edge.dst_id];
            }
        }
    }

    ///
    /// Filters out edges that do not comply with the given predicate.
    ///
    /// Note that this does NOT change node-ids, but may change the
    /// alive-flag on nodes, as well as in-/outcounts.
    ///
    pub fn filter_edges<P>(&mut self, mut predicate: P)
    where
        P: FnMut(&TopoEdge) -> bool,
    {
        for i in 0..self.edges.len() {
            let mut j = 0;
            while j < self.edges[i].len() {
                let keep = predicate(&self.edges[i][j]);
                if keep {
                    j += 1;
                } else {
                    self.edges[i].remove(j);
                    self.nodes[i].outcount -= 1;
                    self.nodes[self.edges[i][j].dst_id].incount -= 1;
                }
            }
        }

        for node in &mut self.nodes {
            if node.incount == 0 && node.outcount == 0 {
                node.alive = false;
            }
        }
    }

    ///
    /// Changes the costs of edges according to a given mapping.
    ///
    pub fn map_costs<M>(&mut self, mut mapping: M)
    where
        M: FnMut(&TopoEdge) -> f64,
    {
        for edges in &mut self.edges {
            for edge in edges {
                edge.cost = mapping(edge);
            }
        }
    }

    /// Creates a .dot output for visualizing the module graph.
    #[must_use]
    pub fn as_dot(&self) -> String {
        let mut output = String::from("digraph D {{\n");

        for def in &self.nodes {
            if def.incount > 0 || def.outcount > 0 {
                output.push_str(&format!("    \"{}\" [shape=box]\n", def.module.as_str()));
            }
        }

        output.push('\n');

        for (src, edges) in self.edges.iter().enumerate() {
            let from_node = self.nodes[src].module.as_str();
            for TopoEdge {
                dst_id: id,
                src,
                dst,
                cost,
            } in edges
            {
                let to_node = self.nodes[*id].module.as_str();
                let label = if *cost == 0.0 {
                    String::new()
                } else {
                    format!("label=\"{cost}\"")
                };

                output.push_str(&format!(
                    "    \"{}\" -> \"{}\" [ headlabel=\"{}\" {} taillabel=\"{}\" ]\n",
                    from_node,
                    to_node,
                    dst.name(),
                    label,
                    src.name(),
                ));
            }
        }

        output.push_str("\n}}");
        output
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
        let dot_output = self.as_dot();
        let mut file = File::create(format!("{path}.dot"))?;
        write!(file, "{dot_output}")?;

        let svg_output = Command::new("dot")
            .arg("-Tsvg")
            .arg(format!("{path}.dot"))
            .output()?;

        let mut file = File::create(format!("{path}.svg"))?;
        write!(file, "{}", String::from_utf8_lossy(&svg_output.stdout))?;

        Ok(())
    }
}
