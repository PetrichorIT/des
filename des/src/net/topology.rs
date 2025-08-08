//! Graph-based tooling for exploring simulation topology.
use fxhash::{FxBuildHasher, FxHashMap};

use super::{
    gate::{GateKind, GateRef},
    globals,
    module::ModuleRef,
    ObjectPath,
};
use std::{
    io::{Result, Write},
    process::Command,
};
use std::{ops::Deref, process::Stdio};

/// A graph-based representation of the simulations topology.
#[derive(Debug, Default, Clone)]
pub struct Topology<N, C> {
    nodes: Vec<Node<N>>,
    edges: Vec<Vec<EdgeRaw<C>>>,
}

type NodeID = usize;

/// A node in the topologcial graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Node<N> {
    data: N,
    module: ModuleRef,
}

/// An edge in the topological graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge<'a, N, C> {
    /// The endpoint where the current edge starts.
    pub from: EdgeEndpoint<'a, N>,
    /// The endpoint where the current edge ends.
    pub to: EdgeEndpoint<'a, N>,
    /// Some data attachment to the current edge.
    pub attachment: &'a C,
}

/// A endpoint of an edge, definining how and where a edge is
/// attached to a node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeEndpoint<'a, N> {
    node: &'a Node<N>,
    gate: &'a GateRef,
    id: NodeID,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EdgeRaw<C> {
    dst: NodeID,
    data: C,

    start: GateRef,
    end: GateRef,
}

// ==== impl Topology ====

impl<N, C> Topology<N, C> {
    /// Returns a slice of nodes, described by this topology object.
    #[must_use]
    pub fn nodes(&self) -> &[Node<N>] {
        &self.nodes
    }

    /// Indicates whether the current graph is bidirectional.
    ///
    /// Bidirectional in this case means:
    ///
    /// For each edge from `gate-a` to `gate-b` there exists an equivalent edge
    /// from `gate-b` to `gate-a`. The attachments on each edge may differ.
    #[must_use]
    pub fn bidirectional(&self) -> bool {
        for (src, bundle) in self.edges.iter().enumerate() {
            for edge in bundle {
                if !self.edges[edge.dst].iter().any(|edge| edge.dst == src) {
                    return false;
                }
            }
        }
        true
    }

    /// Indicates whether the current graph is fully connected.
    ///
    /// A fully connected graph has a path from each abitrary start
    /// node to each abitrary end node.
    #[must_use]
    pub fn connected(&self) -> bool {
        fn visit<N, C>(topo: &Topology<N, C>, i: usize, visited: &mut Vec<NodeID>) {
            if !visited.contains(&i) {
                visited.push(i);
                for edge in &topo.edges[i] {
                    visit(topo, edge.dst, visited);
                }
            }
        }

        for start in 0..self.nodes.len() {
            let mut visited = Vec::new();
            visit(self, start, &mut visited);
            if visited.len() != self.nodes.len() {
                return false;
            }
        }
        true
    }

    /// Filters out edges based on a given predicate `f`.
    ///
    /// This operation will only remove edges, nodes will be unchanged.
    pub fn filter_edges<F>(&mut self, mut f: F)
    where
        F: FnMut(Edge<'_, N, C>) -> bool,
    {
        for (src, src_bundle) in self.edges.iter_mut().enumerate() {
            src_bundle.retain(|raw| {
                let edge = Edge {
                    from: EdgeEndpoint {
                        node: &self.nodes[src],
                        gate: &raw.start,
                        id: src,
                    },
                    to: EdgeEndpoint {
                        node: &self.nodes[raw.dst],
                        gate: &raw.end,
                        id: raw.dst,
                    },
                    attachment: &raw.data,
                };

                f(edge)
            });
        }
    }

    /// Filters out nodes based on a given predicate `f`.
    ///
    /// This operation not only removes nodes, but also edges connected
    /// to the removed nodes.
    pub fn filter_nodes<F>(&mut self, f: F)
    where
        F: FnMut(&Node<N>) -> bool,
        N: std::fmt::Debug,
    {
        let n = self.nodes.len();

        let keep = self.nodes.iter().map(f).collect::<Vec<_>>();

        let mut node_id_mapping = (0..n).collect::<Vec<_>>();
        let mut running_index = 0;
        for index in &mut node_id_mapping {
            if keep[*index] {
                *index = running_index;
                running_index += 1;
            } else {
                *index = usize::MAX;
                self.nodes.remove(running_index);
                self.edges.remove(running_index);
            }
        }

        for bundle in &mut self.edges {
            bundle.retain_mut(|edge| {
                edge.dst = node_id_mapping[edge.dst];
                edge.dst != usize::MAX
            });
        }
    }

    /// Preforms a dijstrak from a starting position.
    ///
    /// # Panics
    ///
    /// This function will panic if no node with the given object path exists
    /// in the topology object.
    pub fn dijkstra(&self, src: impl Into<ObjectPath>) -> FxHashMap<ObjectPath, Edge<'_, N, C>>
    where
        N: Clone,
        C: Clone,
    {
        struct QueueElement<'a, N, C> {
            idx: NodeID,
            distance: usize,
            next: Option<Edge<'a, N, C>>,
        }

        let src = src.into();

        let mut visited = Vec::new();
        let mut queue = Vec::new();
        queue.push(QueueElement {
            idx: self
                .nodes
                .iter()
                .position(|node| node.module.path == src)
                .expect("unknown node"),
            distance: 0,
            next: None,
        });

        let mut mapping = FxHashMap::with_hasher(FxBuildHasher::default());
        while let Some(cur) = queue.pop() {
            if visited.contains(&cur.idx) {
                continue;
            }

            visited.push(cur.idx);
            if let Some(ref hop) = cur.next {
                mapping.insert(self.nodes[cur.idx].module.path(), hop.clone());
            }

            for edge in self.edges_by_id(cur.idx) {
                if !visited.contains(&edge.to.id) {
                    queue.push(QueueElement {
                        idx: edge.to.id,
                        distance: cur.distance + 1,
                        next: Some(cur.next.clone().unwrap_or(edge)),
                    });
                }
            }
        }

        mapping
    }

    /// Adds some attached data to the nodes of the topology object.
    pub fn with_node_attachments<F, A>(&self, mut f: F) -> Topology<A, C>
    where
        F: FnMut(&Node<N>) -> A,
        C: Clone,
    {
        Topology {
            nodes: self
                .nodes
                .iter()
                .map(|node| Node {
                    data: f(node),
                    module: node.module.clone(),
                })
                .collect(),
            edges: self.edges.clone(),
        }
    }

    /// Adds some attached data to the edges of the topology object.
    #[must_use]
    pub fn with_edge_attachments<F, A>(&self, mut f: F) -> Topology<N, A>
    where
        F: FnMut(Edge<'_, N, C>) -> A,
        N: Clone,
    {
        Topology {
            nodes: self.nodes.clone(),
            edges: self
                .edges
                .iter()
                .enumerate()
                .map(|(src_id, bundle)| {
                    bundle
                        .iter()
                        .map(|raw| {
                            let edge = Edge {
                                from: EdgeEndpoint {
                                    node: &self.nodes[src_id],
                                    gate: &raw.start,
                                    id: src_id,
                                },
                                to: EdgeEndpoint {
                                    node: &self.nodes[raw.dst],
                                    gate: &raw.end,
                                    id: raw.dst,
                                },
                                attachment: &raw.data,
                            };
                            EdgeRaw {
                                dst: raw.dst,
                                data: f(edge),
                                start: raw.start.clone(),
                                end: raw.end.clone(),
                            }
                        })
                        .collect()
                })
                .collect(),
        }
    }
}

impl Topology<(), ()> {
    /// Retrieves the current globsal topology
    ///
    /// # Panics
    ///
    /// This function panics if not called from a simulation context.
    #[must_use]
    pub fn current() -> Self {
        globals().topology()
    }

    /// Generates a topology based on all reachable destinations from a root node.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn spanned(root: ModuleRef) -> Self {
        let mut modules = vec![root];
        let mut this = Self::default();

        while let Some(module) = modules.pop() {
            let gates = module.gates();

            this.nodes.push(Node { data: (), module });
            this.edges.push(Vec::new());

            let src_idx = this.nodes.len() - 1;
            for gate in gates {
                if gate.kind() == GateKind::Endpoint {
                    let iter = gate
                        .path_iter()
                        .expect("path_iter should exist on gates of kind: endpoint");

                    // The iterator is finite, since at least one gate (the start point) has degree 1
                    let mut end = gate.clone();
                    for con in iter {
                        end = con.endpoint;
                    }

                    let end_id = end.owner().id();
                    let end_idx = this
                        .nodes
                        .iter()
                        .position(|node| node.module.id() == end_id)
                        .unwrap_or_else(|| {
                            // Node is not yet in the spanned set
                            // but maybe allready in queue
                            if let Some(offset) =
                                modules.iter().position(|module| module.id() == end_id)
                            {
                                src_idx + 1 + offset
                            } else {
                                modules.push(end.owner());
                                src_idx + modules.len()
                            }
                        });

                    let raw = EdgeRaw {
                        dst: end_idx,
                        data: (),
                        start: gate,
                        end,
                    };

                    this.edges[src_idx].push(raw);
                }
            }
        }
        this
    }

    /// Generates a topology object over a given set of modules.
    ///
    /// This function will only created edges between the modules in the list.
    /// Should this function find an edge to a module, not referenced  in this list
    /// the edge will not be recorded.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn from_modules(modules: &[ModuleRef]) -> Self {
        let mut this = Self::default();

        for module in modules.iter().cloned() {
            this.nodes.push(Node { data: (), module });
            this.edges.push(Vec::new());
        }

        for (src_id, module) in modules.iter().enumerate() {
            let gates = module.gates();
            for gate in gates {
                if gate.kind() == GateKind::Endpoint {
                    let iter = gate
                        .path_iter()
                        .expect("path_iter should exist on gates of kind: endpoint");

                    let mut end = gate.clone();
                    for con in iter.take(16) {
                        end = con.endpoint;
                    }

                    let end_id = end.owner().id();
                    let Some(dst) = this
                        .nodes
                        .iter()
                        .position(|node| node.module.id() == end_id)
                    else {
                        // no spanning tree, ignore external links
                        continue;
                    };

                    let raw = EdgeRaw {
                        dst,
                        data: (),
                        start: gate,
                        end,
                    };

                    this.edges[src_id].push(raw);
                }
            }
        }

        this
    }
}

#[cfg(test)]
impl Topology<NodeID, ()> {
    fn raw(edges: &[&[usize]]) -> Self {
        let n = edges.len();
        let module = super::module::ModuleContext::standalone("raw-topology-holder".into());
        let start = module.create_gate("start");
        let end = module.create_gate("end");

        Self {
            nodes: (0..n)
                .map(|data| Node {
                    data,
                    module: module.clone(),
                })
                .collect(),
            edges: edges
                .into_iter()
                .map(|bundle| {
                    bundle
                        .into_iter()
                        .map(|&dst| EdgeRaw {
                            dst,
                            data: (),
                            start: start.clone(),
                            end: end.clone(),
                        })
                        .collect()
                })
                .collect(),
        }
    }
}

impl<N, C> Topology<N, C> {
    /// Exports the current toplogy object as an DOT string.
    #[must_use]
    pub fn as_dot(&self) -> String {
        let nodes = self
            .nodes
            .iter()
            .map(|node| format!("\t\"{}\" [shape=box]\n", node.identifier()))
            .fold(String::new(), |acc, s| acc + &s);

        let edges = self
            .edges()
            .map(|edge| {
                format!(
                    "\t\"{}\" -> \"{}\" [ headlabel = \"{}\" taillable = \"{}\" ]\n",
                    edge.from.identifier(),
                    edge.to.identifier(),
                    edge.from.gate.str(),
                    edge.to.gate.str()
                )
            })
            .fold(String::new(), |acc, s| acc + &s);

        format!("digraph D {{ \n{nodes} {edges}}}")
    }

    /// Exports the current toplogy object as a SVG.
    ///
    /// # Errors
    ///
    /// This function will fail, if the execution of the `dot` subcommand
    /// fails.
    ///
    /// # Panics
    ///
    /// This function will panic if the subprocess stdin could not
    /// be opened.
    pub fn as_svg(&self) -> Result<String> {
        let mut child = Command::new("dot")
            .arg("-Tsvg")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = child.stdin.take().expect("failed to open stdin");
        stdin.write_all(self.as_dot().as_bytes())?;
        drop(stdin);

        let output = child.wait_with_output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// ==== impl Attachments ====

/// An attachment that represents a node local view on available
/// connections.
///
/// > Note that attachments do NOT automatically updated when the topology object is later changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeConnectivityAttachment {
    /// The number of outoing edges.
    pub degree: usize,
}

impl<N, C> Topology<N, C> {
    /// Adds [`NodeConnectivityAttachment`] to all nodes.
    #[must_use]
    pub fn with_node_connectivity_attachment(&self) -> Topology<NodeConnectivityAttachment, C>
    where
        C: Clone,
    {
        self.with_node_attachments(|node| {
            let degree = self
                .edges_for_node(node)
                .filter(|edge| edge.from.gate().owner().id() != edge.to.gate().owner().id())
                .count();

            NodeConnectivityAttachment { degree }
        })
    }
}

/// An attachment that analyses the cost of edges, as well as their ability
/// to transmit messages.
///
/// > Note that attachments do NOT automatically updated when the topology object is later changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeCostAttachment {
    /// A cost definining how expensive a certain link would be.
    pub cost: f64,
    /// A flag whether a link is currently able to transmit messages.
    ///
    /// This may be false if, e.g. a transit node on the link is shut down.
    pub alive: bool,
}

impl<N, C> Topology<N, C> {
    /// Adds [`EdgeCostAttachment`]'s to all edges.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn with_edge_cost_attachment(&self) -> Topology<N, EdgeCostAttachment>
    where
        N: Clone,
    {
        self.with_edge_attachments(|edge| {
            let mut cost = 0.0;
            let mut alive = edge.from.gate.owner().is_active();

            let iter = edge
                .from
                .gate()
                .path_iter()
                .expect("all repesented edges SHOULD exist only on endpoint gates");

            for con in iter.take(16) {
                if con.channel().is_some() {
                    cost += 1.0;
                }
                alive |= con.endpoint.owner().is_active();
            }

            EdgeCostAttachment { cost, alive }
        })
    }
}

// ==== impl Node ====

impl<N> Node<N> {
    /// A module handle to the current nodes module.
    pub fn module(&self) -> ModuleRef {
        self.module.clone()
    }

    fn identifier(&self) -> &str {
        self.module.path.as_logger_scope()
    }
}

impl<N> Deref for Node<N> {
    type Target = N;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

// ==== impl Edges ===

/// An iterator over a set of edges in a [`Topology`] object.
#[derive(Debug)]
#[must_use]
pub struct EdgesIter<'a, N, C> {
    nodes: &'a [Node<N>],
    inner_src: usize, // allways index + 1 (since iter starts as 0)
    inner: &'a [EdgeRaw<C>],
    buffer: &'a [Vec<EdgeRaw<C>>],
}

impl<N, C> EdgesIter<'_, N, C> {
    fn empty() -> Self {
        Self {
            nodes: &[],
            inner_src: 0,
            inner: &[],
            buffer: &[],
        }
    }
}

impl<'a, N, C> Iterator for EdgesIter<'a, N, C> {
    type Item = Edge<'a, N, C>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((raw, remaining)) = self.inner.split_first() {
            self.inner = remaining;
            Some(Edge {
                from: EdgeEndpoint {
                    node: &self.nodes[self.inner_src - 1],
                    gate: &raw.start,
                    id: self.inner_src - 1,
                },
                to: EdgeEndpoint {
                    node: &self.nodes[raw.dst],
                    gate: &raw.end,
                    id: raw.dst,
                },
                attachment: &raw.data,
            })
        } else {
            let (next_slice, remaining) = self.buffer.split_first()?;
            self.buffer = remaining;

            self.inner_src += 1;
            self.inner = next_slice;
            self.next()
        }
    }
}

impl<N, C> Topology<N, C> {
    /// Returns an edge iterator over all edges in the topology.
    pub fn edges(&self) -> EdgesIter<'_, N, C> {
        EdgesIter {
            nodes: &self.nodes,
            inner_src: 0,
            inner: &[],
            buffer: &self.edges,
        }
    }

    /// Retuns an edge iterator for a specific node.
    pub fn edges_for_node(&self, node: &Node<N>) -> EdgesIter<'_, N, C> {
        self.edges_for(node.module.path.clone())
    }

    /// Returns an edge iterator of all edges starting at the specified node.
    pub fn edges_for(&self, src: impl Into<ObjectPath>) -> EdgesIter<'_, N, C> {
        let src = src.into();
        let Some(src) = self.nodes.iter().position(|node| node.module.path == src) else {
            return EdgesIter::empty();
        };
        self.edges_by_id(src)
    }

    fn edges_by_id(&self, src: NodeID) -> EdgesIter<'_, N, C> {
        EdgesIter {
            nodes: &self.nodes,
            inner_src: src + 1,
            inner: &self.edges[src],
            buffer: &[],
        }
    }
}

// ==== impl Edge<'_> ====

impl<N> EdgeEndpoint<'_, N> {
    /// Returns the gate the endpoint is attached to.
    #[must_use]
    pub fn gate(&self) -> GateRef {
        self.gate.clone()
    }
}

impl<N> Deref for EdgeEndpoint<'_, N> {
    type Target = Node<N>;
    fn deref(&self) -> &Self::Target {
        self.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_filtering() {
        let mut topo = Topology::raw(&[&[1, 2], &[0], &[0, 3], &[2, 4], &[3]]);
        assert!(topo.connected());
        assert_eq!(topo.edges().count(), 8);

        topo.filter_nodes(|node| ![2].contains(&node.data));

        assert!(!topo.connected());
        assert_eq!(topo.edges().count(), 4)
    }

    #[test]
    fn edge_filtering() {
        let mut topo = Topology::raw(&[&[1, 2], &[0], &[0, 3], &[2, 4], &[3]]);
        assert!(topo.bidirectional());
        assert_eq!(topo.edges().count(), 8);

        topo.filter_edges(|e| e.from.data < e.to.data);
        assert!(!topo.bidirectional());
        assert_eq!(topo.edges().count(), 4);
    }

    #[test]
    fn edge_iter() {
        let topo = Topology::raw(&[&[1, 2], &[0], &[0, 3], &[2, 4], &[3]]);
        let mapper = |edge: Edge<'_, usize, ()>| (edge.from.data, edge.to.data);
        assert_eq!(
            topo.edges().map(mapper).collect::<Vec<_>>(),
            [
                (0, 1),
                (0, 2),
                (1, 0),
                (2, 0),
                (2, 3),
                (3, 2),
                (3, 4),
                (4, 3)
            ]
        );

        assert_eq!(
            topo.edges_by_id(2).map(mapper).collect::<Vec<_>>(),
            [(2, 0), (2, 3),]
        );
    }
}
