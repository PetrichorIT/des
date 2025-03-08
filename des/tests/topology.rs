use std::{fs, io::Write};

use des::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn main() {
    let app = Sim::ndl("tests/ndl/top.yml", Registry::new().with_default_fallback())
        .map_err(|e| println!("{e}"))
        .unwrap();
    let rt = Builder::new().build(app);
    let app = rt.run().unwrap().0;
    let mut topo = app
        .globals()
        .topology
        .lock()
        .unwrap()
        .clone()
        .with_edge_cost_attachment()
        .with_node_connectivity_attachment();

    assert!(topo.bidirectional());
    assert!(!topo.connected());

    let dj = topo.dijkstra("node[1]");
    assert_eq!(dj.get(&"node[1]".into()), None);

    topo.filter_nodes(|n| n.module().name() != "node[2]");

    assert_eq!(topo.edges().count(), 10);

    // 4 nodes, router, debugger, main + distant
    assert_eq!(topo.nodes().len(), 8);

    topo.filter_nodes(|node| node.degree > 0);
    assert_eq!(topo.nodes().len(), 6);
    assert_eq!(topo.edges().count(), 10);

    assert!(topo
        .edges_for("router")
        .any(|edge| edge.to.gate().owner().path().as_str() == "debugger"));

    if let Ok(output) = topo.as_svg() {
        fs::File::create("tests/topology.svg")
            .unwrap()
            .write_all(output.as_bytes())
            .unwrap();
    }
}

struct Fallback;
impl Module for Fallback {}

#[test]
#[serial]
fn spanned_topology() {
    let mut sim = Sim::new(());

    sim.node("alice", Fallback);
    sim.node("alice.eve", Fallback);
    sim.node("alice.eve.travis", Fallback);
    sim.node("alice.sophie", Fallback);
    sim.node("bob", Fallback);

    sim.gate("alice", "to-eve")
        .connect(sim.gate("alice.eve", "to-alice"), None);
    sim.gate("alice", "to-sophie")
        .connect(sim.gate("alice.sophie", "to-alice"), None);
    sim.gate("alice.eve", "to-travis")
        .connect(sim.gate("alice.eve.travis", "to-eve"), None);
    sim.gate("alice.eve", "to-sophie")
        .connect(sim.gate("alice.sophie", "to-eve"), None);

    let root = sim.get(&"alice".into()).unwrap();

    let topology = Topology::spanned(root);
    assert_eq!(topology.nodes().len(), 4);
    assert_eq!(topology.edges().count(), 2 * 4);
    assert!(!topology.nodes().iter().any(|n| n.module().name() == "bob"));
}
