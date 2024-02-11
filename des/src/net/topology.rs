use fxhash::{FxBuildHasher, FxHashMap};

use crate::{net::module::ModuleRef, prelude::GateRef};

use super::{globals, ObjectPath};

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
    /// The number of connections. (not nessecarily the number of gates)
    pub degree: usize,
    /// An indicate whether the module is at all connected to the rest of of the network.
    pub alive: bool,
}

///
/// A connection in the module connection graph.
///
#[derive(Debug, Clone)]

pub struct TopoEdge {
    ///
    pub src: (GateRef, usize), //
    ///
    pub dst: (GateRef, usize),
    ///
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

    /// Gets the current topology.
    ///
    /// # Panics
    /// 
    /// Panics when called from outside a module context,
    /// or when globals dont exist.
    #[must_use]
    pub fn current() -> Topology {
        globals().topology.lock().unwrap().clone()
    }

    /// All nodes if the current topology.
    #[must_use]
    pub fn nodes(&self) -> &[TopoNode] {
        &self.nodes
    }

    /// An iterator over all edges in the entries network, annotated with the
    /// node-id of the starting node.
    #[must_use]
    pub fn edges(&self) -> usize {
        self.edges
            .iter()
            .enumerate()
            .flat_map(|(i, edges)| edges.iter().map(move |e| (i, e)))
            .count()
            / 2
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
    #[allow(clippy::missing_panics_doc)]
    pub fn build(&mut self, modules: &[ModuleRef]) {
        for module in modules {
            self.nodes.push(TopoNode {
                module: module.clone(),
                degree: 0,
                alive: false,
            });
            self.edges.push(Vec::new());
        }

        for module in modules {
            let gates = module.ctx.gates();
            'outer: for gate in gates {
                let mut cost = 0.0;
                let mut dst = gate.clone();

                for con in gate.path_iter().take(16) {
                    if con.channel().is_some() {
                        cost += 1.0;
                    }

                    let transit_id = con.endpoint.owner().id();
                    let Some(transit) = self
                        .nodes
                        .iter_mut()
                        .find(|k| k.module.ctx.id() == transit_id)
                    else {
                        break 'outer;
                    };
                    transit.alive |= true;

                    dst = con.endpoint.clone();
                }

                let src_id = gate.owner().id();
                let dst_id = dst.owner().id();
                let (src_idx, src_node) = self
                    .nodes
                    .iter_mut()
                    .enumerate()
                    .find(|(_, m)| m.module.ctx.id() == src_id)
                    .unwrap();
                src_node.degree += 1;
                src_node.alive |= true;

                let (dst_idx, dst_node) = self
                    .nodes
                    .iter_mut()
                    .enumerate()
                    .find(|(_, m)| m.module.ctx.id() == dst_id)
                    .unwrap();
                dst_node.degree += 1;
                dst_node.alive |= true;

                let edge = TopoEdge {
                    src: (gate, src_idx),
                    dst: (dst, dst_idx),
                    cost,
                };

                self.edges[src_idx].push(edge);
            }
        }

        // Divide bc each connection was counted twice
        self.nodes.iter_mut().for_each(|node| node.degree /= 2);
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
                // Fix degrees

                for edge in self.edges[i].drain(..) {
                    self.nodes[edge.src.1].degree = self.nodes[edge.src.1].degree.saturating_sub(1);
                    self.nodes[edge.dst.1].degree = self.nodes[edge.dst.1].degree.saturating_sub(1);
                }

                for j in 0..n {
                    if j == i {
                        continue;
                    }
                    self.edges[j].retain(|edge| edge.dst.1 != i);
                }
                debug_assert_eq!(self.nodes[i].degree, 0);
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
                edge.dst.1 = mapping[edge.dst.1];
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
                    self.nodes[i].degree -= 1;
                    self.nodes[self.edges[i][j].dst.1].degree -= 1;
                }
            }
        }

        for node in &mut self.nodes {
            if node.degree == 0 {
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

    /// Generates a disjktra tree
    /// 
    /// # Panics 
    /// 
    /// Panics when the specified nodes does not exist.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn dijkstra(&self, node: ObjectPath) -> FxHashMap<ObjectPath, GateRef> {
        struct QE {
            node: usize,
            distance: usize,
            next_hop: Option<GateRef>,
        }

        let mut visited = Vec::new();
        let mut queue = Vec::new();
        queue.push(QE {
            node: self
                .nodes
                .iter()
                .enumerate()
                .find(|(_, n)| n.module.path() == node)
                .unwrap()
                .0,
            distance: 0,
            next_hop: None,
        });

        let mut mapping = FxHashMap::with_hasher(FxBuildHasher::default());
        while let Some(cur) = queue.pop() {
            if visited.contains(&cur.node) {
                continue;
            }

            // travel along the edges
            visited.push(cur.node);
            if let Some(ref nh) = cur.next_hop {
                mapping.insert(self.nodes[cur.node].module.path(), nh.clone());
            }

            for edge in self.edges_for(cur.node) {
                if !visited.contains(&edge.dst.1) {
                    queue.push(QE {
                        node: edge.dst.1,
                        distance: cur.distance + 1,
                        next_hop: Some(cur.next_hop.clone().unwrap_or(edge.src.0.clone())),
                    });
                }
            }

            // rev
            queue.sort_by(|l, r| r.distance.cmp(&l.distance));
        }

        mapping
    }

    /// Creates a .dot output for visualizing the module graph.
    #[must_use]
    pub fn as_dot(&self) -> String {
        let mut output = String::from("digraph D {{\n");

        for def in &self.nodes {
            if def.degree > 0 {
                output.push_str(&format!("    \"{}\" [shape=box]\n", def.module.as_str()));
            }
        }

        output.push('\n');

        for (src, edges) in self.edges.iter().enumerate() {
            let from_node = self.nodes[src].module.as_str();
            for TopoEdge { src, dst, cost } in edges {
                let to_node = self.nodes[src.1].module.as_str();
                let label = if *cost == 0.0 {
                    String::new()
                } else {
                    format!("label=\"{cost}\"")
                };

                output.push_str(&format!(
                    "    \"{}\" - \"{}\" [ headlabel=\"{}\" {} taillabel=\"{}\" ]\n",
                    from_node,
                    to_node,
                    dst.0.name(),
                    label,
                    src.0.name(),
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
